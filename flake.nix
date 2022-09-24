{
  description = "Lightweight notification daemon with highly customizable layout blocks, written in Rust.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    alejandra = {
      url = "github:kamadorueda/alejandra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    alejandra,
    ...
  }: let
    inherit (builtins) fromTOML readFile substring mapAttrs;

    cargoToml = fromTOML (readFile ./Cargo.toml);
    version = "${cargoToml.package.version}_${substring 0 8 self.lastModifiedDate}_${self.shortRev or "dirty"}";

    mkWired = {
      lib,
      rustPlatform,
      dbus,
      dlib,
      cairo,
      pango,
      pkgconfig,
      xorg,
      ...
    }:
      rustPlatform.buildRustPackage {
        name = "wired-${version}";

        src = lib.cleanSource ./.;
        cargoLock.lockFile = ./Cargo.lock;

        # Requires dbus cairo and pango
        # pkgconfig, glib and xorg are required for x11-crate
        nativeBuildInputs = [pkgconfig];
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
        };
      };
  in ({
      # `nix fmt` (added in Nix 2.8)
      # (generating this outside of `eachDefaultSystem` because alejandra's supported systems may not match ours)
      formatter = mapAttrs (system: pkgs: pkgs.default) alejandra.packages;

      # consumed by github:nix-community/home-manager
      homeManagerModules.default = import ./home-manager.nix;

      overlays = {
        default = final: prev: {
          wired = prev.callPackage mkWired {};
        };
      };
    }
    // (
      utils.lib.eachDefaultSystem (system: let
        pkgs = import nixpkgs {inherit system;};
        wired = pkgs.callPackage mkWired {};
      in {
        # `nix build`
        packages = {
          inherit wired;
          default = wired;
        };
        # `defaultPackage` deprecated in Nix 2.7
        defaultPackage = self.packages.${system}.default;

        # `nix run`
        apps.wired = {
          type = "app";
          program = "${wired}/bin/wired";
        };
        apps.default = self.apps.${system}.wired;
        # `defaultApp` deprecated in Nix 2.7
        defaultApp = self.apps.${system}.default;

        # `nix develop`
        devShells.default = pkgs.mkShell {
          nativeBuildInputs =
            wired.nativeBuildInputs
            ++ (with pkgs; [rustc cargo gcc clippy rustfmt]);
          inherit (wired) buildInputs;
        };
        # `devShell` deprecated in Nix 2.7
        devShell = self.devShells.${system}.default;
      })
    ));
}
