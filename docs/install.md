# Installing, the first run, and taking it out again

Linux or macOS, on x86_64 or aarch64. Needs herdr 0.9.3 or newer and `git`; `curl` or `wget` to download the
program; optionally `gh` (pull requests on GitHub) or `tea` (pull requests on Gitea: [Gitea](gitea.md)) and `tailscale` (private review links). No Node, no npm: the board is one
program.

```sh
herdr plugin install eliasstravik/herdr-projects
```

herdr clones the repo's main branch into its own plugins folder (`herdr plugin list` shows where; `--ref v1.0.1`
takes a given release) and runs the plugin's build step there, which downloads the program. Or, to keep a checkout
of your own:

```sh
git clone https://github.com/eliasstravik/herdr-projects && cd herdr-projects
herdr plugin link "$PWD"          # hooks, the board's tab and the actions
sh bin/run.sh install             # gets the program, writes its launcher, says how to put it on your PATH
```

**The program is one file per platform** (`herdr-projects-linux-x86_64`, `-linux-aarch64`, `-macos-x86_64`,
`-macos-aarch64`; the Linux ones run on any Linux). Every command herdr runs for the plugin goes through
`bin/run.sh`, which runs, the first it finds:

1. a program built in this checkout (`target/release/herdr-projects`) that no source file is newer than;
2. the downloaded one in `bin/`, when it is this checkout's version (`Cargo.toml`) or newer;
3. that file downloaded now from the project's GitHub release of this checkout's version, its checksum checked, put
   in place in one step, one download at a time;
