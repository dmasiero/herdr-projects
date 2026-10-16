//! A pull request merged already when the board goes to merge it (land/merged.rs, finish/follow_up.rs), on GitHub and on
//! Gitea: nothing new on the branch, the task is done with that merge; newer work on it goes in a follow-up pull request
//! from the same branch, which is checked and waits for approval as any other. Gitea's HTTP 405 "already merged" and
//! gh's "was already merged" are taken for merged. A chat with a task whose pull request the board merged (OPS-3) opens
//! a follow-up too, instead of pushing to the merged one.
use crate::common;
use common::tea::{gitea_world, tea_log};
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::finish_line;
use sw::json::*;

fn pr_world() -> World { world_with(Opts { repo: true, origin: true, settings: json!({ "review.when": "never" }), ..Default::default() }) }
/// The branch's work is on main at the remote, as a merge made elsewhere leaves it.
fn on_main(t: &Obj) { git(Path::new(t.s("worktree")), &["push", "-q", "origin", "HEAD:main"]); }
fn merged_view(head: &str) -> serde_json::Value { json!({ "state": "MERGED", "headRefOid": head, "baseRefName": "main", "mergeCommit": { "oid": "m".repeat(40) }, "comments": [], "reviews": [], "author": { "login": "me" } }) }
fn head(t: &Obj) -> String { git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]) }

