//! What the board's processes take from a login shell (`sw::login_env`) besides the PATH and gh's login: the SSH agent,
//! which git needs to fetch and push over SSH, taken only by a process that has none of its own, and asked for again.
use crate::common;
use sw::login_env::ssh_agent;

#[test]
fn a_keeper_started_without_an_ssh_agent_takes_the_login_shells_and_a_process_with_its_own_keeps_it() {
    let out = "PATH=/a/bin\nSSH_AUTH_SOCK=/run/user/1000/ssh-agent\nSSH_AGENT_PID=\nHOME=/home/me\n";
    assert_eq!(ssh_agent(out, |_| false), vec![("SSH_AUTH_SOCK".to_string(), "/run/user/1000/ssh-agent".to_string())]);
    assert!(ssh_agent(out, |k| k == "SSH_AUTH_SOCK").is_empty(), "an agent of its own is kept");
    assert!(ssh_agent("PATH=/x\n", |_| false).is_empty(), "a login shell with no agent gives none");
}

#[test]
fn user_env_takes_the_agent_a_profile_names_from_xdg_runtime_dir_and_asks_again_only_when_it_has_none() {
    let w = common::world();
    // A login shell whose profile names the agent from XDG_RUNTIME_DIR, as NixOS's programs.ssh.startAgent does.
    let shell = w.base.join("login-sh");
    std::fs::write(&shell, "#!/bin/sh\necho \"PATH=$PATH\"\n[ -z \"$XDG_RUNTIME_DIR\" ] || echo \"SSH_AUTH_SOCK=$XDG_RUNTIME_DIR/ssh-agent\"\n").unwrap();
    std::fs::set_permissions(&shell, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let keep: Vec<(&str, Option<String>)> = ["SHELL", "PATH", "SSH_AUTH_SOCK", "SSH_AGENT_PID", "XDG_RUNTIME_DIR"].iter().map(|k| (*k, std::env::var(k).ok())).collect();
    let run = w.base.join("run");
    let start = |own: Option<&str>| unsafe {
        std::env::remove_var("HERDR_PROJECTS_ENV_OK");
        std::env::set_var("HERDR_PLUGIN_ID", "herdr-projects");
        std::env::set_var("SHELL", &shell);
        std::env::set_var("XDG_RUNTIME_DIR", &run);
        for k in ["SSH_AUTH_SOCK", "SSH_AGENT_PID"] { std::env::remove_var(k); }
        if let Some(s) = own { std::env::set_var("SSH_AUTH_SOCK", s); }
        sw::login_env::user_env();
        std::env::var("SSH_AUTH_SOCK").ok()
    };
    let agent = format!("{}/ssh-agent", run.display());
    let first = start(None);
    let own = start(Some("/own/agent"));
    // the names are remembered (env.json, fresh now): a process with none asks the login shell again
    let again = start(None);
    unsafe {
        std::env::remove_var("HERDR_PLUGIN_ID");
        for (k, v) in keep { match v { Some(v) => std::env::set_var(k, v), None => std::env::remove_var(k) } }
    }
    assert_eq!(first.as_deref(), Some(agent.as_str()));
    assert_eq!(own.as_deref(), Some("/own/agent"), "an agent of its own is kept");
    assert_eq!(again.as_deref(), Some(agent.as_str()));
}
