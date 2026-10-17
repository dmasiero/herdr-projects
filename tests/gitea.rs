//! A task in a repo on Gitea: its pull request is opened through tea as a draft (`WIP:`), made ready and merged with
//! `merge.method` at the reviewed commit, gh is never run, and a missing login or a missing tea is said plainly where
//! the pull request could not be opened. Its base is read as gh gives it. A merge made elsewhere is found by the pull
//! requests' watch. A task started from a named branch opens its pull request into it and merges only while it still
//! merges there.
use crate::common;
use common::tea::{gitea_world, tea_log};
use common::*;
use serde_json::json;
use std::path::Path;
use sw::actions::finish_line;
use sw::housekeeping::poll_prs;
use sw::json::*;

#[test]
fn a_pull_request_on_gitea_is_opened_as_a_draft_made_ready_and_merged_with_the_merge_method_at_the_reviewed_commit() {
    let w = gitea_world(json!({ "review.when": "never", "merge.method": "rebase" }));
    let id = to_approval(&w, "On Gitea");
    let t = task(&id);
    assert_eq!(t.s("pr"), "https://gitea.test/o/r/pulls/7");
    let branch = t.t("branch");
    assert!(tea_log(&w).contains(&format!("POST repos/o/r/pulls {{\"head\":\"{branch}\",\"base\":\"main\",\"title\":\"WIP: On Gitea\"")), "{}", tea_log(&w));
    let reviewed = git(Path::new(t.s("worktree")), &["rev-parse", "HEAD"]);
    w.tea_says(json!({ "head": reviewed }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert!(t.s("msg").starts_with("Merged: https://gitea.test/o/r/pulls/7"), "{}", t.s("msg"));
    assert_eq!(t.s("mergeCommit"), "abc123");
    let log = tea_log(&w);
    assert!(log.contains("PATCH repos/o/r/pulls/7 {\"title\":\"On Gitea\"}"), "{log}");
    assert!(log.contains(&format!("POST repos/o/r/pulls/7/merge {{\"Do\":\"rebase\",\"head_commit_id\":\"{reviewed}\"}}")), "{log}");
    assert_eq!(gh_log(&w), "", "gh is never run for a repo on Gitea");
    assert_eq!(git(&w.root, &["ls-remote", "--heads", "origin", &branch]), "", "the merged branch is gone at the remote");
}

#[test]
fn started_from_a_named_branch_its_gitea_pull_request_merges_into_it_and_a_moved_base_is_refused_in_giteas_words() {
    let w = gitea_world(json!({ "review.when": "never" }));
    git(&w.root, &["push", "-q", "origin", "HEAD:refs/heads/develop"]);
    sw::store::settings::change::set_value("workspace.from", &json!("develop"), &sw::store::settings::lookup::Scope::you()).unwrap();
    let id = to_approval(&w, "On develop");
    assert!(tea_log(&w).contains("\"base\":\"develop\""), "{}", tea_log(&w));
    w.tea.pulls.borrow_mut()[0]["base"]["ref"] = json!("main");
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.at("failure").s("title"), "The pull request merges into main, not develop", "{}", t.at("failure"));
    let next = t.at("failure").s("next").to_string();
    assert!(next.contains("back to develop") && next.contains("Gitea page") && !next.contains("gh pr edit"), "{next}");
    w.tea.pulls.borrow_mut()[0]["base"]["ref"] = json!("develop");
    w.tea_says(json!({ "head": task(&id).at("prHead").clone() }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "done", "{}", t.at("failure"));
    assert_eq!(gh_log(&w), "");
}

#[test]
fn gitea_refusing_the_merge_says_so_with_teas_words_and_nothing_merges() {
    let w = gitea_world(json!({ "review.when": "never" }));
    let id = to_approval(&w, "Refused");
    w.tea_says(json!({ "mergeFail": "Please try again later" }));
    finish_line::approve(&id, None, false).unwrap();
    let t = settle(&id);
    assert_eq!(t.s("stage"), "approval");
    assert_eq!(t.at("failure").s("title"), "Pull request could not merge");
    assert!(t.at("failure").s("detail").contains("tea pr merge:\nGitea answered HTTP 405: Please try again later"), "{}", t.at("failure"));
}

#[test]
fn no_login_for_the_gitea_host_or_no_tea_is_said_where_the_pull_request_could_not_be_opened() {
    for (says, words) in [(json!({ "logins": [] }), "tea is not logged in to gitea.test"), (json!({ "noTea": true }), "tea, Gitea's command line, is not installed")] {
        let w = gitea_world(json!({ "review.when": "never", "merge.forge": "gitea" }));
        w.tea_says(says);
        w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }] }));
        let id = w.add("No login");
        delegate(&id);
        let t = settle(&id);
        let f = t.at("failure");
        assert_eq!(f.s("title"), "Could not open the pull request", "{f}");
        assert!(f.s("detail").contains("tea pr create:") && f.s("detail").contains(words), "{f}");
        assert_eq!(gh_log(&w), "");
    }
}

