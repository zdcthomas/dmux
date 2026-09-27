{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/release-0.11.0";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs =
    {
      naersk,
      nixpkgs,
      rust-overlay,
      self,
      flake-utils,
      cargo2nix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        # crates.io rejects the `curl/<ver> Nixpkgs/<ver>` user agent that
        # nixpkgs `fetchurl` sends and answers `403`, so every crate fetch that
        # cargo2nix points at `crates.io/api/v1/.../download` fails. The static
        # CDN serves the identical tarball and does not filter user agents.
        staticCratesIoOverlay = final: prev: {
          rustBuilder = prev.rustBuilder.overrideScope (
            _: prevScope: {
              rustLib = prevScope.rustLib // {
                fetchCratesIo =
                  {
                    name,
                    version,
                    sha256,
                  }:
                  final.buildPackages.fetchurl {
                    name = "${name}-${version}.tar.gz";
                    url = "https://static.crates.io/crates/${name}/${name}-${version}.crate";
                    inherit sha256;
                  };
              };
            }
          );
        };
        overlays = [
          cargo2nix.overlays.default
          staticCratesIoOverlay
        ];
        pkgs = (import nixpkgs) { inherit system overlays; };
        # The cargo2nix flake builds its own binary from its own `pkgs`, which
        # never sees `staticCratesIoOverlay` and so still hits the `403`. Build
        # it here from the same sources with the patched `pkgs` instead.
        cargo2nixPkgs = pkgs.rustBuilder.makePackageSet {
          packageFun = import "${cargo2nix}/Cargo.nix";
          workspaceSrc = cargo2nix;
          rustVersion = "1.75.0";
          packageOverrides = p: p.rustBuilder.overrides.all;
        };
        cargo2nixBin = (cargo2nixPkgs.workspace.cargo2nix { }).bin;
        workspaceShell = rustPkgs.workspaceShell {
          # This adds cargo2nix to the project shell
          packages = [ cargo2nixBin ];
        };
        rustPkgs = pkgs.rustBuilder.makePackageSet {
          packageFun = import ./Cargo.nix;
          rustVersion = "1.73.0";
          extraRustComponents = [
            "rust-analyzer"
            "clippy"
          ];
        };
      in
      rec {
        devShells = {
          default = workspaceShell; # nix develop
        };
        packages = {
          dmux = (rustPkgs.workspace.dmux { }).bin;
          default = packages.dmux;
        };
        apps = rec {
          dmux = {
            type = "app";
            program = "${packages.default}/bin/dmux";
          };
          default = dmux;
        };
      }
    );
}
