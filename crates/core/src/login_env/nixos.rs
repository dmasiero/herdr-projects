//! The fork's NixOS additions to login_env.rs: what a login shell on NixOS needs from a process herdr started without
//! it. Its profile names the SSH agent from `XDG_RUNTIME_DIR` (`programs.ssh.startAgent`) and makes its PATH from the
//! account's name (`/etc/profiles/per-user/$USER/bin`, where cargo and the user's own programs are). Without them the
//! login shell gives no agent and a PATH without the user's programs. Harmless elsewhere: only what is missing is set.
use crate::util::env_set;
use std::process::Command;

/// The login shell gets systemd's runtime folder for this user (`/run/user/<uid>`) when this process has none.
pub fn prepare(c: &mut Command) {
    let run = format!("/run/user/{}", unsafe { libc::getuid() });
    if !env_set("XDG_RUNTIME_DIR") && std::path::Path::new(&run).is_dir() { c.env("XDG_RUNTIME_DIR", run); }
}

/// USER and LOGNAME from the account itself, when this process has none: the login shell and all the board starts
/// get them.
pub fn account() {
    let name = unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() { return; }
        std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned()
    };
    if name.is_empty() { return; }
    for k in ["USER", "LOGNAME"] { if !env_set(k) { unsafe { std::env::set_var(k, &name) } } }
}
