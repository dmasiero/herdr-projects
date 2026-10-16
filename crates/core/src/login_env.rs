//! What a login shell has that herdr's server may not pass on to the board's processes: the user's PATH, the account's
//! name (USER, which a login shell on NixOS makes its PATH from), gh's login (`GH_TOKEN`, `GH_HOST`, …) and the SSH
//! agent (`SSH_AUTH_SOCK`). herdr starts plugin commands, the keeper among them, with its server's environment; started
//! at boot that one may have none of them, and then every gh call of the keeper's jobs fails ("gh auth login"): a merge
//! waiting for GitHub's checks would never be looked at again. Without the agent every git fetch and push over SSH
//! fails ("Permission denied (publickey)"). Used by runner.rs (`user_env`).
use crate::clock::now;
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::util::env_set;
use std::process::{Command, Stdio};

/// What gh reads its login and its host from.
pub const GH_VARS: [&str; 6] = ["GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN", "GH_HOST", "GH_CONFIG_DIR"];
/// Where ssh and git find the SSH agent.
pub const SSH_VARS: [&str; 2] = ["SSH_AUTH_SOCK", "SSH_AGENT_PID"];
/// gh's variables a login shell's `env` sets, for a process that has none of them: one that has any keeps its own
/// login (an agent's pane, a shell the human started it from).
pub fn gh_login(env_out: &str, has: impl Fn(&str) -> bool) -> Vec<(String, String)> { taken(&GH_VARS, env_out, has) }
/// The SSH agent a login shell's `env` names, by the same rule: a process with an agent of its own keeps it.
pub fn ssh_agent(env_out: &str, has: impl Fn(&str) -> bool) -> Vec<(String, String)> { taken(&SSH_VARS, env_out, has) }
fn taken(vars: &[&str], env_out: &str, has: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    if vars.iter().any(|k| has(k)) { return vec![]; }
    env_out.lines().filter_map(|l| l.split_once('=')).filter(|(k, v)| vars.contains(k) && !v.is_empty()).map(|(k, v)| (k.to_string(), v.to_string())).collect()
}
/// The PATH a login shell's `env` gives, empty if none.
pub fn path_of(env_out: &str) -> String { env_out.lines().find_map(|l| l.strip_prefix("PATH=")).unwrap_or("").trim().to_string() }
/// A login shell's whole environment (`env`, which any shell runs): what it prints before it (a greeting) is ignored.
/// A shell's profile names the agent from XDG_RUNTIME_DIR (NixOS's `programs.ssh.startAgent`, other systemd setups):
/// a process started without it gives the shell systemd's own folder for this user, where there is one.
fn login_shell() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let mut c = Command::new(shell);
    let run = format!("/run/user/{}", unsafe { libc::getuid() });
    if !env_set("XDG_RUNTIME_DIR") && std::path::Path::new(&run).is_dir() { c.env("XDG_RUNTIME_DIR", run); }
    let out = c.args(["-lc", "env"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A login shell is asked once and its PATH remembered for a while (`env.json`); the PATH of this process takes it in.
/// gh's login and the SSH agent are never written down: where the login shell has one (the names are remembered) and
/// this process has none, the shell is asked for it, and this process and all it starts take it in.
pub fn user_env() {
    if env_set("HERDR_PROJECTS_ENV_OK") || !env_set("HERDR_PLUGIN_ID") { return; }
    let file = data_dir().join("env.json");
    let mut cached = read_obj(&file);
    let stale = cached.as_ref().map(|c| now() - c.i("at") > 600000 || !c.b("PATH") || !c.has("gh") || !c.has("ssh")).unwrap_or(true);
    let wants = |vars: &[&str], key: &str| !vars.iter().any(|k| env_set(k)) && cached.as_ref().map(|c| !strings(c.at(key)).is_empty()).unwrap_or(false);
    let shell = if stale || wants(&GH_VARS, "gh") || wants(&SSH_VARS, "ssh") { login_shell() } else { None };
    if stale {
        let out = shell.clone().unwrap_or_default();
        let path = Some(path_of(&out)).filter(|p| !p.is_empty()).unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
        let names: Vec<String> = gh_login(&out, |_| false).into_iter().map(|(k, _)| k).collect();
        let ssh: Vec<String> = ssh_agent(&out, |_| false).into_iter().map(|(k, _)| k).collect();
        let c = obj(json!({ "at": now(), "PATH": path, "gh": names, "ssh": ssh }));
        let _ = write_json(&file, &Value::Object(c.clone()));
        cached = Some(c);
    }
    let mut seen = std::collections::HashSet::new();
    let mut all = vec![];
    for p in cached.map(|c| c.t("PATH")).unwrap_or_default().split(':').chain(std::env::var("PATH").unwrap_or_default().split(':').map(|s| s.to_owned()).collect::<Vec<_>>().iter().map(|s| s.as_str())) {
        if !p.is_empty() && seen.insert(p.to_string()) { all.push(p.to_string()); }
    }
    let login = shell.map(|out| [gh_login(&out, env_set), ssh_agent(&out, env_set)].concat()).unwrap_or_default();
    // A login `sh` on NixOS makes its PATH again from the account's name (`/etc/profiles/per-user/$USER/bin`, where the
    // user's own programs are, cargo among them): a process started without USER would find none of them.
    let account = unsafe { let pw = libc::getpwuid(libc::getuid()); if pw.is_null() { String::new() } else { std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned() } };
    unsafe {
        if !account.is_empty() { for k in ["USER", "LOGNAME"] { if !env_set(k) { std::env::set_var(k, &account); } } }
        std::env::set_var("PATH", all.join(":"));
        for (k, v) in login { std::env::set_var(k, v); }
        std::env::set_var("HERDR_PROJECTS_ENV_OK", "1");
    }
}
