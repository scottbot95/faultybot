# Faultybot

Faultybot is a ChatGPT-powered ChatBot for use with discord.

## Features

- ChatGPT powered responses
- What you wanted more features????

## Requirements

### OpenAI

The OpenAI API token can be acquired [here](https://platform.openai.com/account/api-keys). Once acquired,
you will need to either set `openai.key` in the `faultybot.yaml` file or the `OPENAI_KEY` env var.

### Discord

Please follow [the Discord docs](https://discord.com/developers/docs/getting-started) for creating a bot to
acquire the Discord API token. Similarly, once acquired either pass that in using `discord.token` or `DISCORD_TOKEN`.

### PostgresQL

FaultyBot also uses a PostgreSQL 15+ server for managing persistent data.
Once the server is setup and a database created, pass the connection string via `database.url` or `DATABASE_URL`.
Alternatively you can set the components of the connection string individually via `database.host`, `database.name`, etc...

Examples for setting up postgres are detailed below.

## Usage

Configuration is set by default in `<workdir>/config/faultybot.yaml` however a custom config file can be
specific with `-c/--configFile <file>`. All config values can alternatively be specified via environment variables,
using `_` as the separator for nesting (ie the `openai.key` config value can be set via `OPENAI_KEY`). Environment variables
have precedence over values in the config file. 

### NixOS

A NixOS module is provided to easily integrate FaultyBot into any NixOS configuration.

Simply add the following to your `flake.nix`

```nix
# flake.nix
{
  inputs = {
    faultybox.url = "github:scottbot95/faultybox";
  };
}
```

Then use the module like so
```nix
# configuration.nix

{ config, pkgs, ... }: {
  services.faultybot = {
    enable = true;
    envfile = "/run/secrets/faultybot.env";
    settings = {
      database.url = "postgresql:///faultybot?host=/var/run/postgresql";
    };
  };

  services.postgresql = {
    enable = true;
    # Postgres 15+ is required
    package = pkgs.postgresql_15;
    # Need an init script since you can't grant schema under a specific database with
    # services.postgresql.ensureUsers
    initialScript = pkgs.writeText "faultybot-initScript" ''
      CREATE DATABASE faultybot;
      CREATE USER "faultybot";
      GRANT ALL PRIVILEGES ON DATABASE faultybot TO "faultybot";
      \c faultybot
      GRANT ALL ON SCHEMA public TO "faultybot";
    '';
  };

  # Ensure the database starts up before trying to launch faultybot
  systemd.services.faultybot = {
    requires = [ "postgresql.service" ];
    after = [ "postgresql.service" ];
  };
}
```

### Docker

A docker image can also easily be built using `nix` like so:

```shell
# Build the gzipped image
$ nix build .#faultybot-docker

# Import the image into the docker daemon
$ docker load < result
Loaded image: faultybot:7y6jkxxvn5qhqky07zgr9s6zmqpc71lz

# Run it!
$ docker run -t faultybot:7y6jkxxvn5qhqky07zgr9s6zmqpc71lz
```

You can also use the image from `docker-compose`:
```yaml
# docker-compose.yaml
version: '3.8'
services:
  db:
    image: postgres:15.3-alpine
    restart: always
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=postgres
    ports:
      - '5432:5432'
    volumes:
      - db:/var/lib/postgresql/data
      - ./init-db.sql:/docker-entrypoint-initdb.d/init-db.sql
  faultybot:
    image: faultybot:7y6jkxxvn5qhqky07zgr9s6zmqpc71lz
    restart: always
    depends_on:
      - db
    environment:
      DATABASE_URL: postgresql://postgres:postgres@db:5432/faultybot
      DISCORD_TOKEN: <my token>
      OPENAI_KEY: <my key>
volumes:
  db:
```

```postgresql
--  init-db.sql

CREATE DATABASE faultybot;
```

### Native

You can also just build/run the project natively from source using `cargo`.

```shell
# Build in debug mode
cargo build

# Build in release mode
cargo build --relase

# Run in debug mode
cargo run

# Run in release mode
cargo run --release
```
