# NixOS and Nix

**The plugin works on NixOS as it is.** Install it the usual way ([installing](install.md)). The downloaded program is
built for musl and linked statically, so it runs on NixOS too.

```sh
herdr plugin install eliasstravik/herdr-projects
```

## What you need

- Herdr, `git`, and `curl` or `wget` in your system or user packages. Optional: `gh` and `tailscale`.
- Rust only to build the program yourself, or to work on this repo (below).

`bin/run.sh`, which runs every command Herdr starts for the plugin, also looks in Nix's folders:
`/run/current-system/sw/bin`, `~/.nix-profile/bin` and `/etc/profiles/per-user/$USER/bin`. It looks there last, so a
program found elsewhere first is the one that runs.

## The command on your PATH with Nix

The repo is a flake. Its package is the program built from source:

```sh
nix profile install github:eliasstravik/herdr-projects   # herdr-projects on your PATH
nix run github:eliasstravik/herdr-projects -- doctor     # once, without installing it
```

In a NixOS or Home Manager configuration, add the flake as an input and its package to your packages
(`inputs.herdr-projects.packages.${system}.default`).

The plugin itself still runs the program through `bin/run.sh` (a build in its checkout, or the download). The package
is for the command in a shell, and for a machine that other machines ask for its tasks ([other machines](other-machines.md)).

## Working on the repo

```sh
nix develop          # Rust, mold, Node (for npm run check), git, and what the tests run (flock, ps)
sh scripts/setup.sh
npm run check
```

Without a shell that has Rust, the setup and the check stop and say so, instead of `cargo: command not found`.

## What is different on NixOS

- **Linking.** NixOS runs no program built for other Linux systems, so the pinned mold of the workspace setup cannot be
  installed. The builds link with the `mold` on the PATH instead (the dev shell has one), or as Rust does by itself
  ([linking](linking.md)).
- **The SSH agent.** Herdr's server, started at boot or by a systemd user service, may have no `SSH_AUTH_SOCK`. Then
  every `git fetch` and `git push` over SSH fails with `Permission denied (publickey)`. The board's processes take the
  agent from a login shell, as they take the PATH and gh's login. This is not only on NixOS: any setup where Herdr starts
  without the desktop's environment has it.
- **A bare login shell.** `herdr-projects machines` checks this machine as SSH would find it. On NixOS that bare PATH
  includes `/run/current-system/sw/bin`, as NixOS's own sshd gives it.
