{
  description = "A server processes an audio and a video stream sent by the AirPlay protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rustVersion = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };
        gstreamer-libs = with pkgs; [
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-bad
          gst_all_1.gst-plugins-ugly
          gst_all_1.gst-plugins-rs
          gst_all_1.gst-libav
        ];
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages.default =
          let
            manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
          in
          rustPlatform.buildRustPackage {
            pname = manifest.name;
            version = manifest.version;

            src = pkgs.lib.cleanSource ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            nativeBuildInputs = with pkgs; [
              libiconv
              pkg-config
            ];

            buildInputs = gstreamer-libs;
          };

        devShells.default = pkgs.mkShell {
          buildInputs = [ rustVersion ];
          nativeBuildInputs = [ pkgs.pkg-config ] ++ gstreamer-libs;

          RUST_BACKTRACE = "full";
        };
      }
    );
}
