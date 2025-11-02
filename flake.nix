{
  description = "A server processes an audio and a video stream sent by the AirPlay protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    shairplay = {
      url = "github:juhovh/shairplay";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      shairplay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustVersion = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustVersion;
          rustc = rustVersion;
        };
        manifest = (pkgs.lib.importTOML ./examples/trancode2file/Cargo.toml).package;
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

        packages.default = rustPlatform.buildRustPackage {
          pname = "rairplay-transcode2file";
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

          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            (rustVersion.override {
              extensions = [
                "rust-src"
                "rust-analyzer"
              ];
            })
          ];

          nativeBuildInputs = [ pkgs.pkg-config ] ++ gstreamer-libs;

          RUST_BACKTRACE = "full";
          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };
      }
    );
}
