{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    naersk,
    nixpkgs,
    rust-overlay,
    self,
    utils,
  }:
    utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = (import nixpkgs) {inherit system overlays;};
      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      naersk' = pkgs.callPackage naersk {
        cargo = toolchain;
        rustc = toolchain;
        clippy = toolchain;
      };
      buildInputs = with pkgs; lib.optionals stdenv.isDarwin [libiconv darwin.apple_sdk.frameworks.Security];
    in {
      defaultPackage = naersk'.buildPackage ./.;
      devShell = with pkgs;
        mkShell {
          buildInputs = [
            tmux
            fzf
            cargo
            rustc
            rustfmt
            pre-commit
            rustPackages.clippy
            rust-analyzer
          ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
    });
}
