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
        homeManagerModules.default = import ./home-manager.nix;
      };

      perSystem = { self', system, pkgs, ... }:
        let
          inherit (builtins) fromTOML readFile;

          cargoToml = fromTOML (readFile ./Cargo.toml);
          version = "${cargoToml.package.version}";
          mkWired =
            { lib
            , rustPlatform
            , dbus
            , dlib
            , cairo
            , pango
            , pkg-config
            , xorg
            , ...
            }:
            rustPlatform.buildRustPackage {
              name = "wired-${version}";

              src = lib.cleanSource ./.;
              cargoLock.lockFile = ./Cargo.lock;

              # Requires dbus cairo and pango
              # pkgconfig, glib and xorg are required for x11-crate
              nativeBuildInputs = [ pkg-config ];
              buildInputs = [
                dbus
                dlib
                cairo
                pango
                xorg.libX11
                xorg.libXi
                xorg.libXrandr
                xorg.libXcursor
                xorg.libXScrnSaver
              ];
              # install extra files (i.e. the systemd service)
              postInstall = ''
                # /usr/bin/wired doesn't exist, here, because the binary will be somewhere in /nix/store,
                # so this fixes the bin path in the systemd service and writes the updated file to the output dir.
                mkdir -p $out/usr/lib/systemd/system
                substitute ./wired.service $out/usr/lib/systemd/system/wired.service --replace /usr/bin/wired $out/bin/wired
                # install example/default config files to etc/wired -- Arch packages seem to use etc/{pkg} for this,
                # so there's precedent
                install -Dm444 -t $out/etc/wired wired.ron wired_multilayout.ron
              '';

              meta = {
                homepage = "https://github.com/Toqozz/wired-notify";
                downloadPage = "https://github.com/Toqozz/wired-notify/releases";
                license = lib.licenses.mit;
                mainProgram = "wired";
              };
            };
        in
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

          packages.default = pkgs.callPackage mkWired { };

          devShells.default =
            let
              wired = pkgs.callPackage mkWired { };

              rust-toolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                extensions = [ "rust-src" "rust-analyzer" ];
              };
            in
            pkgs.mkShell {
              packages = [ rust-toolchain ] ++ wired.nativeBuildInputs ++ wired.buildInputs;
            };
        };
    };
}