4. failing all that, an older build or an older download, with a line saying so. A download that failed is tried
   again ten minutes later, not on every command (each of herdr's hooks would wait for it).

With none it says what to do and stops at once, so herdr is never held up. The plugin's build step
(`sh bin/run.sh --fetch`) only gets the program, and builds it from source when it cannot download it.
`HERDR_PROJECTS_RELEASES` names another place for the releases (a fork, a mirror, a folder as `file:///…`). The
release files are built for every version tag by `.github/workflows/release.yml`.

**Updating.** `herdr-projects update` puts the newest release in place of the downloaded program, in one step
(`update 0.3.1` a given version); an open board and the keeper see the file change and load it by themselves
([the board's size](board-size.md)). Pulling a newer version of the plugin (`git pull`) does the same at the next
command herdr runs: `bin/run.sh` downloads that version. A program you built yourself is updated with `git pull` and
`cargo build --release`, never by a download. The keeper starts its successor on the new file itself once the file has stopped changing
(a build still being linked is waited for), with no board open and no agent running, and goes only when the successor
runs; a new file that does not start leaves the old keeper running, and it tries again a little later (`keeper.log` in
the data folder says so).

**Building from source** instead: `cargo build --release` in the checkout (Rust: https://rustup.rs). `bin/run.sh`
then runs that build, and builds again are picked up the same way.

**The command on your PATH.** Agents and hooks are handed the launcher `~/.herdr-projects/data/bin/herdr-projects`,
which runs the program. The board writes it when it opens (with `herdr plugin install`, open it first with the
plugin's action "Open the work board"); `herdr-projects install` writes it again and says whether something else named
`herdr-projects` comes first on your PATH. To type `herdr-projects` in a shell, put that folder on your PATH (in
`~/.profile` or your shell's start-up file).

**From herdr-superwork or the old herdr-projects 0.2**: the first run takes them over by itself (their data moved or
set aside, their extras in herdr's config, the agents' settings and skills folders made right, their processes
replaced) and says so; `herdr-projects doctor` looks again and says where things stand ([older installs](old-installs.md)).

**From the older Node version**: `npm unlink -g herdr-projects` takes the Node program off your PATH. The first run
of this version keeps a copy of the data folder as it was (`~/.herdr-projects/data-0.2.0`) before it changes
anything; the data's form is the same, so nothing is converted.

On a machine that another one will ask for its tasks ([other machines](other-machines.md)), `herdr-projects` must
be found by a login shell that nobody types into, which is what SSH starts: a PATH set only in `~/.bashrc` is often
not enough. `herdr-projects machines` checks it on the machine itself and says how to fix it; from the other
machine the test is `ssh <machine> 'exec $SHELL -lc "herdr-projects remote list"' | tail -c 200`, which ends with a
line of JSON.

Typing `herdr-projects` in any pane opens the board. The first time it opens inside herdr it shows one screen with
two things it can add to your herdr `config.toml`. Each is your choice, answered with single keys: `1` and `2` switch
them on or off, `k` picks another key, `enter` saves, `esc` asks again next time. The file is backed up first, each
block sits between two marker lines, and a change herdr rejects is rolled back:

- a key that opens the board: it offers `prefix+a`, which herdr does not use itself. If your config already binds
  `prefix+a`, it offers the next free key (`prefix+y`, then `prefix+u`, `prefix+i`, `prefix+m`) and says why. A key
  that herdr binds by default, or that your config already binds, is never taken: the screen refuses it and says what
  the key does today (`prefix+w`, for one, is herdr's own workspace picker), for the offered key and for one you type
  with `k` alike, and however your config writes a binding (one key, a list of keys, the older `[keys.indexed]`). A key that is already set up for the board is left as it is;
- sidebar rows, so that an agent's entry in herdr's sidebar is a small copy of its task's row on the board
  ([herdr's sidebar](sidebar.md)): the task's number and title; who is on it and where it stands, with the review
  round and the pull request (`agent · working · R#1 PR#113`); and the row's percent and time to go (`8% 16m`).
  Without the rows herdr shows the first two, with the tab's name (`Agent`) after the title, and not the
  third: its default sidebar shows no custom value. The rows an older version of the plugin wrote are brought up to
  date by themselves, between the same marker lines; rows of your own are never touched.

Nothing else is needed: no herdr integration, no hooks in your own agent settings. To add the blocks by hand instead:

```toml
[[keys.command]]
key = "prefix+a"
type = "plugin_action"
command = "herdr-projects.open"

[ui.sidebar.agents]
rows = [["state_icon", "machine", "workspace", { token = "tab", rules = [{ equals = "Agent", hide = true }, { starts_with = "Reviewer #", hide = true }] }], ["agent"], [{ token = "$summary", dim = true }, "$progress"]]

[ui.sidebar.spaces]
rows = [["state_icon", "workspace"], ["branch", "git_status"], [{ token = "$summary", dim = true }, "$progress"]]
```

A `[[keys.command]]` wins over herdr's own binding of the same key, without a warning: written by hand on a key
herdr uses, it switches herdr's action off (an early version of this page showed `prefix+w` here, which took away
herdr's workspace picker). `herdr --default-config` lists most of the keys herdr binds (not copy mode, `prefix+[`,
nor swapping panes, `prefix+shift+h` `j` `k` `l`); to move the board, change the `key`
line and run `herdr server reload-config`.

The key opens the board in a tab of its own. To have it open the board as a popup over herdr's window, and
close it again: `herdr-projects set board.open popup` ([the board](board.md)). The key itself stays as it is.

Optionally bind `herdr-projects.approve-current` too: for the task that owns the focused pane it opens the board on
that task with the merge question up (`y` = yes, `n` or `esc` = no), and does nothing anywhere else.

**Before unlinking the plugin** run `herdr-projects remove-extras`: herdr gives a plugin no uninstall hook, so this
is what takes the marked blocks out of `config.toml` and removes the how-to files (below). With the herdr web UI
plugin installed it also points that web UI's shared link back at the web UI and stops the board's front door
([the board in the herdr web UI](web-ui.md)).

Everything the board keeps for you is in one folder, `~/.herdr-projects/`: [the home folder](home-folder.md).
Updating from a version that kept its files in `~/.local/state`, `~/.config` and `~/.local/share` needs nothing from
you: the first run moves them there and says so in one line.

Your running herdr server must be 0.9.3 or newer: an older server refuses the plugin until it is restarted.
