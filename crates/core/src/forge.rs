//! Which forge holds a repo's pull requests, from its remote: GitHub (asked through gh) or Gitea (asked through tea).
//! `merge.forge` says it outright (`github`, `gitea`); `auto`, the default, reads the host of the remote `origin`:
//! github.com is GitHub, a host tea has a login for (`tea login list`) is Gitea, any other host GitHub, as before.
//! flow/git.rs (`gh`) asks here before every gh call, and a Gitea repo's are answered through tea (forge/gitea.rs).
//! More: docs/gitea.md.
pub mod gitea;
pub mod gitea_checks;
pub mod gitea_view;
pub mod tea;

use crate::json::*;
use crate::store::repos::{git_out, known_repos, primary_root};
use crate::store::settings::lookup::{lookup, Scope};
use crate::store::settings::places::yours;
use std::path::Path;

/// A repo on a Gitea server: the tea login that reaches it, and its owner and name there.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Repo { pub login: String, pub host: String, pub owner: String, pub name: String }
impl Repo {
    /// Its path in Gitea's API, without the leading `/api/v1/`.
    pub fn path(&self) -> String { format!("repos/{}/{}", self.owner, self.name) }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Forge {
    GitHub,
    Gitea(Repo),
    /// Gitea was asked for (`merge.forge` is `gitea`) and cannot be reached: why, said where gh's error would be.
    Broken(String),
}
/// Where a remote points: its host, and the last two parts of its path (owner and repo, `.git` taken off).
#[derive(Clone, Debug, PartialEq)]
pub struct Remote { pub host: String, pub owner: String, pub name: String }

/// `ssh://git@host:2222/owner/repo.git`, `git@host:owner/repo.git` and `https://host/owner/repo` alike. None for a
/// folder on this machine (a remote with no host).
pub fn parse_remote(url: &str) -> Option<Remote> {
    let url = url.trim();
    let (host, path) = if let Some((_, rest)) = url.split_once("://") {
        let (auth, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host = auth.rsplit('@').next().unwrap_or("").split(':').next().unwrap_or("");
        (host.to_string(), path.to_string())
    } else {
        let (before, path) = url.split_once(':')?;
        if before.contains('/') || before.is_empty() { return None; }
        (before.rsplit('@').next().unwrap_or("").to_string(), path.to_string())
    };
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if host.is_empty() || parts.len() < 2 { return None; }
    let name = parts[parts.len() - 1].trim_end_matches(".git");
    Some(Remote { host: host.to_lowercase(), owner: parts[parts.len() - 2].to_string(), name: name.to_string() })
}
pub fn is_github(host: &str) -> bool { host == "github.com" || host.ends_with(".github.com") }

/// What `merge.forge` says for the repo of this folder: your entry for the repo, then yours, then `auto`.
pub fn said(root: &Path) -> String {
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let name = known_repos().iter().find(|(_, v)| { let p = std::path::PathBuf::from(text(v)); p == root || real(&p) == real(root) }).map(|(k, _)| k.clone()).unwrap_or_default();
    let scope = if name.is_empty() { Scope::you() } else { Scope::repo(&name) };
    lookup("merge.forge", &scope, &yours()).map(|f| text(&f.value)).unwrap_or_else(|_| "auto".into())
}
/// The remote `origin` of a folder's repo, as configured (before any `insteadOf`).
pub fn origin(root: &Path) -> Option<Remote> { parse_remote(&git_out(root, &["config", "--get", "remote.origin.url"]).ok()?) }

/// The forge of the repo a folder (a checkout, a worktree) belongs to.
pub fn of(cwd: &Path) -> Forge {
    let root = primary_root(cwd);
    let said = said(&root);
    if said == "github" { return Forge::GitHub; }
    let remote = origin(&root);
    let gitea = said == "gitea";
    let Some(r) = remote else {
        return if gitea { Forge::Broken("merge.forge is gitea, but this repo has no remote origin on a Gitea server (git remote get-url origin).".into()) } else { Forge::GitHub };
    };
    if !gitea && is_github(&r.host) { return Forge::GitHub; }
    match tea::logins() {
        Err(e) => if gitea { Forge::Broken(e) } else { Forge::GitHub },
        Ok(list) => match list.iter().find(|l| l.serves(&r.host)) {
            Some(l) => Forge::Gitea(Repo { login: l.name.clone(), host: r.host.clone(), owner: r.owner.clone(), name: r.name.clone() }),
            None if gitea => Forge::Broken(tea::no_login(&r.host)),
            None => Forge::GitHub,
        },
    }
}
/// The command line and the forge's name the board says for a folder's repo: tea and Gitea where it is on Gitea, or
/// is to be and cannot be reached.
pub fn cli_of(cwd: &Path) -> (&'static str, &'static str) { if of(cwd) == Forge::GitHub { ("gh", "GitHub") } else { ("tea", "Gitea") } }
/// How to change a pull request's base, said in a hint: gh's command on GitHub; tea has none, so Gitea's page.
pub fn base_edit_how(cwd: &Path, number: &str, base: &str) -> String {
    match cli_of(cwd) { ("gh", _) => format!("gh pr edit {number} --base {base}"), (_, name) => format!("on its {name} page: Edit, target branch {base}") }
}
/// The Gitea repo of a folder, where it is one.
pub fn gitea_repo(cwd: &Path) -> Option<Repo> { if let Forge::Gitea(r) = of(cwd) { Some(r) } else { None } }
/// Said after gh's own error, where it found no GitHub host: the remote is not on GitHub, and how to reach it.
pub fn not_github_hint(cwd: &Path) -> Option<String> {
    let r = origin(&primary_root(cwd)).filter(|r| !is_github(&r.host))?;
    let why = match tea::logins() { Err(e) => e, Ok(_) => tea::no_login(&r.host) };
    Some(format!("This repo's remote is on {}, not GitHub. If it is a Gitea server: {why} Or set merge.forge to gitea for this repo.", r.host))
}

/// A pull request's number, from its link on GitHub (`/pull/7`) or on Gitea (`/pulls/7`), or as it is.
pub fn pr_number(pr: &str) -> String {
    for sep in ["/pull/", "/pulls/"] {
        let n: String = pr.split(sep).nth(1).map(|r| r.chars().take_while(|c| c.is_ascii_digit()).collect()).unwrap_or_default();
        if !n.is_empty() { return n; }
    }
    pr.to_string()
}
/// How the board names a forge and its command line in what it says. A pull request on Gitea has `/pulls/` in its link.
pub struct Words { pub name: &'static str, pub cli: &'static str, pub login: &'static str }
pub fn words_of_pr(pr: &str) -> Words {
    if pr.contains("/pulls/") { Words { name: "Gitea", cli: "tea", login: "tea login list" } } else { Words { name: "GitHub", cli: "gh", login: "gh auth status" } }
}
