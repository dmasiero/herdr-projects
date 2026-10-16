//! A pull request found merged already when the board goes to merge it (merged outside the board, or the forge refused
//! the merge as "already merged": Gitea's HTTP 405, gh's "was already merged"). The task's branch is compared with the
//! base: nothing new on it, the task is done with that merge; work the merge did not have goes in a follow-up pull
//! request (finish/follow_up.rs), which then goes through the finish line as any other: check, review, approval.
use super::merge::{fail, landed};
use crate::err::Res;
use crate::finish::follow_up;
use crate::flow;
use crate::json::*;
use crate::runner::Guard;
use crate::store::repos::git_out;
use crate::store::tasks::Task;
use std::path::Path;

/// `out`: what the forge said when it refused the merge, if it did. Ok(false): the pull request is not merged, the
/// merge goes on (or fails) as before.
pub(super) fn handled(id: &str, g: &mut Guard, t: &Task, root: &Path, out: Option<&str>) -> Res<bool> {
    let seen = follow_up::view(root, t.s("pr")).unwrap_or(Value::Null);
    if seen.s("state") != "MERGED" && !out.map(follow_up::says_merged).unwrap_or(false) { return Ok(false); }
    let tree = crate::sync::tree_of(t);
    let at = if tree.is_empty() { root.to_path_buf() } else { Path::new(&tree).to_path_buf() };
    let reviewed = t.at("mergeIntent").t("verifiedHead");
    let head = if !reviewed.is_empty() { reviewed } else if !tree.is_empty() { flow::git::head_of(&at) } else { t.t("prHead") };
    let base = if seen.b("baseRefName") { seen.t("baseRefName") } else { crate::sync::main_branch(root, t) };
    let _ = crate::sync::fetch_main(&at, &base);
    let new = git_out(&at, &["diff", "--name-only", &format!("origin/{base}...{head}")]);
    let commit = seen.at("mergeCommit").t("oid");
    match new {
        Ok(files) if files.trim().is_empty() => {
            // Everything on the branch is on the base: that merge was this task's.
            flow::git::drop_remote_branch(root, t.s("branch"), &head);
            landed(id, g, t, &format!("Already merged: {}", t.s("pr")), &head, &commit)?;
        }
        Ok(_) => {
            let summary = if t.b("summary") { t.t("summary") } else { t.t("title") };
            match follow_up::open(g, t, &at, &summary) {
                Ok(url) => {
                    g.set(|t| { t.put("mergeIntent", Value::Null); t.put("msg", format!("{} was merged already: follow-up {url}", t.arr("prEarlier").last().map(text).unwrap_or_default())); })?;
                    crate::finish::pipeline::pipeline(id, g, &summary, false, "", None)?;
                }
                Err(e) => {
                    let (cli, name) = crate::forge::cli_of(root);
                    let said = e.msg.strip_prefix("pr:").unwrap_or(&e.msg).to_string();
                    fail(g, "Could not open the follow-up pull request", &format!("Pull request: {} (merged already)\nBranch: {}\n\nThe branch has work that merge did not bring in.\n\n{cli} pr create:\n{said}", t.s("pr"), t.s("branch")), &format!("Inspect the {name} error, then approve again: the board opens the follow-up pull request then."), super::merge::no_fix())?;
                }
            }
        }
        Err(e) => { fail(g, "Could not compare the branch with its base", &format!("Pull request: {} (merged already)\nBranch: {}\nBase: {base}\n\n{}", t.s("pr"), t.s("branch"), e.msg), "Investigate the Git error, then approve again.", Default::default())?; }
    }
    Ok(true)
}
