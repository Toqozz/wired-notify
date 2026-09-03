{
  description = "Lightweight notification daemon with highly customizable layout blocks, written in Rust.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {

      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake = {
        homeManagerModules.default = import ./nix/home-manager.nix;

        overlays.default = final: prev: {
          wired = prev.callPackage ./nix/package.nix { };
        };
      };

      perSystem = { self', system, pkgs, ... }:
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = with inputs; [
              rust-overlay.overlays.default
            ];
          };

          formatter = pkgs.nixpkgs-fmt;

          apps = {
            default = self'.apps.wired;

            wired = {
              type = "app";
              program = "${pkgs.lib.getExe self'.packages.default}";
            };
          };

          packages.default = pkgs.callPackage ./nix/package.nix { };

          devShells.default =
            let
              rust-toolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                extensions = [ "rust-src" "rust-analyzer" ];
              };
            in
            pkgs.mkShell {
              inputsFrom = [ self'.packages.default ];
              packages = [ rust-toolchain ];
              env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.libxkbcommon ];
            };
        };
    };
}
