{
  description = "A chatbot built by FaultyMuse";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, advisory-db, rust-overlay, ... }:
    (flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        packages = import ./nix/packages.nix {
          inherit pkgs craneLib;
        };
      in
      {
        inherit packages;

        checks = import ./nix/checks.nix {
          inherit pkgs craneLib advisory-db;
          faultybot = packages.faultybot;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = packages.faultybot;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks;

          # Extra inputs can be added here
          nativeBuildInputs = [ 
            rustToolchain
          ] ++ packages.faultybot.nativeBuildInputs;
        };
      })) // {
      nixosModules.faultybot = import ./nix/module.nix { flake = self; };
    };
}
