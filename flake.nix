{
  description =
    "A server processes an audio and a video stream sent by the AirPlay protocol";

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
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
            gst_all_1.gst-plugins-ugly
            (gst_all_1.gst-plugins-rs.overrideAttrs (final: prev: {
              doCheck = false;
              doInstallCheck = false;
              patches = prev.patches or [ ] ++ [
                ./rtpmp4gdepay_fix_constantduration_condition.patch
              ];
            }))
            gst_all_1.gst-libav
          ];

          RUST_BACKTRACE = "full";
          FAIRPLAY3_SRC = "${shairplay}/src/lib/playfair";
        };
      });
}