#[test]
fn a_pull_request_merged_or_reviewed_on_gitea_is_found_by_the_watch() {
    let w = gitea_world(json!({ "review.when": "never", "review.rounds": 0 }));
    let id = to_approval(&w, "Watched");
    let reviewed = task(&id).at("prHead").clone();
    w.tea_says(json!({ "head": reviewed, "reviews": [{ "id": 1, "user": { "login": "bob" }, "state": "REQUEST_CHANGES", "body": "no" }] }));
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("prState"), "failing");
    assert_eq!(t.at("prFailure").s("title"), "Pull request changes requested");
    assert!(t.at("prFailure").s("detail").contains("tea pr 7 --comments"), "{}", t.at("prFailure"));
    { let mut p = w.tea.pulls.borrow_mut(); p[0]["merged"] = json!(true); p[0]["state"] = json!("closed"); p[0]["merge_commit_sha"] = json!("def456"); }
    poll_prs(); let t = settle(&id);
    assert_eq!(t.s("stage"), "done");
    assert_eq!((t.s("prState"), t.s("mergeCommit")), ("merged", "def456"));
}

#[test]
fn a_review_decision_is_each_reviewers_last_say() {
    use sw::forge::gitea_view::decision;
    let r = |who: &str, state: &str| json!({ "author": { "login": who }, "state": state });
    assert_eq!(decision(&[r("a", "CHANGES_REQUESTED"), r("a", "APPROVED")]), "APPROVED");
    assert_eq!(decision(&[r("a", "APPROVED"), r("b", "CHANGES_REQUESTED")]), "CHANGES_REQUESTED");
    assert_eq!(decision(&[r("a", "COMMENTED")]), "");
    assert_eq!(sw::forge::gitea_view::without_wip("WIP: x"), "x");
    assert_eq!(sw::forge::gitea_view::without_wip("[wip] x"), "x");
}

#[test]
fn a_pull_requests_base_is_read_as_gh_gives_it_and_changing_it_is_never_told_as_gh_pr_edit() {
    let w = gitea_world(json!({}));
    let gh = |a: &[&str]| { let r = sw::flow::git::gh(a, &w.root, 1000); assert_eq!(r.status, 0, "{}", r.stderr); r.stdout };
    gh(&["pr", "create", "--head", "b", "--base", "develop", "--title", "T", "--body", ""]);
    let listed: serde_json::Value = serde_json::from_str(&gh(&["pr", "list", "--head", "b", "--state", "all", "--limit", "10", "--json", "number,url,state,baseRefName"])).unwrap();
    assert_eq!(listed[0].s("baseRefName"), "develop", "{listed}");
    let viewed: serde_json::Value = serde_json::from_str(&gh(&["pr", "view", "7", "--json", "headRefOid,baseRefName"])).unwrap();
    assert_eq!(viewed.s("baseRefName"), "develop", "{viewed}");
    assert_eq!(gh_log(&w), "", "gh is never run for a repo on Gitea");
    let how = sw::forge::base_edit_how(&w.root, "7", "main");
    assert!(how.contains("Gitea page") && how.contains("main") && !how.contains("gh "), "{how}");
    git(&w.root, &["remote", "set-url", "origin", "git@github.com:o/r.git"]);
    assert_eq!(sw::forge::base_edit_how(&w.root, "7", "main"), "gh pr edit 7 --base main");
}
