# The whole check on a real NixOS (the fork's own file, FORK.md): a NixOS VM, with /etc/NIXOS, next to nothing in
# /usr/bin and /bin, its programs in /run/current-system/sw/bin and coreutils as one program, runs
# `npm run check -- --all` as an ordinary user, offline, with the crates of Cargo.lock vendored. CI's NixOS job and the
# sync script run it: `nix build .#nixos-check` (KVM needed). A Nix shell on another Linux is not this.
{ pkgs, src }:
let
  vendor = pkgs.rustPlatform.importCargoLock { lockFile = src + "/Cargo.lock"; };
  cargoConfig = pkgs.writeText "cargo-config.toml" ''
    [source.crates-io]
    replace-with = "vendored-sources"

    [source.vendored-sources]
    directory = "${vendor}"

    [net]
    offline = true
  '';
in
pkgs.testers.runNixOSTest {
  name = "herdr-projects-check-on-nixos";
  nodes.machine = { ... }: {
    virtualisation = { cores = 4; memorySize = 8192; diskSize = 20480; };
    # What the dev shell has (flake.nix), as a NixOS user has it: system packages.
    environment.systemPackages = with pkgs; [ cargo rustc clippy rustfmt stdenv.cc nodejs git curl mold util-linux procps ];
    programs.ssh.startAgent = true;
    users.users.dev = { isNormalUser = true; };
  };
  testScript = ''
    machine.wait_for_unit("multi-user.target")
    machine.succeed("test -e /etc/NIXOS")
    machine.succeed("cp -r ${src} /home/dev/repo && mkdir -p /home/dev/.cargo && cp ${cargoConfig} /home/dev/.cargo/config.toml")
    machine.succeed("chown -R dev:users /home/dev && chmod -R u+w /home/dev")
    machine.succeed("su - dev -c 'cd repo && git init -q && git add -A && git -c user.name=ci -c user.email=ci@example.com commit -qm src'")
    print(machine.succeed("su - dev -c 'cd repo && GIT_AUTHOR_NAME=ci GIT_AUTHOR_EMAIL=ci@example.com GIT_COMMITTER_NAME=ci GIT_COMMITTER_EMAIL=ci@example.com npm run check -- --all 2>&1'"))
  '';
}
