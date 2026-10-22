//! The fork's NixOS additions to the login shell's environment (`sw::login_env::nixos`): a keeper started without
//! `XDG_RUNTIME_DIR` still finds the SSH agent a NixOS profile names from it (`programs.ssh.startAgent`).
use crate::common;

#[test]
fn user_env_takes_the_agent_a_profile_names_from_xdg_runtime_dir_and_asks_again_only_when_it_has_none() {
    let w = common::world();
    // A login shell whose profile names the agent from XDG_RUNTIME_DIR, as NixOS's programs.ssh.startAgent does.
    let shell = w.base.join("login-sh");
    std::fs::write(&shell, "#!/bin/sh\necho \"PATH=$PATH\"\n[ -z \"$XDG_RUNTIME_DIR\" ] || echo \"SSH_AUTH_SOCK=$XDG_RUNTIME_DIR/ssh-agent\"\n").unwrap();
    std::fs::set_permissions(&shell, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let keep: Vec<(&str, Option<String>)> = ["SHELL", "PATH", "SSH_AUTH_SOCK", "SSH_AGENT_PID", "XDG_RUNTIME_DIR"].iter().map(|k| (*k, std::env::var(k).ok())).collect();
    let run = w.base.join("run");
    std::fs::create_dir_all(&run).unwrap();
    let _agent = std::os::unix::net::UnixListener::bind(run.join("ssh-agent")).unwrap();
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

#[test]
fn account_sets_user_and_logname_only_where_they_are_missing() {
    let keep: Vec<(&str, Option<String>)> = ["USER", "LOGNAME"].iter().map(|k| (*k, std::env::var(k).ok())).collect();
    let (user, logname) = unsafe {
        std::env::remove_var("USER");
        std::env::set_var("LOGNAME", "someone-else");
        sw::login_env::nixos::account();
        (std::env::var("USER").ok(), std::env::var("LOGNAME").ok())
    };
    unsafe { for (k, v) in keep { match v { Some(v) => std::env::set_var(k, v), None => std::env::remove_var(k) } } }
    assert!(user.is_some_and(|u| !u.is_empty()), "USER comes from the account");
    assert_eq!(logname.as_deref(), Some("someone-else"), "one that is set is kept");
}
