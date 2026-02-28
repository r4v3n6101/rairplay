{
  description = "A server processes an audio and a video stream sent by the AirPlay protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
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
      naersk,
      shairplay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk' = pkgs.callPackage naersk { };
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

        packages.default = naersk'.buildPackage {
          name = "rairplay-transcode2file";
          version = "0.1.0";
          src = ./.;

          nativeBuildInputs = with pkgs; [
            libiconv
            pkg-config
          ];

          buildInputs = gstreamer-libs;

          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            rust-analyzer
            rustPackages.clippy
          ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          nativeBuildInputs = [ pkgs.pkg-config ] ++ gstreamer-libs;

          RUST_BACKTRACE = "full";
          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };
      }
    );
}
