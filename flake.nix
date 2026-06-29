{
  description = "RustBot — a Serenity/Poise Discord bot";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        rustbot = pkgs.rustPlatform.buildRustPackage {
          pname = "rustbot";
          version = "0.1.0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          # All TLS is rustls (serenity rustls_backend, reqwest rustls-tls), so no
          # OpenSSL / system libs are needed at build time.
          # The bot loads GIFs from ./assets/{bonk,hit} relative to its CWD, so we
          # ship the assets alongside the binary and run from $out/share/rustbot.
          postInstall = ''
            mkdir -p $out/share/rustbot
            cp -r assets $out/share/rustbot/assets
          '';
          meta.mainProgram = "rustbot";
        };
        default = rustbot;
      });

      # Multi-instance NixOS module. Each instance is a separate systemd service
      # with its own environment file and data dir. The bot hard-codes
      # /var/lib/rustbot, so each instance bind-mounts its own dataDir there,
      # keeping the two bots' parking data fully separate.
      nixosModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.services.rustbot;
          pkg = self.packages.${pkgs.system}.default;
        in
        {
          options.services.rustbot.instances = lib.mkOption {
            default = { };
            description = "Named RustBot instances.";
            type = lib.types.attrsOf (lib.types.submodule ({ name, ... }: {
              options = {
                environmentFile = lib.mkOption {
                  type = lib.types.path;
                  description = "File with DISCORD_TOKEN, PROTECTED_USERS, etc. (not in the nix store).";
                };
                dataDir = lib.mkOption {
                  type = lib.types.str;
                  default = "/var/lib/rustbot-${name}";
                  description = "Persistent dir bind-mounted to /var/lib/rustbot for this instance.";
                };
              };
            }));
          };

          config = lib.mkIf (cfg.instances != { }) {
            users.users.rustbot = { isSystemUser = true; group = "rustbot"; };
            users.groups.rustbot = { };

            systemd.tmpfiles.rules =
              lib.mapAttrsToList (name: inst: "d ${inst.dataDir} 0750 rustbot rustbot - -") cfg.instances;

            systemd.services = lib.mapAttrs'
              (name: inst: lib.nameValuePair "rustbot-${name}" {
                description = "RustBot instance: ${name}";
                wantedBy = [ "multi-user.target" ];
                after = [ "network-online.target" ];
                wants = [ "network-online.target" ];
                serviceConfig = {
                  ExecStart = lib.getExe pkg;
                  WorkingDirectory = "${pkg}/share/rustbot";
                  EnvironmentFile = inst.environmentFile;
                  User = "rustbot";
                  Group = "rustbot";
                  Restart = "always";
                  RestartSec = "10";
                  BindPaths = [ "${inst.dataDir}:/var/lib/rustbot" ];
                  PrivateTmp = true;
                  NoNewPrivileges = true;
                  ProtectSystem = "strict";
                  ProtectHome = true;
                  ReadWritePaths = [ inst.dataDir ];
                };
              })
              cfg.instances;
          };
        };
    };
}
