//! The fork's own layer (FORK.md): its version says which upstream it is built on (`1.2.2+masiero.1`), and bin/run.sh
//! runs a build of the fork's checkout, never upstream's downloaded program, which lacks the fork's changes.
use std::path::{Path, PathBuf};

/// NixOS's login shell: a keeper without `XDG_RUNTIME_DIR` still finds the SSH agent (login_env/nixos.rs).
mod nixos_login_env;
/// A pull request merged already: newer work goes in a follow-up pull request (finish/follow_up.rs).
mod pull_requests_follow_up;

fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")) }

#[test]
fn the_version_is_upstreams_and_the_forks_own_part() {
    let part = std::fs::read_to_string(root().join("FORK_VERSION")).unwrap();
    assert!(!part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '.'), "FORK_VERSION is one word of letters, digits and dots, with no newline: {part:?}");
    assert!(part.rsplit('.').next().is_some_and(|n| n.parse::<u32>().is_ok()), "it ends in the fork's own number: {part:?}");
    assert_eq!(sw::store::files::VERSION, format!("{}+{part}", env!("CARGO_PKG_VERSION")));
    assert_eq!(sw::install::version_said(&format!("herdr-projects {} (/x)", sw::store::files::VERSION)).as_deref(), Some(sw::store::files::VERSION));
    // Upstream's own version stays as upstream states it, so a sync never conflicts on it.
    let plugin = std::fs::read_to_string(root().join("herdr-plugin.toml")).unwrap();
    assert!(plugin.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))));
    assert!(!sw::util::newer(env!("CARGO_PKG_VERSION"), sw::store::files::VERSION), "the fork is never older than the upstream it is built on");
}

fn write(p: &Path, text: &str) { std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, text).unwrap(); }
fn exec(p: &Path, text: &str) { write(p, text); std::fs::set_permissions(p, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap(); }
fn run_sh(p: &Path, args: &[&str]) -> (i32, String, String) {
    let o = std::process::Command::new("sh").arg(p.join("bin/run.sh")).args(args).env_remove("HERDR_PROJECTS_RELEASES").env("PATH", sw::util::bare_path("/usr/bin:/bin")).output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}

#[test]
fn real_programs_run_sh_in_the_fork_runs_its_own_build_and_never_downloads() {
    let p = std::env::temp_dir().join(format!("sw-fork-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    write(&p.join("bin/run.sh"), &std::fs::read_to_string(root().join("bin/run.sh")).unwrap());
    write(&p.join("Cargo.toml"), "[package]\nname = \"herdr-projects\"\nversion = \"0.3.0\"\n");
    write(&p.join("src/main.rs"), "");
    write(&p.join("FORK_VERSION"), "fork.1");
    // Nothing built yet: it says how to build it and downloads nothing (no cargo on this PATH).
    let (code, out, err) = run_sh(&p, &["hook"]);
    assert_eq!((code, out.as_str()), (1, ""), "{err}");
    assert!(err.contains("built from source, never downloaded") && !err.contains("downloading"), "{err}");
    assert!(std::fs::read_dir(p.join("bin")).unwrap().count() == 1, "nothing was downloaded");
    // A build newer than its source runs; one older than its source still runs, and says so.
    exec(&p.join("target/release/herdr-projects"), "#!/bin/sh\necho \"built $*\"\n");
    assert_eq!(run_sh(&p, &["board"]), (0, "built board\n".into(), String::new()));
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(p.join("src/main.rs")).unwrap().set_modified(later).unwrap();
    let (code, out, err) = run_sh(&p, &["board"]);
    assert_eq!((code, out.as_str()), (0, "built board\n"), "{err}");
    assert!(err.contains("older than its source"), "{err}");
    let _ = std::fs::remove_dir_all(&p);
}
