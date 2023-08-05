{ flake }:
{ config, lib, pkgs, ...}:
with lib;
let 
  cfg = config.services.faultybot;
  promCfg = config.services.prometheus.exporters.faultybot;
  faultybotConfig = builtins.toFile "faultybot.yaml" (lib.generators.toYAML {} cfg.settings);
in
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
        Must contain at least `DISCORD__TOKEN` and `OPENAI__KEY`.
        Should be quoted so that path does not get copied to /nix/store
      """;
      example = "/run/secrets/faultybot.env";
    };
    ansi_colors = mkEnableOption "ANSI colors in log output";
    log_level = mkOption {
      type = types.str;
      default = "info";
      description = mdDoc """
        Controls the log level.

        See [tracing-subscriber's EnvFilter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
        for details
      """;
      example = "warn,faultybot=info";
    };
    metrics = {
      statsd = {
        enable = mkEnableOption "statsd metrics exporter";
        host = mkOption {
          type = types.str;
          description = "Host to send statsd updates to.";
          default = "127.0.0.1";
        };
        port = mkOption {
          type = types.port;
          description = "Port on `host` to send metrics to.";
          default = 8125;
        };
      };
    };
    settings = mkOption {
      type = (pkgs.formats.yaml { }).type;
      default = { };
      description = "Additional settings to be included the generated config file";
    };
  };

  options.services.prometheus.exporters.faultybot = {
    enable = mkEnableOption "FaultyBot Prometheus exporter";
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
    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = lib.mdDoc ''
        Open port in firewall for incoming connections.
      '';
    };
  };

  config = mkIf cfg.enable {
    services.faultybot.settings = {
      ansi.colors = mkIf (!cfg.ansi_colors) "false";
      prometheus.listen = mkIf promCfg.enable "${promCfg.listenAddress}:${toString promCfg.port}";
      statsd = mkIf cfg.metrics.statsd.enable {
        host = cfg.metrics.statsd.host;
        port = toString cfg.metrics.statsd.port;
      };
    };

    systemd.services.faultybot = {
      description = "FaultyBot chat bot";

      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      restartIfChanged = true;

      serviceConfig = {
        DynamicUser = true;
        ExecStart = "${cfg.package}/bin/faultybot -c ${faultybotConfig}";
        EnvironmentFile = "-${cfg.envfile}";
        Restart = "always";
      };

      environment = {
        RUST_LOG = cfg.log_level;
      };
    };

    networking.firewall.allowedTCPPorts = mkIf (promCfg.enable && promCfg.openFirewall) [ promCfg.port ];

    assertions = [{
      assertion = !(promCfg.enable && cfg.metrics.statsd.enable);
      message = "Cannot enable both prometheus and statsd recorders";
    }];
  };
}