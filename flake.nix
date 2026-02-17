{
  description = "A server processes an audio and a video stream sent by the AirPlay protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    shairplay = {
      url = "github:r4v3n6101/shairplay";
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
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
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
            manifest = (pkgs.lib.importTOML ./examples/trancode2file/Cargo.toml).package;
            rustPlatform = pkgs.makeRustPlatform {
              cargo = rustToolchain;
              rustc = rustToolchain;
            };
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

            FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
          };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            (rustToolchain.override {
              extensions = [
                "rustfmt"
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
