{
  config,
  pkgs,
  lib,
  ...
}:
with builtins; let
  std = pkgs.lib;
  cfg = config.services.wired;
in {
  options.services.wired = with lib; {
    enable = mkEnableOption "wired notification daemon";
    package = mkOption {
      type = types.package;
      default = pkgs.wired;
      description = "Package providing wired.";
    };
    config = mkOption {
      type = types.nullOr types.path;
      default = null;
      example = ./wired.ron;
      description = "File containing wired configuration.";
    };
  };
  config = lib.mkMerge [
    (lib.mkIf cfg.enable {
      home.packages = [cfg.package];
      # As far as I know, the only "official" way to install systemd units
      # through home-manager is to define `systemd.user.<unit name>`,
      # which only allows unit configuration directly from Nix (i.e. you
      # can't just give it a raw file). So, this just installs the existing service.
      xdg.dataFile."systemd/user/wired.service".source = "${cfg.package}/usr/lib/systemd/system/wired.service";
    })
    (lib.mkIf (cfg.enable && cfg.config != null) {
      # Ideally, we could generate the config from a Nix expression,
      # but that's complicated, so right now this just symlinks a file.
      xdg.configFile."wired/wired.ron".source = cfg.config;
    })
  ];
}
