# herdr-projects for Nix: the program as a package (`nix build`, `nix profile install`), and a shell with what building
# and checking this repo needs (`nix develop`: Rust, mold, Node for `npm run check`, git, curl, flock, ps).
# docs/nixos.md.
{
  description = "A work board for herdr";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      each = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
    in {
      packages = each (pkgs: rec {
        herdr-projects = pkgs.rustPlatform.buildRustPackage {
          pname = "herdr-projects";
          inherit version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "-p" "herdr-projects" ];
          # The tests need git, a herdr of their own and a writable home: `npm run check` in `nix develop` runs them.
          doCheck = false;
          meta = {
            description = "A work board for herdr";
            homepage = "https://github.com/eliasstravik/herdr-projects";
            license = pkgs.lib.licenses.mit;
            mainProgram = "herdr-projects";
          };
        };
        default = herdr-projects;
      });

      devShells = each (pkgs: {
        default = pkgs.mkShell {
          # curl fetches the shared build cache (scripts/build-cache.sh); the tests take locks with util-linux's flock,
          # and the program reads processes with ps (macOS has its own of both).
          packages = [ pkgs.cargo pkgs.rustc pkgs.clippy pkgs.rustfmt pkgs.nodejs pkgs.git pkgs.curl ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.mold pkgs.util-linux pkgs.procps ];
        };
      });
    };
}
