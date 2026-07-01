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
          # All TLS is rustls (serenity rustls_backend, reqwest rustls-tls). The one
          # native dependency is libopus, needed by songbird's voice encoder; its
          # `audiopus_sys` build script finds it via pkg-config.
          nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
          buildInputs = [ pkgs.libopus ];
          # The bot loads GIFs from ./assets/{bonk,hit} relative to its CWD, so we
          # ship the assets alongside the binary and run from $out/share/rustbot.
          # The -play command shells out to yt-dlp and ffmpeg at runtime, so put
          # both on the binary's PATH regardless of how the service invokes it.
          postInstall = ''
            mkdir -p $out/share/rustbot
            cp -r assets $out/share/rustbot/assets
            wrapProgram $out/bin/rustbot \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.yt-dlp pkgs.ffmpeg ]}
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
                package = lib.mkOption {
                  type = lib.types.package;
                  default = pkg;
                  defaultText = lib.literalExpression "the flake's default rustbot package";
                  description = ''
                    The rustbot build this instance runs. Defaults to this flake's
                    package (tracking main). Override to run a different branch/commit
                    for a given instance, e.g. point the dev instance at a feature
                    branch while stable stays on main.
                  '';
                };
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
                  ExecStart = lib.getExe inst.package;
                  WorkingDirectory = "${inst.package}/share/rustbot";
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
