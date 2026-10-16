#!/bin/sh
# Finds Rust's cargo for the scripts that build (scripts/check.sh, scripts/setup.sh), which source it: on the PATH, else
# where rustup puts it (~/.cargo/bin) or a Nix profile does (~/.nix-profile/bin, and NixOS's
# /etc/profiles/per-user/$USER/bin), which the PATH that herdr's server or a bare login gives may lack. `cargo_found` puts that folder first on the PATH;
# when there is none it fails, and `cargo_hint` says how to get Rust. Sourced under `set -e`: nothing here may fail.
cargo_hint="install it with https://rustup.rs, or on NixOS start a shell that has it: nix develop (docs/nixos.md)"
cargo_found() {
  command -v cargo >/dev/null 2>&1 && return 0
  for d in "$HOME/.cargo/bin" "$HOME/.nix-profile/bin" ${USER:+"/etc/profiles/per-user/$USER/bin"}; do
    [ -x "$d/cargo" ] && { PATH="$d:$PATH"; export PATH; return 0; }
  done
  return 1
}
