{ stdenv, rustPlatform }:

let
  rustNightlyChannel = (nixpkgs.rustChannelOf { date = "2021-06-09"; channel = "nightly"; }).rust.override {
    extensions = [
      "rust-src"
      "rls-preview"
      "clippy-preview"
      "rustfmt-preview"
    ];
  };
in

rustNightlyChannel.buildRustPackage rec {
  pname = "cursetool-rs";
  version = "0.1.0";

  src = builtins.filterSource
    (path: type: type != "symlink" && baseNameOf path != "target")
    ./.;

  cargoSha256 = "sha256-vPNaq/pIB+f53GLvezArcNHwafNVdHwAeEb75WIKHCQ=";
}
