let
  # Temporary fix until mozilla/nixpkgs-mozilla#250 is merged
  moz_overlay = import (builtins.fetchTarball https://github.com/andersk/nixpkgs-mozilla/archive/stdenv.lib.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  rustNightlyChannel = (nixpkgs.rustChannelOf { date = "2021-06-09"; channel = "nightly"; }).rust.override {
    extensions = [
      "rust-src"
      "rls-preview"
      "clippy-preview"
      "rustfmt-preview"
    ];
  };
in
with nixpkgs;
stdenv.mkDerivation {
  name = "moz_overlay_shell";
  buildInputs = [
    rustNightlyChannel
    rls
    rustup
    diffutils
   ];
  RUST_BACKTRACE = 1;
}
