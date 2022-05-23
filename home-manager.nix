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
      # Ideally this would just install the service unit already provided in this repo,
      # but Home Manager doesn't have an idiomatic way to do that as of 2022-05-22
      systemd.user.services."wired" = {
        Unit = {
          Description = "Wired Notification Daemon";
          PartOf = "graphical-session.target";
        };
        Service = {
          Type = "dbus";
          BusName = "org.freedesktop.Notifications";
          ExecStart = "${cfg.package}/bin/wired";
        };
        Install = {
          WantedBy = [ "graphical-session.target" ];
        };
      };
    })
    (lib.mkIf (cfg.enable && cfg.config != null) {
      # Ideally, we could generate the config from a Nix expression,
      # but that's complicated, so right now this just symlinks a file.
      xdg.configFile."wired/wired.ron".source = cfg.config;
    })
  ];
}
