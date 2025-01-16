{
  description =
    "A server that process audio and video sent by AirPlay protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };
        manifest = (pkgs.lib.importTOML ./bin/Cargo.toml).package;
        myRustBuild = rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          #src = pkgs.lib.cleanSource ./.;
          src = pkgs.fetchgit {
            url = ./.;
            rev = "096b61ad14c90169f438e690d096e3fcf87e504e";
            fetchSubmodules = true;
          };
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          nativeBuildInputs = [ pkgs.libiconv pkgs.clang ];
        };
      in
      {
        packages.default = myRustBuild;
        formatter = pkgs.nixpkgs-fmt;
        devShell = pkgs.mkShell {
          buildInputs = [
            (rustVersion.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
          ];
        };
      });
}
