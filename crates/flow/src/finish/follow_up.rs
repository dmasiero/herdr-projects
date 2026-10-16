//! A follow-up pull request: the task's pull request merged already, and its branch has work since that never reached
//! the base branch (a chat with the done task went on with it, OPS-3). The board opens a second pull request from the
//! same branch, titled after the task with "(follow-up)", and takes it as the task's own. Also how a forge words "that
//! pull request is merged already" when asked to merge it (Gitea's HTTP 405, gh's "was already merged").
use super::already::open_pr;
use crate::err::{Error, Res};
use crate::flow::git::{gh, pr_number};
use crate::json::*;
use crate::runner::Guard;
use crate::store::keys::{repo_keys, task_ref};
use crate::store::tasks::Task;
use std::path::Path;

pub const MARK: &str = " (follow-up)";

/// The board's own record says the task's pull request merged: it merged it, or saw it merged elsewhere.
pub fn merged(t: &Task) -> bool { t.b("pr") && (t.b("prMerged") || t.s("prState") == "merged") }
/// A refused merge, in the forge's words, because the pull request is merged already: Gitea answers `HTTP 405: The PR
/// is already merged`, gh `Pull request #7 was already merged`.
pub fn says_merged(out: &str) -> bool { out.to_lowercase().contains("already merged") }
/// The pull request as its forge has it now: `state`, `mergeCommit`, `baseRefName`, `url`. None when it cannot say.
pub fn view(cwd: &Path, pr: &str) -> Option<Value> {
    let r = gh(&["pr", "view", &pr_number(pr), "--json", "state,mergeCommit,baseRefName,url"], cwd, 60000);
    if r.status != 0 { return None; }
    serde_json::from_str::<Value>(r.stdout.trim()).ok().filter(|v| v.is_object())
}
/// The follow-up's title: the task's, marked.
pub fn title_of(t: &Task) -> String { format!("{}{MARK}", if t.b("title") { t.t("title") } else { t.t("id") }) }
/// Opens the follow-up pull request from the task's branch to the base the merged one went into, as a draft, and
/// records it as the task's pull request: the earlier one is kept in `prEarlier`. Err `pr:<what the forge said>`.
pub fn open(g: &mut Guard, t: &Task, cwd: &Path, text: &str) -> Res<String> {
    let old = t.t("pr");
    let seen = view(cwd, &old).unwrap_or(Value::Null);
    let base = seen.t("baseRefName");
    let commit = seen.at("mergeCommit").t("oid");
    let at = if commit.is_empty() { String::new() } else { format!(" as {}", &commit[..commit.len().min(7)]) };
    let into = if base.is_empty() { String::new() } else { format!(" into {base}") };
    let what = if text.trim().is_empty() { t.t("title") } else { text.trim().to_string() };
    let body = format!("Follow-up to {old}.\n\nThat pull request was merged{into}{at}, and the task's branch has work since that it did not bring in. This pull request brings in that work.\n\n{what}\n\nTask: {}", task_ref(t.s("id"), &repo_keys()));
    let title = title_of(t);
    let mut a = vec!["pr", "create", "--draft", "--head", t.s("branch"), "--title", title.as_str(), "--body", body.as_str()];
    if !base.is_empty() { a.extend(["--base", base.as_str()]); }
    let r = gh(&a, cwd, 120000);
    let mut url = r.stdout.split_whitespace().find(|w| w.starts_with("http://") || w.starts_with("https://")).unwrap_or("").to_string();
    if url.is_empty() || url == old { url = open_pr(t.s("branch"), cwd).filter(|u| *u != old).unwrap_or_default(); }
    if url.is_empty() { return Err(Error::new(format!("pr:{}", [r.stdout.trim(), r.stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")))); }
    g.set(|t| {
        let mut earlier = t.arr("prEarlier").to_vec();
        earlier.push(json!(old));
        t.put("prEarlier", earlier);
        t.put("pr", url.clone()); t.put("prSeen", ""); t.put("prState", "waiting"); t.put("prMerged", false); t.put("prFailure", Value::Null);
    })?;
    Ok(url)
}
/// A task done with nothing to land whose pull request its forge has as merged (merged elsewhere): that merge is the
/// task's, recorded as the board's own merge would be.
pub fn record_merged(g: &mut Guard, t: &Task, cwd: &Path) -> Res<()> {
    if !t.b("pr") || t.b("prMerged") { return Ok(()); }
    let Some(v) = view(cwd, t.s("pr")).filter(|v| v.s("state") == "MERGED") else { return Ok(()) };
    let commit = v.at("mergeCommit").t("oid");
    g.set(|t| { t.put("prMerged", true); t.put("prState", "merged"); if !commit.is_empty() { t.put("mergeCommit", commit.clone()); } if !t.b("mergedAt") { t.put("mergedAt", crate::clock::now()); } })?;
    Ok(())
}
