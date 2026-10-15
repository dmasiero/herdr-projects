//! A Gitea pull request in the shape `gh pr view --json` gives one, so that what reads GitHub's reads it the same:
//! its state (`OPEN`, `CLOSED`, `MERGED`), its head and base, its merge commit, its comments and reviews, the review
//! decision they make, and its commit's statuses (Gitea Actions report as those) as `statusCheckRollup`.
use super::tea::{api, said, Answer};
use super::Repo;
use crate::json::*;
use std::path::Path;

const WAIT_MS: u64 = 60000;
pub fn get(repo: &Repo, path: &str, cwd: &Path) -> Result<Value, String> {
    let a: Answer = api(&repo.login, "GET", path, None, cwd, WAIT_MS)?;
    if a.ok() { Ok(a.body) } else { Err(format!("{} ({path})", said(&a))) }
}
pub fn state_of(p: &Value) -> &'static str { if p.b("merged") { "MERGED" } else if p.s("state") == "closed" { "CLOSED" } else { "OPEN" } }
/// What `gh pr list --json number,url,state,…` gives of one.
pub fn short(p: &Value) -> Value {
    json!({ "number": p.at("number"), "url": p.at("html_url"), "state": state_of(p), "title": p.at("title"), "headRefName": p.at("head").at("ref"), "headRefOid": p.at("head").at("sha"), "baseRefName": p.at("base").at("ref"), "isDraft": p.b("draft") || is_wip(p.s("title")) })
}
/// Gitea's draft: a title that starts with one of its work-in-progress marks.
pub fn is_wip(title: &str) -> bool { let l = title.trim_start().to_lowercase(); l.starts_with("wip:") || l.starts_with("[wip]") }
pub fn without_wip(title: &str) -> String {
    let t = title.trim_start();
    let cut = if t.to_lowercase().starts_with("wip:") { 4 } else if t.to_lowercase().starts_with("[wip]") { 5 } else { 0 };
    t[cut..].trim_start().to_string()
}

/// A commit status's state, in GitHub's words: Gitea's `warning` passes (it blocks nothing there either).
pub fn state_word(s: &str) -> String {
    match s.to_lowercase().as_str() { "success" => "SUCCESS", "failure" => "FAILURE", "error" => "ERROR", "warning" => "NEUTRAL", "skipped" => "SKIPPED", "" | "pending" => "PENDING", o => return o.to_uppercase() }.into()
}
/// The latest status of each context on a commit, as GitHub's `StatusContext`.
pub fn statuses(repo: &Repo, sha: &str, cwd: &Path) -> Result<Vec<Value>, String> {
    if sha.is_empty() { return Ok(vec![]); }
    let v = get(repo, &format!("{}/commits/{sha}/status", repo.path()), cwd)?;
    Ok(v.arr("statuses").iter().map(|s| {
        let st = if s.b("status") { s.s("status") } else { s.s("state") };
        json!({ "__typename": "StatusContext", "context": s.at("context"), "state": state_word(st), "targetUrl": s.at("target_url"), "description": s.at("description") })
    }).collect())
}
fn review_state(s: &str) -> &str { match s { "REQUEST_CHANGES" => "CHANGES_REQUESTED", "COMMENT" => "COMMENTED", o => o } }
/// GitHub's review decision, from each reviewer's last say: one asking for changes, else one approving, else none.
pub fn decision(reviews: &[Value]) -> &'static str {
    let mut last: Vec<(String, String)> = vec![];
    for r in reviews.iter().filter(|r| !r.b("dismissed") && !r.b("stale") && ["APPROVED", "CHANGES_REQUESTED"].contains(&r.s("state"))) {
        let who = r.at("author").t("login");
        last.retain(|(w, _)| *w != who);
        last.push((who, r.t("state")));
    }
    if last.iter().any(|(_, s)| s == "CHANGES_REQUESTED") { "CHANGES_REQUESTED" } else if last.iter().any(|(_, s)| s == "APPROVED") { "APPROVED" } else { "" }
}
fn reviews(repo: &Repo, n: &str, cwd: &Path) -> Result<Vec<Value>, String> {
    let v = get(repo, &format!("{}/pulls/{n}/reviews", repo.path()), cwd)?;
    Ok(v.as_array().cloned().unwrap_or_default().iter().filter(|r| r.s("state") != "PENDING" && r.s("state") != "REQUEST_REVIEW").map(|r| json!({
        "id": r.at("id"), "author": { "login": r.at("user").at("login") }, "body": r.at("body"), "state": review_state(r.s("state")),
        "submittedAt": r.at("submitted_at"), "dismissed": r.b("dismissed"), "stale": r.b("stale"),
    })).collect())
}
fn comments(repo: &Repo, n: &str, cwd: &Path) -> Result<Vec<Value>, String> {
    let v = get(repo, &format!("{}/issues/{n}/comments", repo.path()), cwd)?;
    Ok(v.as_array().cloned().unwrap_or_default().iter().map(|c| json!({
        "id": c.at("id"), "url": c.at("html_url"), "author": { "login": c.at("user").at("login") }, "body": c.at("body"), "updatedAt": c.at("updated_at"),
    })).collect())
}
/// `gh pr view <n> --json <fields>`: only what is asked for is fetched.
pub fn view(repo: &Repo, n: &str, fields: &str, cwd: &Path) -> Result<Value, String> {
    let p = get(repo, &format!("{}/pulls/{n}", repo.path()), cwd)?;
    let mut out = obj(short(&p));
    let want: Vec<&str> = fields.split(',').map(str::trim).collect();
    if want.contains(&"mergeCommit") { out.put("mergeCommit", if p.b("merge_commit_sha") { json!({ "oid": p.at("merge_commit_sha") }) } else { Value::Null }); }
    if want.contains(&"author") { out.put("author", json!({ "login": p.at("user").at("login") })); }
    if want.contains(&"comments") { out.put("comments", comments(repo, n, cwd)?); }
    if want.contains(&"reviews") || want.contains(&"reviewDecision") {
        let r = reviews(repo, n, cwd)?;
        out.put("reviewDecision", decision(&r));
        if want.contains(&"reviews") { out.put("reviews", r); }
    }
    if want.contains(&"statusCheckRollup") { out.put("statusCheckRollup", statuses(repo, &p.at("head").t("sha"), cwd)?); }
    Ok(Value::Object(out))
}
