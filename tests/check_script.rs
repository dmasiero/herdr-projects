//! [real programs] The check's own script (`scripts/check.sh`) as a fresh machine starts it: an empty home, with no
//! settings and no build cache, gets past its start on the default share of the cores, and a check that stops says
//! why on stderr, never with no word.
use std::path::{Path, PathBuf};

/// A copy of the check's scripts in a git repo of its own, an empty home, and a `cargo` that writes down what it was
/// given and stops the check (exit 7, without a word), so only the check's start runs.
struct Bench { base: PathBuf }
impl Bench {
    fn new(name: &str) -> Bench {
        let base = std::env::temp_dir().join(format!("sw-check-script-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let proj = base.join("proj");
        std::fs::create_dir_all(proj.join("scripts")).unwrap();
        std::fs::create_dir_all(base.join("home")).unwrap();
        std::fs::create_dir_all(base.join("bin")).unwrap();
        for f in ["check.sh", "run-tests.sh", "build-cache.sh", "share-cores.sh"] {
            std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts").join(f), proj.join("scripts").join(f)).unwrap();
        }
        let stub = |name: &str, body: &str| {
            let p = base.join("bin").join(name);
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&p, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        };
        stub("cargo", &format!("echo \"$CARGO_BUILD_JOBS $RUST_TEST_THREADS ${{RUSTC_WRAPPER:-none}}\" > '{}'\nexit 7", base.join("cargo-saw").display()));
        stub("rustc", "echo rustc 0.0.0");
        let b = Bench { base };
        b.git(&["init", "-q"]);
        b
    }
    fn git(&self, a: &[&str]) {
        let o = std::process::Command::new("git").args(a).current_dir(self.base.join("proj")).env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1").output().unwrap();
        assert!(o.status.success(), "git {a:?}: {}", String::from_utf8_lossy(&o.stderr));
    }
    fn home(&self) -> PathBuf { self.base.join("home") }
    /// `sh scripts/check.sh --all` with nothing but this home and a plain PATH: its exit code, stderr, and what the
    /// stand-in cargo was given (empty when the check never got to it).
    fn run(&self) -> (i32, String, String) {
        let o = std::process::Command::new("/bin/sh").args(["scripts/check.sh", "--all"]).current_dir(self.base.join("proj")).env_clear()
            .env("HOME", self.home()).env("PATH", sw::util::bare_path(&format!("{}:/usr/bin:/bin", self.base.join("bin").display()))).output().unwrap();
        let saw = std::fs::read_to_string(self.base.join("cargo-saw")).unwrap_or_default();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stderr).into_owned(), saw.trim().to_string())
    }
}
impl Drop for Bench { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.base); } }

fn cores() -> u64 {
    let o = std::process::Command::new("getconf").arg("_NPROCESSORS_ONLN").output().unwrap();
    String::from_utf8_lossy(&o.stdout).trim().parse().unwrap()
}

#[test]
fn real_programs_the_check_starts_in_an_empty_home_on_the_default_share_and_says_why_it_stopped() {
    let b = Bench::new("empty");
    let (code, err, saw) = b.run();
    // no settings: 4 agents at once; no build cache: cargo as it is
    let jobs = cores().div_ceil(4);
    assert_eq!(saw, format!("{jobs} {jobs} none"), "the check stopped before building: {err}");
    assert_eq!(code, 7);
    assert!(err.contains("The check stopped while building the program (exit 7)."), "{err}");
    assert!(std::fs::read_dir(b.home()).unwrap().next().is_none(), "the check wrote nothing to an empty home before it built");
}

#[test]
fn real_programs_a_settings_file_that_cannot_be_read_is_the_default_and_one_that_names_the_agents_is_used() {
    let b = Bench::new("settings");
    let settings = b.home().join(".herdr-projects/settings/config.json");
    std::fs::create_dir_all(&settings).unwrap(); // a folder where the file should be: awk cannot read it
    let (code, err, saw) = b.run();
    let jobs = cores().div_ceil(4);
    assert_eq!((code, saw), (7, format!("{jobs} {jobs} none")), "{err}");
    std::fs::remove_dir(&settings).unwrap();
    std::fs::write(&settings, "{\n  \"board\": {\n    \"agents\": 1000\n  }\n}\n").unwrap();
    let (code, err, saw) = b.run();
    assert_eq!((code, saw), (7, "1 1 none".to_string()), "{err}");
}

#[test]
fn real_programs_a_check_that_stops_before_it_starts_says_why_once() {
    let b = Bench::new("refused");
    let (code, err, saw) = {
        let o = std::process::Command::new("/bin/sh").args(["scripts/check.sh", "--nope"]).current_dir(b.base.join("proj")).env_clear()
            .env("HOME", b.home()).env("PATH", sw::util::bare_path("/usr/bin:/bin")).output().unwrap();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stderr).into_owned(), std::fs::read_to_string(b.base.join("cargo-saw")).unwrap_or_default())
    };
    assert_eq!((code, err.as_str(), saw.as_str()), (2, "check does not know --nope. It takes: --all, --fresh, --since <commit>, and names of tests.\n", ""));
}
