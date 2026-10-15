//! The gh commands the board runs, answered for a repo on Gitea through tea's API calls, in gh's own output: `pr
//! create` (a draft is a `WIP:` title, as Gitea has it), `pr view`, `pr list --head`, `pr ready`, `pr comment`, `pr
//! merge` with the merge method and the reviewed head, `api user` and `repo view`. Any other is refused by name, so
//! nothing GitHub-only is tried on Gitea. Used by flow/git.rs (`gh`).
use super::gitea_view::{get, is_wip, short, view, without_wip};
use super::tea::{api, said, TeaOut};
use super::Repo;
use crate::json::*;
use std::path::Path;

fn ok(out: &str) -> TeaOut { TeaOut { status: 0, stdout: if out.is_empty() { String::new() } else { format!("{out}\n") }, stderr: String::new() } }
fn fail(m: &str) -> TeaOut { TeaOut { status: 1, stdout: String::new(), stderr: format!("{m}\n") } }
fn flag<'a>(a: &'a [String], name: &str) -> Option<&'a str> { a.iter().position(|s| s == name).and_then(|i| a.get(i + 1)).map(|s| s.as_str()) }
fn has(a: &[String], name: &str) -> bool { a.iter().any(|s| s == name) }
/// The pull request's number: the first word after `pr <verb>` that is not a flag.
fn number(a: &[String]) -> String { a.get(2).filter(|s| !s.starts_with('-')).map(|s| super::pr_number(s)).unwrap_or_default() }

pub fn as_gh(a: &[String], repo: &Repo, cwd: &Path, timeout_ms: u64) -> TeaOut {
    let (x, y) = (a.first().map(|s| s.as_str()).unwrap_or(""), a.get(1).map(|s| s.as_str()).unwrap_or(""));
    let r = match (x, y) {
        ("pr", "create") => create(a, repo, cwd, timeout_ms),
        ("pr", "view") => view(repo, &number(a), flag(a, "--json").unwrap_or("number,url,state"), cwd).map(|v| v.to_string()),
        ("pr", "list") if !has(a, "--jq") => list(a, repo, cwd),
        ("pr", "ready") => ready(repo, &number(a), cwd),
        ("pr", "comment") => post(repo, &format!("{}/issues/{}/comments", repo.path(), number(a)), &json!({ "body": flag(a, "--body").unwrap_or("") }), cwd, timeout_ms).map(|_| String::new()),
        // Gitea's own auto-merge is never set by the board: there is none to switch off.
        ("pr", "merge") if has(a, "--disable-auto") => Ok(String::new()),
        ("pr", "merge") => merge(a, repo, cwd, timeout_ms),
        ("api", "user") => get(repo, "user", cwd).map(|u| u.t("login")),
        ("repo", "view") => get(repo, &repo.path(), cwd).map(|r| if has(a, "--jq") { r.b("archived").to_string() } else { json!({ "isArchived": r.b("archived") }).to_string() }),
        _ => Err(format!("gh {} has no Gitea counterpart here (this repo is on Gitea, {}).", a.join(" "), repo.host)),
    };
    match r { Ok(out) => ok(&out), Err(e) => fail(&e) }
}
fn post(repo: &Repo, path: &str, body: &Value, cwd: &Path, timeout_ms: u64) -> Result<Value, String> { send(repo, "POST", path, body, cwd, timeout_ms) }
fn send(repo: &Repo, method: &str, path: &str, body: &Value, cwd: &Path, timeout_ms: u64) -> Result<Value, String> {
    let a = api(&repo.login, method, path, Some(body), cwd, timeout_ms)?;
    if a.ok() { Ok(a.body) } else { Err(format!("{} ({method} {path})", said(&a))) }
}
/// `gh pr create [--draft] --head <branch> --title <t> --body <b> [--base <b>]`: its link, as gh prints it.
fn create(a: &[String], repo: &Repo, cwd: &Path, timeout_ms: u64) -> Result<String, String> {
    let base = match flag(a, "--base") { Some(b) => b.to_string(), None => get(repo, &repo.path(), cwd)?.t("default_branch") };
    let title = flag(a, "--title").unwrap_or("").to_string();
    let title = if has(a, "--draft") && !is_wip(&title) { format!("WIP: {title}") } else { title };
    let head = flag(a, "--head").map(String::from).unwrap_or_else(|| crate::store::repos::branch_of(cwd));
    let p = post(repo, &format!("{}/pulls", repo.path()), &json!({ "head": head, "base": base, "title": title, "body": flag(a, "--body").unwrap_or("") }), cwd, timeout_ms)?;
    Ok(p.t("html_url"))
}
/// `gh pr list --head <branch> --state all --limit <n> --json …`: the newest first.
fn list(a: &[String], repo: &Repo, cwd: &Path) -> Result<String, String> {
    let state = match flag(a, "--state").unwrap_or("open") { "all" => "all", "closed" | "merged" => "closed", _ => "open" };
    let limit: usize = flag(a, "--limit").and_then(|n| n.parse().ok()).unwrap_or(30);
    let all = get(repo, &format!("{}/pulls?state={state}&sort=newest&limit=50", repo.path()), cwd)?;
    let head = flag(a, "--head");
    let out: Vec<Value> = all.as_array().cloned().unwrap_or_default().iter().filter(|p| head.map(|h| p.at("head").s("ref") == h).unwrap_or(true)).map(short).take(limit).collect();
    Ok(Value::Array(out).to_string())
}
/// `gh pr ready <n>`: the draft's `WIP:` comes off its title.
fn ready(repo: &Repo, n: &str, cwd: &Path) -> Result<String, String> {
    let path = format!("{}/pulls/{n}", repo.path());
    let p = get(repo, &path, cwd)?;
    if is_wip(p.s("title")) { send(repo, "PATCH", &path, &json!({ "title": without_wip(p.s("title")) }), cwd, 60000)?; }
    Ok(String::new())
}
/// `gh pr merge <n> --squash|--merge|--rebase [--match-head-commit <sha>]`: Gitea refuses it when the head moved.
fn merge(a: &[String], repo: &Repo, cwd: &Path, timeout_ms: u64) -> Result<String, String> {
    let how = ["squash", "rebase", "merge"].into_iter().find(|h| has(a, &format!("--{h}"))).unwrap_or("merge");
    let mut body = json!({ "Do": how });
    if let Some(h) = flag(a, "--match-head-commit") { body["head_commit_id"] = json!(h); }
    send(repo, "POST", &format!("{}/pulls/{}/merge", repo.path(), number(a)), &body, cwd, timeout_ms).map(|_| String::new())
}
