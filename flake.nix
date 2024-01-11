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

  outputs = {
    naersk,
    nixpkgs,
    rust-overlay,
    self,
    flake-utils,
    cargo2nix,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [cargo2nix.overlays.default];
      pkgs = (import nixpkgs) {inherit system overlays;};
      workspaceShell = rustPkgs.workspaceShell {
        # This adds cargo2nix to the project shell via the cargo2nix flake
        packages = [cargo2nix.packages."${system}".cargo2nix];
      };
      rustPkgs = pkgs.rustBuilder.makePackageSet {
        packageFun = import ./Cargo.nix;
        rustVersion = "1.73.0";
        extraRustComponents = [
          "rust-analyzer"
          "clippy"
        ];
      };
    in rec {
      devShells = {
        default = workspaceShell; # nix develop
      };
      packages = {
        dmux = (rustPkgs.workspace.dmux {}).bin;
        default = packages.dmux;
      };
      apps = rec {
        dmux = {
          type = "app";
          program = "${packages.default}/bin/dmux";
        };
        default = dmux;
      };
    });
}
