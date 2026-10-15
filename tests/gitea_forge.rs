//! Which forge a repo is on, from its remote (`merge.forge`, forge.rs): github.com is GitHub and tea is never asked, a
//! host tea has a login for is Gitea, any other host stays GitHub with a hint where gh finds no GitHub host, and
//! `gitea` said outright with no login or no tea says why plainly.
use crate::common;
use common::tea::{gitea_world, tea_log};
use common::*;
use serde_json::json;
use sw::forge::{of, parse_remote, Forge, Remote, Repo};
use sw::store::settings::{change::set_value, lookup::Scope};

fn remote(h: &str, o: &str, n: &str) -> Option<Remote> { Some(Remote { host: h.into(), owner: o.into(), name: n.into() }) }

#[test]
fn a_remote_is_read_in_each_of_its_forms_and_a_folder_has_no_host() {
    assert_eq!(parse_remote("ssh://git@gitea.masiero.internal:2222/doug/dotfiles.git"), remote("gitea.masiero.internal", "doug", "dotfiles"));
    assert_eq!(parse_remote("git@github.com:o/r.git"), remote("github.com", "o", "r"));
    assert_eq!(parse_remote("https://Gitea.Example.org/sub/o/r/"), remote("gitea.example.org", "o", "r"));
    assert_eq!(parse_remote("https://user:token@host.test:3000/o/r"), remote("host.test", "o", "r"));
    assert_eq!(parse_remote("/srv/git/r.git"), None);
    assert_eq!(parse_remote("../r.git"), None);
    assert_eq!(sw::forge::pr_number("https://gitea.test/o/r/pulls/12"), "12");
    assert_eq!(sw::forge::pr_number("https://github.com/o/r/pull/7"), "7");
}

#[test]
fn github_com_and_a_folder_are_github_without_asking_tea_and_a_host_tea_has_a_login_for_is_gitea() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    assert_eq!(of(&w.root), Forge::GitHub);
    git(&w.root, &["remote", "set-url", "origin", "git@github.com:o/r.git"]);
    assert_eq!(of(&w.root), Forge::GitHub);
    assert_eq!(tea_log(&w), "", "tea is not asked for a repo on GitHub");
    git(&w.root, &["remote", "set-url", "origin", "ssh://git@gitea.test:2222/o/r.git"]);
    assert_eq!(of(&w.root), Forge::Gitea(Repo { login: "home".into(), host: "gitea.test".into(), owner: "o".into(), name: "r".into() }));
    // found by the address of the login too, and from a worktree of the repo
    w.tea_says(json!({ "logins": [{ "name": "work", "url": "https://gitea.test:3000", "ssh_host": "" }] }));
    let wt = w.base.join("wt");
    git(&w.root, &["worktree", "add", "-q", "-b", "side", &wt.display().to_string()]);
    assert!(matches!(of(&wt), Forge::Gitea(r) if r.login == "work"));
}

#[test]
fn another_host_stays_github_and_gh_failing_to_find_it_says_how_to_reach_it_through_tea() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    git(&w.root, &["remote", "set-url", "origin", "ssh://git@git.other.test/o/r.git"]);
    assert_eq!(of(&w.root), Forge::GitHub);
    w.gh_says(json!({ "createFail": "none of the git remotes configured for this repository point to a known GitHub host" }));
    let r = sw::flow::git::gh(&["pr", "create", "--head", "b"], &w.root, 1000);
    assert!(r.stderr.contains("This repo's remote is on git.other.test, not GitHub."), "{}", r.stderr);
    assert!(r.stderr.contains("tea is not logged in to git.other.test") && r.stderr.contains("merge.forge"), "{}", r.stderr);
}

#[test]
fn merge_forge_says_it_outright_per_repo_and_gitea_without_a_login_or_without_tea_says_why() {
    let w = gitea_world(json!({}));
    let (name, _) = sw::store::repos::repo_of(&w.root).unwrap();
    set_value("merge.forge", &json!("github"), &Scope::repo(&name)).unwrap();
    assert_eq!(of(&w.root), Forge::GitHub);
    set_value("merge.forge", &json!("gitea"), &Scope::repo(&name)).unwrap();
    assert!(matches!(of(&w.root), Forge::Gitea(_)));
    w.tea_says(json!({ "logins": [] }));
    let Forge::Broken(why) = of(&w.root) else { panic!("not broken") };
    assert!(why.contains("tea is not logged in to gitea.test") && why.contains("tea login add --url https://gitea.test"), "{why}");
    assert!(sw::store::failures::is_auth_failure(&why), "a login is missing: {why}");
    w.tea_says(json!({ "noTea": true }));
    let Forge::Broken(why) = of(&w.root) else { panic!("not broken") };
    assert!(why.contains("tea, Gitea's command line, is not installed"), "{why}");
    // auto, without tea: as before, GitHub
    sw::store::settings::change::unset_value("merge.forge", &Scope::repo(&name)).unwrap();
    assert_eq!(of(&w.root), Forge::GitHub);
}
