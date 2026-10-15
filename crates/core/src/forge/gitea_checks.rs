//! What a Gitea pull request's checks are to be before it merges (`merge.checks`): the commit statuses its branch
//! protection requires, and whether Gitea Actions are coming at all. A repo with workflows (`.gitea/workflows`, or
//! `.github/workflows`, which Gitea reads too) and Actions on gets a short while for its first status to appear; one
//! with none never waits for a check that will not come. Used by land/checks.rs and the pull requests' watch.
use super::gitea_view::get;
use super::Repo;
use crate::json::*;
use std::path::Path;

/// How long a pull request with no status yet waits for Gitea Actions to report one, where workflows could run.
pub const GRACE_MS: i64 = 3 * 60000;
/// The board has waited longer than that for this commit's first status (`checksWait`, land/checks.rs).
pub fn grace_over(t: &Obj, head: &str) -> bool { let w = t.at("checksWait"); w.s("head") == head && w.b("since") && crate::clock::now() - w.i("since") >= GRACE_MS }
/// The worktree has workflows Gitea Actions could run.
pub fn has_workflows(tree: &Path) -> bool {
    [".gitea/workflows", ".github/workflows"].iter().any(|d| std::fs::read_dir(tree.join(d)).map(|r| r.flatten().any(|e| e.path().extension().map(|x| x == "yml" || x == "yaml").unwrap_or(false))).unwrap_or(false))
}
/// Gitea Actions are on for the repo. Where Gitea cannot say, they count as on.
pub fn actions_on(repo: &Repo, cwd: &Path) -> bool { get(repo, &repo.path(), cwd).map(|r| r.at("has_actions").as_bool().unwrap_or(true)).unwrap_or(true) }
/// Does a branch protection rule's name (a glob, as Gitea reads it) take in this branch?
fn covers(rule: &str, branch: &str) -> bool {
    if !rule.contains('*') { return rule == branch; }
    let re = format!("^{}$", regex::escape(rule).replace(r"\*\*", ".*").replace(r"\*", "[^/]*"));
    regex::Regex::new(&re).map(|r| r.is_match(branch)).unwrap_or(false)
}
/// The status checks the branch it merges into requires, by name (a pattern with `*` names none to wait for).
pub fn required(repo: &Repo, base: &str, cwd: &Path) -> Vec<String> {
    if base.is_empty() { return vec![]; }
    let Ok(rules) = get(repo, &format!("{}/branch_protections", repo.path()), cwd) else { return vec![] };
    let mut out: Vec<String> = vec![];
    for r in rules.as_array().cloned().unwrap_or_default().iter().filter(|r| r.b("enable_status_check")) {
        let name = if r.b("rule_name") { r.t("rule_name") } else { r.t("branch_name") };
        if !covers(&name, base) { continue; }
        for c in strings(r.at("status_check_contexts")).into_iter().filter(|c| !c.is_empty() && !c.contains('*')) { if !out.contains(&c) { out.push(c); } }
    }
    out
}
