{
  description = "Lightweight notification daemon with highly customizable layout blocks, written in Rust.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    alejandra = {
      url = "github:kamadorueda/alejandra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
    alejandra,
  }:
    with builtins; let
      std = nixpkgs.lib;
    in ({
        # `nix fmt` (added in Nix 2.8)
        # (generating this outside of `eachDefaultSystem` because alejandra's supported systems may not match ours)
        formatter = std.mapAttrs (system: pkgs: pkgs.default) alejandra.packages;

        # consumed by github:nix-community/home-manager
        homeManagerModules.default = import ./home-manager.nix;
      }
      // (
        utils.lib.eachDefaultSystem (system: let
          pkgs = import nixpkgs {
            # `builtins.currentSystem` isn't actually provided when building a flake;
            # flakes don't yet have a standard pipeline for cross-compilation, so
            # this is just here to try to convey intent
            localSystem = builtins.currentSystem or system;
            crossSystem = system;
            overlays = [self.overlays.${system}];
          };
          naersk-lib = naersk.lib."${system}";
        in {
          overlays = final: prev: {
            wired = naersk-lib.buildPackage {
              pname = "wired";
              src = ./.;
              meta = {
                homepage = "https://github.com/Toqozz/wired-notify";
                downloadPage = "https://github.com/Toqozz/wired-notify/releases";
                license = std.licenses.mit;
              };
              # Requires dbus cairo and pango
              # pkgconfig, glib and xorg are required for x11-crate
              buildInputs = with final; [
                dbus
                dlib
                cairo
                pango
                pkgconfig
                xorg.libX11
                xorg.libXi
                xorg.libXrandr
                xorg.libXcursor
                xorg.libXScrnSaver
              ];
              # Without this wired_derive build would fail
              singleStep = true;
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
            };
          };

          # `nix build`
          packages.wired = pkgs.wired;
          packages.default = self.packages.${system}.wired;
          # `defaultPackage` deprecated in Nix 2.7
          defaultPackage = self.packages.${system}.default;

          # `nix run`
          apps.wired = utils.lib.mkApp {drv = self.packages.${system}.wired;};
          apps.default = self.apps.${system}.wired;
          # `defaultApp` deprecated in Nix 2.7
          defaultApp = self.apps.${system}.default;

          # `nix develop`
          devShells.default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [rustc cargo gcc clippy rustfmt];
            inherit (self.packages.${system}.wired) buildInputs;
          };
          # `devShell` deprecated in Nix 2.7
          devShell = self.devShells.${system}.default;
        })
      ));
}
