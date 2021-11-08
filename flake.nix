{
  description =
    "Lightweight notification daemon with highly customizable layout blocks, written in Rust.";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        naersk-lib = naersk.lib."${system}";
        # Requires dbus cairo and pango
        # pkgconfig, glib and xorg is required for x11-crate
        buildInputs = with pkgs; [
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
      in rec {
        # `nix build`
        packages.wired = naersk-lib.buildPackage {
          pname = "wired";
          root = ./.;
          inherit buildInputs;
          # Without this wired_derive build would fail
          singleStep = true;
        };
        defaultPackage = packages.wired;

        # `nix run`
        apps.wired = utils.lib.mkApp { drv = packages.wired; };
        defaultApp = apps.wired;

        # `nix develop`
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo gcc clippy rustfmt ];
          inherit buildInputs;
        };
      });
}
