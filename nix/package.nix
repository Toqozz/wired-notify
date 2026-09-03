{ lib
, rustPlatform
, dbus
, dlib
, cairo
, pango
, pkg-config
, libxkbcommon
, libX11
, libXi
, libXrandr
, libXcursor
, libXScrnSaver
}:
rustPlatform.buildRustPackage {
  pname = "wired";
  version = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).package.version;

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  # Requires dbus cairo and pango
  # pkgconfig, glib and xorg are required for x11-crate
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    dbus
    dlib
    cairo
    pango
    libX11
    libXi
    libXrandr
    libXcursor
    libXScrnSaver
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

  preFixup = ''
    patchelf $out/bin/wired \
      --add-needed libxkbcommon-x11.so \
      --add-rpath ${libxkbcommon}/lib
  '';

  meta = {
    homepage = "https://github.com/Toqozz/wired-notify";
    downloadPage = "https://github.com/Toqozz/wired-notify/releases";
    license = lib.licenses.mit;
    mainProgram = "wired";
  };
}
