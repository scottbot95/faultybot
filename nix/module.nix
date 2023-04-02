{ flake }:
{ config, lib, pkgs, ...}:
let 
  cfg = config.services.faultybot;
in
with lib; 
{
  options.services.faultybot = {
    enable = mkEnableOption "FaultyBot chat bot";
    package = mkOption {
      type = types.package;
      default = flake.packages.${pkgs.system}.faultybot;
      defaultText = "flake.packages.\${system}.faultybot";
      description = "Package to use for FaultyBot service. Allows customizing version";
    };
    envfile = mkOption {
      type = types.path;
      description = mdDoc """
        Path to file to load environment variables from.
        Must contain at least `DISCORD_TOKEN` and `OPENAI_KEY`.
        Should be quoted so that path does not get copied to /nix/store
      """;
      example = "/run/secrets/faultybot.env";
    };
    ansi_colors = mkEnableOption "ANSI colors in log output";
    metrics = {
      listenAddress = mkOption {
        type = types.str;
        description = mdDoc "Listen address to bind prometheus exporter to";
        default = "0.0.0.0";
      };
      port = mkOption {
        type = types.port;
        description = mdDoc "Port to lisen on";
        default = 9000;
      };
    };
  };

  config = mkIf cfg.enable {
    systemd.services.faultybot = {
      description = "FaultyBot chat bot";

      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      restartIfChanged = true;

      serviceConfig = {
        DynamicUser = true;
        ExecStart = "${cfg.package}/bin/faultybot";
        EnvironmentFile = "${cfg.envfile}";
        Restart = "always";
      };

      environment = {
        ANSI_COLORS = mkIf (!cfg.ansi_colors) "false";
        METRICS_LISTEN_ADDRESS = "${cfg.metrics.listenAddress}:${cfg.metrics.port}";
      };
    };
  };
}