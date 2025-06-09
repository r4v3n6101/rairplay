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
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            (rustVersion.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
            (
              pkgs.writeShellScriptBin "ffplay_video" ''
                ${pkgs.ffmpeg-full}/bin/ffplay -f rawvideo -pixel_format yuvj420p -color_range 2 -video_size 498x1080 -framerate 30 $1
              ''
            )
          ];
          nativeBuildInputs = [ pkgs.pkg-config pkgs.ffmpeg-full.dev ];

          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
          RUST_BACKTRACE = "full";
        };
      });
}