#[test]
fn on_github_a_pull_request_merged_elsewhere_with_nothing_new_on_the_branch_is_done_with_that_merge() {
    let w = pr_world();
    let id = to_approval(&w, "Merged elsewhere");
    let t = task(&id);
    on_main(&t);
    w.gh_says(json!({ "view": merged_view(&head(&t)) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Already merged: https://github.com/o/r/pull/7"), "{}", t.s("msg"));
    assert_eq!(t.s("mergeCommit"), "m".repeat(40));
    assert!(t.b("prMerged"));
    assert!(!gh_log(&w).contains("pr merge 7 --squash"), "{}", gh_log(&w));
}

#[test]
fn on_github_a_pull_request_merged_elsewhere_with_newer_work_on_the_branch_gets_a_follow_up_that_waits_for_approval() {
    let w = pr_world();
    let id = to_approval(&w, "Went on");
    let t = task(&id);
    let branch = t.t("branch");
    w.gh_says(json!({ "view": merged_view(&head(&t)), "createUrl": "https://github.com/o/r/pull/8" }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(!t.b("failure"), "{}", t.at("failure"));
    assert_eq!(t.s("pr"), "https://github.com/o/r/pull/8");
    assert_eq!(t.at("prEarlier"), &json!(["https://github.com/o/r/pull/7"]));
    assert!(!t.b("prMerged") && !t.b("mergeIntent"), "the approval was for the merged one");
    let log = gh_log(&w);
    assert!(log.contains(&format!("pr create --draft --head {branch} --title Went on (follow-up) --body Follow-up to https://github.com/o/r/pull/7.\n")), "{log}");
    assert!(log.contains("\n\nTask: ") && log.contains(" --base main\n"), "{log}");
    assert!(!log.contains("pr merge 7 --squash"), "{log}");
    // approved, the follow-up merges as any pull request does
    w.gh_says(json!({ "head": head(&t) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Merged: https://github.com/o/r/pull/8"), "{}", t.s("msg"));
    assert!(gh_log(&w).contains("pr merge 8 --squash"), "{}", gh_log(&w));
}

#[test]
fn ghs_already_merged_answer_is_taken_for_merged_not_for_an_unknown_error() {
    let w = pr_world();
    let id = to_approval(&w, "Raced");
    let t = task(&id);
    on_main(&t);
    w.gh_says(json!({ "mergeFail": "X Pull request o/r#7 (Raced) was already merged" }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Already merged: https://github.com/o/r/pull/7"), "{}", t.s("msg"));
}

#[test]
fn a_chat_with_a_task_whose_pull_request_the_board_merged_opens_a_follow_up_instead_of_pushing_to_the_merged_one() {
    let w = pr_world();
    let id = to_approval(&w, "Chatted on");
    let t = task(&id);
    w.gh_says(json!({ "head": head(&t), "createUrl": "https://github.com/o/r/pull/8" }));
    // the board merged it, and its space stayed (its agent was in use again, as on OPS-3)
    on_main(&t);
    w.gh.merged.borrow_mut().push("7".into());
    sw::store::tasks::update_wait(&id, 10000, |t| { t.put("stage", "done"); t.put("msg", "Merged: https://github.com/o/r/pull/7"); t.put("prMerged", true); t.put("prState", "merged"); t.put("mergedAt", sw::clock::now()); Ok(sw::store::tasks::Change::Write) }).unwrap();
    // the human goes on in the agent's pane, as on OPS-3: the agent commits more on the same branch
    sw::clock::set(Some(sw::clock::now()));
    w.script(json!({ "worker": [{ "files": { "g.txt": "more\n" }, "commit": "more", "note": "Did more." }] }));
    sw::actions::talk::reply(&id, "Do both", "you").unwrap();
    settle(&id);
    sw::clock::advance(61_000);
    sw::housekeeping::housekeeping(&w.socket, sw::housekeeping::Force::Yes);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{} {}", t.s("msg"), t.at("failure"));
    assert_eq!(t.s("pr"), "https://github.com/o/r/pull/8");
    assert_eq!(t.at("prEarlier"), &json!(["https://github.com/o/r/pull/7"]));
    assert!(gh_log(&w).contains("--title Chatted on (follow-up)"), "{}", gh_log(&w));
}

#[test]
fn on_gitea_a_pull_request_merged_already_with_nothing_new_is_done_and_tea_never_merges_it_again() {
    let w = gitea_world(json!({ "review.when": "never" }));
    let id = to_approval(&w, "Gitea merged");
    let t = task(&id);
    on_main(&t);
    { let mut p = w.tea.pulls.borrow_mut(); p[0]["merged"] = json!(true); p[0]["state"] = json!("closed"); p[0]["merge_commit_sha"] = json!("08377dd"); }
    w.tea_says(json!({ "head": head(&t) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Already merged: https://gitea.test/o/r/pulls/7"), "{}", t.s("msg"));
    assert_eq!(t.s("mergeCommit"), "08377dd");
    assert!(!tea_log(&w).contains("pulls/7/merge"), "{}", tea_log(&w));
}

#[test]
fn on_gitea_a_405_already_merged_with_newer_work_on_the_branch_gets_a_follow_up_that_waits_for_approval() {
    let w = gitea_world(json!({ "review.when": "never" }));
    let id = to_approval(&w, "OPS-3 again");
    let t = task(&id);
    let branch = t.t("branch");
    // Gitea's own answer on OPS-3, while its pull request still reads as open here
    w.tea_says(json!({ "head": head(&t), "mergeFail": "The PR is already merged" }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval", "{}", t.at("failure"));
    assert!(!t.b("failure") && t.at("failure").is_null(), "not 'Pull request could not merge': {}", t.at("failure"));
    assert_eq!(t.s("pr"), "https://gitea.test/o/r/pulls/8");
    assert_eq!(t.at("prEarlier"), &json!(["https://gitea.test/o/r/pulls/7"]));
    let log = tea_log(&w);
    assert!(log.contains(&format!("POST repos/o/r/pulls {{\"head\":\"{branch}\",\"base\":\"main\",\"title\":\"WIP: OPS-3 again (follow-up)\"")), "{log}");
    assert_eq!(gh_log(&w), "");
    // approved, the follow-up merges
    w.tea_says(json!({ "head": head(&t) }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Merged: https://gitea.test/o/r/pulls/8"), "{}", t.s("msg"));
    assert!(tea_log(&w).contains("POST repos/o/r/pulls/8/merge"), "{}", tea_log(&w));
}
