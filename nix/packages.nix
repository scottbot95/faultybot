{ pkgs,
  craneLib,
  ...
}:
let
  inherit (pkgs) lib;

  src = lib.cleanSourceWith {
    src = ../.;
    filter = craneLib.filterCargoSources;
  };

  nativeBuildInputs = with pkgs; [
    makeWrapper
    cargo-make
    trunk
  ];

  buildInputs = [
    # Add additional build inputs here
  ] ++ lib.optionals pkgs.stdenv.isDarwin [
    # Additional darwin specific inputs can be set here
    pkgs.libiconv
  ];

  # Build *just* the cargo dependencies, so we can reuse
  # all of that work (e.g. via cachix) when running in CI
  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src buildInputs;
  };

  # Build the actual crate itself, reusing the dependency
  # artifacts from above.
  faultybot = craneLib.buildPackage {
    inherit cargoArtifacts src buildInputs nativeBuildInputs;
  };

  faultybot-docker = pkgs.dockerTools.buildImage {
    name = "faultybot";
    config = {
      Cmd = [ "${faultybot}/bin/faultybot" ];
    };
  };
in {
  inherit faultybot faultybot-docker;

  default = faultybot;
}