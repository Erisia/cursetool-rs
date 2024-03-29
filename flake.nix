{
  description = "Cursetool-rs";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
	pkgs = import nixpkgs {
	  inherit system overlays;
	};
	rust = pkgs.rust-bin.stable.latest.default;
	rustPlatform = pkgs.makeRustPlatform {
	  rustc = rust;
	  cargo = rust;
	};
      in with pkgs; {
        devShell = mkShell {
	  buildInputs = [
	    rust
	    rls
	    diffutils
	  ];

	  shellHook = ''
	    export RUST_BACKTRACE=1
	  '';
	};

	defaultPackage = rustPlatform.buildRustPackage {
	  pname = "cursetool-rs";
	  version = "0.2.0";

	  src = builtins.filterSource
	    (path: type: type != "symlink" && baseNameOf path != "target")
	    ./.;

	  doCheck = false;  # The tests require internet access.

	  cargoSha256 = "sha256-S87EM/atPVl0gJKo64ieBz3WI6Ed3D3bddamfNWPi54=";
	};
  });
}
