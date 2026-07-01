{
  description = "RustBot — a Serenity/Poise Discord bot";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  # crane gives us incremental Nix builds: dependency crates are compiled once
  # into a cached artifact, so editing the bot's own code recompiles only the
  # rustbot crate instead of all ~450 deps every time. Pinned to v0.20.3, the
  # last line that supports nixpkgs 24.11/25.05 (master now requires 25.11).
  inputs.crane.url = "github:ipetkov/crane/v0.20.3";
  # songbird 0.6's DAVE stack (davey -> openmls) needs a newer rustc than the one
  # nixpkgs 25.05 ships (1.86). rust-overlay lets us build with a current stable
  # toolchain while the rest of the system stays on the pinned nixpkgs.
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, crane, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        f (import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        }));
    in
    {
      packages = forAllSystems (pkgs:
        let
          # Build with a current stable rustc (openmls uses Rust 1.87+ features);
          # only the build toolchain changes, not the system.
          craneLib = (crane.mkLib pkgs).overrideToolchain
            (p: p.rust-bin.stable."1.88.0".default);

          # All TLS is rustls (serenity rustls_backend, reqwest rustls-tls). The one
          # native dependency is libopus, needed by songbird's voice encoder; its
          # `audiopus_sys` build script finds it via pkg-config.
          commonArgs = {
            pname = "rustbot";
            version = "0.1.0";
            src = craneLib.cleanCargoSource self;
            strictDeps = true;
            # cmake is a fallback: songbird 0.6's opus2 -> libopus_sys prefers the
            # system libopus via pkg-config, but can build a bundled copy with cmake.
            nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper pkgs.cmake ];
            buildInputs = [ pkgs.libopus ];
          };

          # Compile just the dependencies. This derivation is keyed on Cargo.toml /
          # Cargo.lock, so it stays cached across rebuilds until the deps change.
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          rustbot = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
            # The final build also needs the runtime GIF assets, which the
            # cargo-only source filter above strips out — use a fuller source here.
            src = pkgs.lib.cleanSource self;
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
          });
        in
        {
          inherit rustbot;
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
