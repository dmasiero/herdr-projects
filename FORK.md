# This fork

This repo is a fork of [eliasstravik/herdr-projects](https://github.com/eliasstravik/herdr-projects) ("upstream"). It
follows upstream and adds a small layer that upstream chose not to take:

- **Gitea pull requests through `tea`** ([Gitea](docs/gitea.md)): a repo whose remote is on a Gitea server gets its
  pull requests there, behind the existing forge choice `merge.forge` (`auto`, `gitea`, `github`). Upstream closed
  this as #189: no Gitea support for now.
- **NixOS** ([NixOS and Nix](docs/nixos.md)): a flake (package and dev shell), the real-program tests passing on
  NixOS, linking with a `mold` on the PATH, and the login-shell details a NixOS profile needs. Upstream took its own
  SSH-agent fix in 1.2.2 and closed the rest as #188.
- **A pull request merged already** ([follow-up](docs/follow-up.md)): newer work on a merged branch goes in a
  follow-up pull request instead of failing.

Everything else is upstream's, unchanged. The fork is maintained privately for now; the end of this page lists what
to do before it goes public.

## Branches

- `upstream/main`: upstream as it is. Never changed here, never pushed to.
- `main`: upstream plus the fork's layer. `git diff upstream/main main` shows the whole layer at any time.
- `sync/upstream-<version>`: one sync, made by `scripts/sync-upstream.sh`, merged into `main` by a pull request.
- Feature work goes on its own branch into `main`, as on upstream.

**Upstream comes in by merge, never by rebase or squash.** `main` is published, so rebasing it onto upstream would
rewrite it on every sync and force-push it. A merge keeps upstream's commits as they are, and each sync is one merge
commit you can read: `git log --first-parent main` lists the syncs and the fork's own changes, one line each.

A sync pull request must be merged with **Create a merge commit**. Squashing it loses the link to upstream's commits:
git then sees them as never merged, and the next sync conflicts on every line they touched. That happened once (the
1.2.1 sync, #3); the 1.2.2 sync was merged against the upstream commit that squash matched, to repair it. The fork's
own feature pull requests can be squashed as you like.

The board squashes by default (`merge.method squash`). A sync that the board merges, as a task or an automation, needs
`merge.method merge`: `herdr-projects set merge.method merge --task <id>` (or `--automation <name>`) before you approve.

**Delete a branch once it is merged or superseded**, so `origin` holds only `main` and open work. The cleanup with the
1.2.2 sync (2026-10-17) deleted these from `origin`:

| Branch | Why |
| --- | --- |
| `legacy` | Merged into `main`. |
| `work/1-support-gitea-pull-requests-vi` | Merged into `main` (Gitea pull requests). |
| `work/10-release-herdr-projects-1-0-1` | Merged into `main`. |
| `work/5-6-9-release-herdr-projects-1-0` | Merged into `main`. |
| `gitea-tea-support` | Superseded by `main`; its one test `main` lacked moved to `tests/gitea.rs`. |
| `nixos-support` | Superseded by `main` (the NixOS layer, `crates/core/src/login_env/nixos.rs`, the flake). |
| `hp/herdr-projects/t-0058-agent-driven-rename` | The same commit as upstream's branch of that name: upstream's work. |
| `work/2-1-9-last-rust-release-0-2-35-w` | The same commit as upstream's branch of that name: upstream's work. |
| `work/7-board-misses-an-agent-finishin` | The same commit as upstream's branch of that name: upstream's work. |

## Syncing with upstream

```sh
sh scripts/sync-upstream.sh            # fetch, merge, check, push sync/upstream-<version>, open the pull request
sh scripts/sync-upstream.sh --no-pr    # the same, but push and open nothing
```

The script never touches the checkout it runs from: it makes the sync branch from `origin/main` in a worktree of its
own (in `$TMPDIR`, or `--dir`). It then:

1. fetches `origin` and `upstream` (adding the `upstream` remote when it is missing), and stops when `main` already has
   upstream's main (`--ref v1.2.3` syncs to a release instead);
2. merges upstream into `sync/upstream-<version>` (with the date added when that name is taken);
3. starts the fork's own number again on a new upstream version (`FORK_VERSION`: `masiero.4` becomes `masiero.1`);
4. runs the whole check, `npm run check -- --all`: every test, upstream's and the fork's, with GitHub (`gh` faked) and
   Gitea (`tea` faked), on this machine's system: NixOS's paths on NixOS, plain Linux's elsewhere;
5. where Nix is installed, runs `nix flake check` and `nix build`, and, where KVM is too, the whole check on NixOS in a
   VM (`nix build .#nixos-check`, `nix/nixos-check.nix`). So a NixOS machine with KVM covers NixOS twice and plain
   Linux not at all, and a plain Linux machine with Nix and KVM covers both; the pull request text says which ran;
6. pushes the branch to `origin` and opens a pull request to the fork's `main` with upstream's commits, the conflicts,
   what was checked and where, and the size of the layer after it. Its CI then runs both systems whatever machine
   ran the script: the whole check on Ubuntu, and on NixOS in a VM (below). Merge it only when both pass.

It refuses to push when `origin` and `upstream` are the same repo. Exit codes, for an agent: 0 done or nothing new, 1
an error, 3 conflicts to resolve, 4 the check failed.

**Conflicts.** The script stops with exit 3 and lists the files. In its worktree, take upstream's version of each
file and put the fork's change back on top of it: the table below says what the fork changes in each upstream file
and why. Where upstream now does what the fork did, keep upstream's and drop the fork's (say so in the pull request).
Then `sh scripts/sync-upstream.sh --continue --dir <that worktree>` commits, checks and opens the pull request. A
conflict in a made table (`docs/keys.md`, `docs/settings.md`) is settled by running the check, never by picking lines.

**How often.** Upstream releases often (four releases in three days around 1.2.x). Sync on each upstream release, at
most once a day: run by hand when you see a release, or make it a daily board automation that runs the script; it
exits at once when there is nothing new, and its pull request still waits for your review (merged with a merge
commit, above). Syncing often keeps each merge small.

## The layer

Kept small so syncs rarely conflict: fork-only files wherever possible, and a few lines in upstream's files where
there is no other way in. `git diff --stat upstream/main main` lists them as they are now.

**Fork-only files** (upstream has none of them, so they never conflict): `FORK.md`, `FORK_VERSION`,
`flake.nix`, `flake.lock`, `nix/nixos-check.nix`, `scripts/cargo.sh`, `scripts/sync-upstream.sh`, `.github/workflows/fork.yml`,
`crates/core/src/forge.rs` and `crates/core/src/forge/` (`gitea.rs`, `gitea_checks.rs`, `gitea_view.rs`, `tea.rs`),
`crates/core/src/login_env/nixos.rs`, `crates/flow/src/finish/follow_up.rs`, `crates/flow/src/land/merged.rs`,
`docs/gitea.md`, `docs/nixos.md`, `docs/follow-up.md`, and the tests `tests/fork.rs`, `tests/gitea.rs`,
`tests/gitea_checks.rs`, `tests/gitea_forge.rs`, `tests/nixos_login_env.rs`, `tests/pull_requests_follow_up.rs`,
`tests/common/tea.rs`.

**Upstream files the fork changes:**

| File | What the fork changes, and why |
|---|---|
| `crates/core/src/lib.rs` | `pub mod forge;`: the Gitea code's module. |
| `crates/core/src/store/settings_list.rs`, `docs/settings.md` | The `merge.forge` setting (the made table follows). |
| `crates/board/src/board/view/panel/settings.rs` | The task panel names the forge (Gitea's checks, not GitHub's). |
| `crates/flow/src/flow/git.rs` | Every `gh` call goes to `tea` for a repo on Gitea; a hint when gh finds no GitHub host. |
| `crates/flow/src/finish/pipeline.rs` | Errors name tea and Gitea's page where the repo is on Gitea; a merged PR's newer work opens a follow-up PR. |
| `crates/flow/src/finish/check_again.rs` | On Gitea, a branch's workflows decide whether checks will come. |
| `crates/flow/src/finish.rs`, `crates/flow/src/land.rs` | `pub mod follow_up;` and `pub mod merged;`. |
| `crates/flow/src/land/checks.rs` | Gitea's commit statuses as the PR's checks; "Waiting for Gitea checks". |
| `crates/flow/src/land/merge.rs` | A PR merged already counts as merged; tea's words in errors. |
| `crates/flow/src/housekeeping/prs.rs` | The PR watch says Gitea and tea's commands for a Gitea PR. |
| `crates/core/src/login_env.rs` | Two hook lines into `login_env/nixos.rs`: `XDG_RUNTIME_DIR` for the login shell (NixOS names the SSH agent from it; upstream's fix finds no socket without it) and `USER`/`LOGNAME` (NixOS builds the PATH from it). |
| `crates/core/src/util.rs` | `bare_path`: a bare PATH that includes NixOS's `/run/current-system/sw/bin` and `/run/wrappers/bin`. |
| `crates/flow/src/remote/check.rs` | Checks a machine with that bare PATH, as NixOS's sshd gives it. |
| `crates/core/src/store/files.rs` | `VERSION` adds `FORK_VERSION` (`1.2.2+masiero.1`). |
| `bin/run.sh` | Looks in Nix's folders last; a fork checkout is built from source and never downloads upstream's program (which lacks the layer). |
| `scripts/check.sh`, `scripts/setup.sh` | Find cargo through `scripts/cargo.sh`, and say how to get Rust where it is missing (`nix develop`). |
| `scripts/linker.sh`, `docs/linking.md` | A `mold` on the PATH where the pinned one cannot be installed (NixOS). |
| `docs/install.md`, `docs/how-work-gets-done.md` | One line each linking the fork's pages. |
| `tests/main.rs` | The fork's test modules. |
| `tests/common/mod.rs`, `tests/common/gh.rs` | The fake `tea` in every test world; the fake gh knows a PR merged already. |
| `tests/check_script.rs`, `tests/install.rs`, `tests/linking.rs`, `tests/take_over_more.rs` | Real-program tests that pass on NixOS (`bare_path`, coreutils as one program, mold on the PATH). |
| `README.md` | One line linking this page. |

Upstream may take some of this one day (the follow-up PR, `bare_path`, the NixOS test fixes are not Nix-only in
spirit): each one taken makes the layer smaller.

## CI

Upstream's `.github/workflows/ci.yml` runs on the fork as it is: the whole check on Linux, which includes the fork's
tests, and the macOS sandbox test. `.github/workflows/fork.yml` adds:

- **layer**: the fork's own tests by name (Gitea with `tea` faked, follow-up PRs, version, NixOS login shell), so a
  failure in the layer shows as its own red job;
- **nix**: `nix flake check`, `nix build` (and the built program's version), and the whole check inside
  `nix develop`, with Nix's Rust and mold. This is still Ubuntu: Nix's tools, not a NixOS system;
- **nixos**: the whole check on a real NixOS, in a VM (`nix build .#nixos-check`, `nix/nixos-check.nix`): `/etc/NIXOS`,
  next to nothing in `/usr/bin` and `/bin`, programs in `/run/current-system/sw/bin`, coreutils as one program, the
  SSH agent from `programs.ssh.startAgent`. The NixOS paths of `scripts/linker.sh`, `bin/run.sh`, `bare_path` and the
  real-program tests run here. It needs KVM, which GitHub's Linux runners have.

Together with upstream's Check job (the whole check on Ubuntu), every change is checked on plain Linux and on NixOS.

GitHub turns Actions off on a new fork: turn them on once under the fork's Actions tab. `release.yml` is upstream's and
stays unused: the fork pushes no `v*` tags.

## Version

The fork's version is upstream's with the fork's own part after a `+`: **`1.2.2+masiero.1`** is the first fork build on
upstream 1.2.2. `FORK_VERSION` holds the part after the `+`; raise its number when the layer changes between upstream
releases, and the sync script starts it at 1 again on a new upstream version.

The program says it everywhere it says its version: `herdr-projects version`, `doctor`, what other machines are told,
the keeper, and the Nix package (`herdr-projects-1.2.2+masiero.1`). Upstream's own version lines (`Cargo.toml`,
`herdr-plugin.toml`, `package.json`) stay as upstream states them, so a sync never conflicts on them; that is why
herdr's plugin list shows the plain upstream version.

## Installing and updating from the fork

The fork has no releases of its own: it is built from source. `bin/run.sh` in a checkout with `FORK_VERSION` runs the
build in that checkout and never downloads upstream's program (it would run without Gitea support). `herdr-projects
update` refuses a build from source, and `herdr plugin update` only updates plugins installed from GitHub, so neither
replaces it.

The usual setup: a clone of the fork registered as a local plugin.

```sh
git clone git@github.com:dmasiero/herdr-projects.git <folder>
cd <folder> && cargo build --release --locked   # in `nix develop` on NixOS
herdr plugin link <folder>                       # herdr runs bin/run.sh of this folder
sh bin/run.sh install                            # the herdr-projects launcher, pointing at this build
```

**Update** (after a sync or feature PR is merged into `main`):

```sh
cd <folder>
git rev-parse HEAD > ../herdr-projects-fork.previous   # to roll back to
cp -a ~/.herdr-projects/data ~/.herdr-projects/data.before-update   # a newer version may change the data
git fetch origin && git merge --ff-only origin/main
cargo build --release --locked
./target/release/herdr-projects version   # 1.2.2+masiero.1 or newer
./target/release/herdr-projects doctor
```

The keeper and an open board load the new program by themselves when its file changes. If `cargo build` fails, the
old program keeps running (`bin/run.sh` runs the older build and says so), so a failed build breaks nothing.

**Roll back**:

```sh
cd <folder>
git checkout --detach "$(cat ../herdr-projects-fork.previous)"
cargo build --release --locked
```

Copy `data.before-update` back over `~/.herdr-projects/data` only if the older program cannot read the data (stop
herdr's server first). Return with `git checkout main`.

With Nix only the command can be installed (`nix profile install github:dmasiero/herdr-projects`); the plugin itself
still needs the checkout above ([NixOS and Nix](docs/nixos.md)).

## Before going public

Not done yet, on purpose: the fork is private use for now.

- [ ] **Name and branding** (open, Doug's decision): keep "herdr-projects" or rename. The plugin id `herdr-projects`
      is upstream's, so the two cannot be installed side by side; the version's `masiero` part (`FORK_VERSION`), the
      flake's homepage and the clone URLs in `docs/nixos.md` follow from the choice.
- [ ] **No internal names**: no internal host names, IPs, user names or secrets in fork-only files. Check with
      `git grep -nIiE 'masiero|\.internal|\.lan\b|/home/|192\.168\.|10\.[0-9]+\.[0-9]+\.' $(git diff --name-only upstream/main main)`
      and `git log -p upstream/main..main | grep -iE '\.internal|/home/'`. Examples use `gitea.example.org`. The
      history already holds one internal Gitea host name in a test (commits `f0ff4d4` and `563b942`, fixed since); the
      fork is a public GitHub repo already, so it is public now. Decide whether to accept that or rewrite those
      commits before announcing (a rewrite changes every later commit id).
- [ ] **License and attribution**: upstream is MIT (`LICENSE`, Copyright 2026 Elias Stravik), which allows a fork,
      changes and redistribution as long as the copyright and permission notice stay. Keep `LICENSE` as it is; add a
      copyright line for the fork's own changes if wanted. Do not use upstream's name or logo in a way that suggests
      upstream endorses the fork.
- [ ] **README**: say at the top what the fork is for and how it differs (this page's first section), and replace
      upstream's install line (`herdr plugin install eliasstravik/herdr-projects`) for the fork's users, or point them
      to the fork's install steps here. Upstream's pages (`docs/install.md`, `docs/get-started.md`) keep upstream's
      URLs; changing them adds conflicts on each sync, so prefer explaining it in README and here.
- [ ] **Releases** (optional): to ship binaries, point `REPO` in `crates/flow/src/install.rs` and the release URLs in
      `bin/run.sh` at the fork, tag versions as `v1.2.2+masiero.1` (`release.yml` checks the tag against Cargo.toml,
      so the fork's suffix has to be in its version there too), and drop the build-from-source rule in `bin/run.sh`.
      Without releases, users build from source as above.
- [ ] **GitHub settings**: Actions on, Issues on, "Allow merge commits" on (sync PRs need it), a default branch
      protection on `main` that requires the Check and Fork workflows.
- [ ] **Stale branches**: only `main` and open work on `origin` (see Branches).
