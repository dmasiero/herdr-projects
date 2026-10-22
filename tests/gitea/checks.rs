//! A Gitea pull request's checks (`merge.checks`, forge/gitea_checks.rs): its commit statuses still pending, the merge
//! waits and says so; one failed, nothing merges and the link to its log is said; a status the branch protection
//! requires is waited for; with workflows and none reported yet it waits a few minutes, then merges; a repo with no
//! workflows never waits.
use crate::common;
use common::tea::{gitea_world, tea_log};
use common::*;
use serde_json::json;
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;

fn merges(w: &World) -> usize { tea_log(w).lines().filter(|l| l.contains("/merge ")).count() }
fn status(state: &str) -> serde_json::Value { json!([{ "context": "ci / build (pull_request)", "status": state, "target_url": "https://gitea.test/o/r/actions/runs/3/jobs/0" }]) }

#[test]
fn pending_statuses_are_waited_for_and_the_merge_goes_on_when_they_pass() {
    let w = gitea_world(json!({ "review.when": "never" }));
    let id = to_approval(&w, "Pending");
    let head = task(&id).at("prHead").clone();
    w.tea_says(json!({ "head": head, "statuses": status("pending") }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!((t.s("stage"), t.b("autoMerge")), ("merging", true), "{}", t.at("failure"));
    assert_eq!(t.s("msg"), "Waiting for Gitea checks · 0 of 1 passed");
    assert_eq!(merges(&w), 0);
    // Gitea's warning blocks nothing
    w.tea_says(json!({ "head": head, "statuses": status("warning") }));
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(merges(&w), 1);
}

#[test]
fn a_failed_status_stops_the_merge_with_the_link_to_its_log() {
    let w = gitea_world(json!({ "review.when": "never", "review.rounds": 0 }));
    let id = to_approval(&w, "Failing");
    w.tea_says(json!({ "head": task(&id).at("prHead"), "statuses": status("failure") }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval");
    assert!(t.s("msg").starts_with("Gitea's checks failed: ci / build (pull_request)"), "{}", t.s("msg"));
    let d = t.at("prFailure").s("detail");
    assert!(d.contains("Its log is on Gitea: https://gitea.test/o/r/actions/runs/3/jobs/0") && d.contains("on the pull request's page"), "{d}");
    assert_eq!(merges(&w), 0);
}

#[test]
fn a_status_the_branch_protection_requires_is_waited_for_until_it_is_listed() {
    let w = gitea_world(json!({ "review.when": "never" }));
    let id = to_approval(&w, "Required");
    let head = task(&id).at("prHead").clone();
    w.tea_says(json!({ "head": head, "protections": [{ "rule_name": "main", "enable_status_check": true, "status_check_contexts": ["lint", "ci / *"] }] }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "merging");
    assert_eq!(strings(t.at("checksWait").at("names")), vec!["lint".to_string()]);
    w.tea_says(json!({ "head": head, "statuses": [{ "context": "lint", "status": "success" }], "protections": [{ "rule_name": "main", "enable_status_check": true, "status_check_contexts": ["lint"] }] }));
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
}

#[test]
fn with_workflows_and_no_status_yet_it_waits_a_few_minutes_then_merges_and_with_actions_off_it_does_not_wait() {
    for actions in [true, false] {
        let w = gitea_world(json!({ "review.when": "never" }));
        w.script(json!({ "worker": [{ "files": { ".gitea/workflows/ci.yml": "on: [pull_request]\n" }, "commit": "ci", "note": "Added CI." }] }));
        let id = to_approval(&w, "Workflows");
        w.tea_says(json!({ "head": task(&id).at("prHead"), "noActions": !actions }));
        sw::clock::set(Some(sw::clock::now()));
        finish_line::approve(&id, None, false).unwrap();
        let t = settle(&id);
        if !actions { assert_eq!(t.s("stage"), "done", "{}", t.at("failure")); continue; }
        assert_eq!(t.s("stage"), "merging");
        assert_eq!(t.s("msg"), "Waiting for Gitea checks to start");
        poll_prs(); assert_eq!(settle(&id).s("stage"), "merging", "still within the grace");
        sw::clock::advance(sw::forge::gitea_checks::GRACE_MS + 1000);
        poll_prs(); let t = settle(&id);
        assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
        assert_eq!(merges(&w), 1);
    }
}
