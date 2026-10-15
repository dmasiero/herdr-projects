//! A stand-in for the tea program and the Gitea server behind it, answering in the test's own thread
//! (forge::tea::set_fake_tea): its logins, and the API calls the board makes, with pull requests that are kept between
//! calls. Canned answers set by the test (`w.tea_says(json!({..}))`), every call logged (`tea_log(&w)`). No real tea,
//! no network. A world's repo is on Gitea with `gitea_world()`.
use super::{git, world_with, Opts, World};
use serde_json::{json, Value};
use std::cell::RefCell;
use sw::forge::tea::TeaOut;
use sw::json::*;

/// What it answers: `logins` (tea's logins; one for gitea.test by default), `noTea` (tea is not installed), `head` (the
/// pull request's head commit), `statuses` (its commit's statuses, Gitea's own: { context, status, target_url }),
/// `reviews`, `comments`, `protections` (branch protection rules), `mergeFail` (Gitea refuses the merge with that
/// message), `noActions` (Actions off for the repo), `archived`, `unauthorized` (every API call answers 401).
#[derive(Default)]
pub struct FakeTea { pub cfg: RefCell<Value>, pub log: RefCell<Vec<String>>, pub pulls: RefCell<Vec<Value>> }
fn out(code: u16, body: Value) -> TeaOut { TeaOut { status: 0, stdout: if body.is_null() { String::new() } else { body.to_string() }, stderr: format!("HTTP/1.1 {code} X\n") } }
impl FakeTea {
    pub fn run(&self, a: &[String]) -> TeaOut {
        let c = self.cfg.borrow().clone();
        if c.b("noTea") { return TeaOut { status: 127, stdout: String::new(), stderr: sw::forge::tea::NOT_INSTALLED.into() }; }
        if a.first().map(|s| s.as_str()) == Some("login") {
            self.log.borrow_mut().push(a.join(" "));
            let l = if c.at("logins").is_array() { c.at("logins").clone() } else { json!([{ "name": "home", "url": "https://gitea.test", "ssh_host": "gitea.test" }]) };
            return TeaOut { status: 0, stdout: l.to_string(), stderr: String::new() };
        }
        let flag = |f: &str| a.iter().position(|s| s == f).and_then(|i| a.get(i + 1)).cloned();
        let method = flag("-X").unwrap_or_else(|| "GET".into());
        let body: Value = flag("-d").and_then(|d| serde_json::from_str(&d).ok()).unwrap_or(Value::Null);
        let path = a.last().cloned().unwrap_or_default();
        self.log.borrow_mut().push(format!("{method} {path}{}", if body.is_null() { String::new() } else { format!(" {body}") }));
        if c.b("unauthorized") { return out(401, json!({ "message": "token is required" })); }
        let path = path.split('?').next().unwrap_or("").to_string();
        let parts: Vec<&str> = path.split('/').collect();
        let pull = |n: &str| self.pulls.borrow().iter().find(|p| p.at("number").to_string() == n).cloned();
        match (method.as_str(), parts.as_slice()) {
            ("GET", ["user"]) => out(200, json!({ "login": "me" })),
            ("GET", ["repos", _, _]) => out(200, json!({ "default_branch": "main", "archived": c.b("archived"), "has_actions": !c.b("noActions") })),
            ("POST", ["repos", o, r, "pulls"]) => {
                let n = 7 + self.pulls.borrow().len() as i64;
                let p = json!({ "number": n, "html_url": format!("https://gitea.test/{o}/{r}/pulls/{n}"), "state": "open", "merged": false, "title": body.at("title"), "body": body.at("body"),
                    "head": { "ref": body.at("head") }, "base": { "ref": body.at("base") }, "user": { "login": "me" } });
                self.pulls.borrow_mut().push(p.clone());
                out(201, p)
            }
            ("GET", ["repos", _, _, "pulls"]) => out(200, Value::Array(self.pulls.borrow().iter().rev().cloned().collect())),
            ("GET", ["repos", _, _, "pulls", n]) => match pull(n) {
                Some(mut p) => { p["head"]["sha"] = c.at("head").clone(); out(200, p) }
                None => out(404, json!({ "message": "pull request does not exist" })),
            },
            ("PATCH", ["repos", _, _, "pulls", n]) => {
                for p in self.pulls.borrow_mut().iter_mut().filter(|p| p.at("number").to_string() == *n) { if body.b("title") { p["title"] = body.at("title").clone(); } }
                out(201, pull(n).unwrap_or(Value::Null))
            }
            ("POST", ["repos", _, _, "pulls", n, "merge"]) => {
                if c.b("mergeFail") { return out(405, json!({ "message": c.at("mergeFail") })); }
                if pull(n).map(|p| sw::forge::gitea_view::is_wip(p.s("title"))).unwrap_or(false) { return out(405, json!({ "message": "work in progress" })); }
                if c.b("head") && body.b("head_commit_id") && body.s("head_commit_id") != c.s("head") { return out(409, json!({ "message": "head out of date" })); }
                for p in self.pulls.borrow_mut().iter_mut().filter(|p| p.at("number").to_string() == *n) { p["merged"] = json!(true); p["state"] = json!("closed"); p["merge_commit_sha"] = json!("abc123"); }
                out(200, Value::Null)
            }
            ("GET", ["repos", _, _, "pulls", _, "reviews"]) => out(200, if c.at("reviews").is_array() { c.at("reviews").clone() } else { json!([]) }),
            ("GET", ["repos", _, _, "issues", _, "comments"]) => out(200, if c.at("comments").is_array() { c.at("comments").clone() } else { json!([]) }),
            ("POST", ["repos", _, _, "issues", _, "comments"]) => out(201, json!({ "id": 1, "body": body.at("body") })),
            ("GET", ["repos", _, _, "commits", _, "status"]) => out(200, json!({ "statuses": if c.at("statuses").is_array() { c.at("statuses").clone() } else { Value::Null } })),
            ("GET", ["repos", _, _, "branch_protections"]) => out(200, if c.at("protections").is_array() { c.at("protections").clone() } else { json!([]) }),
            _ => out(404, json!({ "message": format!("fake gitea: unknown {method} {path}") })),
        }
    }
}
/// A world whose repo's remote is on the Gitea server gitea.test (pushes still go to the bare repo beside it).
pub fn gitea_world(settings: Value) -> World {
    let w = world_with(Opts { repo: true, origin: true, settings, ..Default::default() });
    let bare = w.base.join("origin.git").display().to_string();
    let url = "ssh://git@gitea.test:2222/o/r.git";
    git(&w.root, &["remote", "set-url", "origin", url]);
    git(&w.root, &["config", &format!("url.{bare}.insteadOf"), url]);
    w
}
pub fn tea_log(w: &World) -> String { w.tea.log.borrow().iter().map(|l| format!("{l}\n")).collect() }
