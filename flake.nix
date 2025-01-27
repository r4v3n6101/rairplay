{
  description =
    "A server that process audio and video sent by AirPlay protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    shairplay = {
      url = "github:juhovh/shairplay";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, shairplay, ... }:
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
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages.default = rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          src = pkgs.lib.cleanSource ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };
          nativeBuildInputs = [ pkgs.libiconv ];

          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            (rustVersion.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
          ];

          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };
      });
}
