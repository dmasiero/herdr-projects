#!/bin/sh
# The linker of the builds of this repo on Linux: the C compiler runs it as its `ld` (scripts/ld/ld, named in
# .cargo/config.toml). It is mold, one pinned version checked against its published sha256 and kept in
# ~/.cache/herdr-projects beside the build cache; where that one cannot be installed (NixOS), a mold on the PATH. Where
# mold is not installed (CI, another machine, no network), and for release builds, it is the linker Rust uses by
# itself: its own lld on x86_64, the system's ld elsewhere.
# `sh scripts/linker.sh --install` installs mold (the workspace setup does); HERDR_PROJECTS_NO_MOLD=1 turns it off.
# How and why: docs/linking.md.
ln_version=2.42.1
ln_dir="${XDG_CACHE_HOME:-$HOME/.cache}/herdr-projects/mold-$ln_version"
# A mold of the system's own on the PATH (NixOS has one from its own packages): used where the pinned one is not
# installed.
ln_path_mold() { command -v mold 2>/dev/null; }
# Fetches the pinned mold into the cache; says why when it does not.
ln_fetch() {
  case "$(uname -m)" in
    x86_64) a=x86_64; sum=6ff270c9bf07d2bec5c98aa324eb7c4daf6a1a4d815c05ff1708049616047855 ;;
    aarch64|arm64) a=aarch64; sum=16b025652d3d7456689e6025a77e1903bb2a15e7630877c26cc133f5df95b9c6 ;;
    *) echo "No mold for $(uname -m)." >&2; return 0 ;;
  esac
  name="mold-$ln_version-$a-linux"; url="https://github.com/rui314/mold/releases/download/v$ln_version/$name.tar.gz"
  tmp=$(mktemp -d) || return 0
  if command -v curl >/dev/null 2>&1; then curl -fsSL -o "$tmp/a.tgz" "$url"; else wget -qO "$tmp/a.tgz" "$url"; fi || {
    rm -rf "$tmp"; echo "Could not download mold ($url)." >&2; return 0; }
  got=$( (sha256sum "$tmp/a.tgz" 2>/dev/null || shasum -a 256 "$tmp/a.tgz") | cut -d' ' -f1)
  if [ "$got" != "$sum" ]; then echo "mold did not install (sha256 $got, not $sum)." >&2
  elif ! tar -xzf "$tmp/a.tgz" -C "$tmp" || ! "$tmp/$name/bin/mold" --version >/dev/null 2>&1; then
    echo "mold did not install: its release does not run on this machine." >&2
  elif mkdir -p "${ln_dir%/*}" && mv "$tmp/$name" "$ln_dir.$$"; then
    # one rename: a link that starts while it installs finds all of mold or none of it
    mv -T "$ln_dir.$$" "$ln_dir" 2>/dev/null || rm -rf "$ln_dir.$$"
    [ -x "$ln_dir/bin/mold" ] && echo "Installed mold: $ln_dir/bin/mold"
  else echo "mold did not install (could not move it into $ln_dir)." >&2; fi
  rm -rf "$tmp"; return 0
}
ln_install() {
  [ "$(uname -s)" = Linux ] || return 0 # macOS links with Apple's own linker, already as quick
  [ -x "$ln_dir/bin/mold" ] && return 0
  # NixOS runs no program built for other Linux systems, mold's own release among them: not worth the download.
  [ -e /etc/NIXOS ] || ln_fetch
  [ -x "$ln_dir/bin/mold" ] && return 0
  if m=$(ln_path_mold); then echo "Using the mold on the PATH: $m"; else echo "No mold: this machine links as Rust does by itself." >&2; fi
}
[ "$1" = --install ] && { ln_install; exit 0; }
# A release build (its files are under a `release` folder: `-o …/release/deps/…`) links as before.
ln_mold="$ln_dir/bin/mold"
[ -x "$ln_mold" ] || ln_mold=$(ln_path_mold) || ln_mold=""
ln_use=1
[ -n "$ln_mold" ] && [ -z "$HERDR_PROJECTS_NO_MOLD" ] || ln_use=""
ln_prev=""
for a in "$@"; do [ "$ln_prev" = -o ] && case "$a" in */release/*) ln_use="" ;; esac; ln_prev=$a; done
[ -z "$ln_use" ] || exec "$ln_mold" "$@"
# Rust runs the linker with its own tools first on PATH, rust-lld among them.
[ "$(uname -m)" = x86_64 ] && command -v rust-lld >/dev/null 2>&1 && exec rust-lld -flavor gnu "$@"
exec ld "$@"
