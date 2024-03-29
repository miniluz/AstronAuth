{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          src = craneLib.cleanCargoSource (craneLib.path ./.);

          nativeBuildInputs = [ rustToolchain ];
          buildInputs = [ ];

          commonArgs = {
            inherit src buildInputs nativeBuildInputs;
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          bin = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });

          dockerImage = pkgs.dockerTools.streamLayeredImage {
            name = "AstronAuth";
            tag = "latest";
            contents = [ bin ];
            config = {
              Cmd = [ "${bin}/bin/astronauth" ];
            };
          };
        in
        with pkgs;
        {
          packages = {
            inherit bin dockerImage;
            default = bin;
          };
          devShells.default = mkShell {
            inherit nativeBuildInputs;
            buildInputs = [ ];
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          };
        }
      );
}
