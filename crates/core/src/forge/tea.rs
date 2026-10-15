//! tea, Gitea's command line: its logins (`tea login list`, remembered for a while) and one call of Gitea's API
//! through it (`tea api`), with the HTTP status it answered. A test answers for tea itself (`set_fake_tea`), so no
//! program is started then.
use crate::clock::now;
use crate::json::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// What tea gave.
pub struct TeaOut { pub status: i32, pub stdout: String, pub stderr: String }
type Fake = std::rc::Rc<dyn Fn(&[String]) -> TeaOut>;
thread_local! { static FAKE_TEA: std::cell::RefCell<Option<Fake>> = const { std::cell::RefCell::new(None) }; }
pub fn set_fake_tea(f: Option<Fake>) { FAKE_TEA.with(|x| *x.borrow_mut() = f); }
fn faked() -> Option<Fake> { FAKE_TEA.with(|x| x.borrow().clone()) }

pub const NOT_INSTALLED: &str = "tea, Gitea's command line, is not installed here (or not on the PATH): install it (https://gitea.com/gitea/tea) and log in to the Gitea server (tea login add).";
pub fn no_login(host: &str) -> String { format!("tea is not logged in to {host} (no login for it in tea login list): log in to it (tea login add --url https://{host}).") }

pub fn tea(args: &[&str], cwd: &Path, timeout_ms: u64) -> TeaOut {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    if let Some(f) = faked() { return f(&a); }
    crate::login_env::user_env();
    let dir = if cwd.is_dir() { cwd.to_path_buf() } else { std::env::temp_dir() };
    let child = Command::new("tea").args(&a).current_dir(dir).env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let child = match child {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return TeaOut { status: 127, stdout: String::new(), stderr: NOT_INSTALLED.into() },
        Err(e) => return TeaOut { status: -1, stdout: String::new(), stderr: e.to_string() },
    };
    match crate::herdr::cli::wait_with_timeout(child, timeout_ms) {
        Ok(r) => TeaOut { status: r.status, stdout: r.stdout, stderr: r.stderr },
        Err(e) => TeaOut { status: -1, stdout: String::new(), stderr: if e.code == "timeout" { format!("tea did not answer within {}s", timeout_ms / 1000) } else { e.message } },
    }
}

/// One of tea's logins: its name, and the host it reaches (by its address, and by its SSH host).
#[derive(Clone, Debug)]
pub struct Login { pub name: String, pub url: String, pub ssh_host: String }
impl Login {
    pub fn serves(&self, host: &str) -> bool {
        let of_url = self.url.split_once("://").map(|(_, r)| r).unwrap_or(&self.url).split(['/', ':']).next().unwrap_or("");
        [of_url, self.ssh_host.as_str()].iter().any(|h| !h.is_empty() && h.eq_ignore_ascii_case(host))
    }
}
/// tea's logins, asked again after five minutes. Err: why tea cannot be asked (not installed).
static LOGINS: Mutex<Option<(i64, Result<Vec<Login>, String>)>> = Mutex::new(None);
const KEEP_MS: i64 = 5 * 60000;
pub fn logins() -> Result<Vec<Login>, String> {
    let fake = faked().is_some();
    if !fake { if let Some((at, l)) = LOGINS.lock().ok().and_then(|g| g.clone()) { if now() - at < KEEP_MS { return l; } } }
    let r = tea(&["login", "list", "--output", "json"], &std::env::temp_dir(), 20000);
    let got = if r.status == 127 { Err(NOT_INSTALLED.to_string()) } else if r.status != 0 {
        Err(format!("tea login list failed: {}", [r.stderr.trim(), r.stdout.trim()].into_iter().find(|s| !s.is_empty()).unwrap_or("no output")))
    } else {
        // No login at all: tea prints no list.
        let v: Value = serde_json::from_str(r.stdout.trim()).unwrap_or(json!([]));
        Ok(v.as_array().map(|a| a.iter().map(|l| Login { name: l.t("name"), url: l.t("url"), ssh_host: l.t("ssh_host") }).filter(|l| !l.name.is_empty()).collect()).unwrap_or_default())
    };
    if !fake { if let Ok(mut g) = LOGINS.lock() { *g = Some((now(), got.clone())); } }
    got
}

/// What Gitea's API answered: its HTTP status and its body (`null` when it had none).
pub struct Answer { pub code: u16, pub body: Value }
impl Answer { pub fn ok(&self) -> bool { (200..300).contains(&self.code) } }
/// One call of Gitea's API as the repo's login: `path` without `/api/v1/`. Err: tea could not ask at all.
pub fn api(login: &str, method: &str, path: &str, body: Option<&Value>, cwd: &Path, timeout_ms: u64) -> Result<Answer, String> {
    let data = body.map(|b| b.to_string());
    let mut a = vec!["api", "--login", login, "-i"];
    if method != "GET" { a.extend(["-X", method]); }
    if let Some(d) = &data { a.extend(["-d", d.as_str()]); }
    a.push(path);
    let r = tea(&a, cwd, timeout_ms);
    // `-i` writes the status line to stderr: `HTTP/1.1 404 Not Found`. tea itself exits 0 whatever it was.
    let code = crate::re!(r"HTTP/[0-9.]+ (\d{3})").captures(&r.stderr).and_then(|c| c[1].parse::<u16>().ok());
    let Some(code) = code.or(if r.status == 0 { Some(200) } else { None }) else {
        return Err([r.stderr.trim(), r.stdout.trim()].into_iter().find(|s| !s.is_empty()).unwrap_or("tea gave no answer").to_string());
    };
    let body = if r.stdout.trim().is_empty() { Value::Null } else { serde_json::from_str(r.stdout.trim()).unwrap_or_else(|_| json!(r.stdout.trim())) };
    Ok(Answer { code, body })
}
/// Gitea's own words for an answer that was not a success.
pub fn said(a: &Answer) -> String {
    let m = if a.body.b("message") { a.body.t("message") } else if a.body.is_string() { text(&a.body) } else { String::new() };
    let login = if a.code == 401 { " (tea is not logged in to this server any more: tea login add)" } else { "" };
    format!("Gitea answered HTTP {}{}{login}", a.code, if m.is_empty() { String::new() } else { format!(": {m}") })
}
