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
        EnvironmentFile = "-${cfg.envfile}";
        Restart = "always";
      };
    };
  };
}