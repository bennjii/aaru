{
  description = "Routers — Rust-Based Routing Tooling for System-Agnostic Maps";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        # Linked by openssl-sys/aws-lc-sys, dlopen'd by eframe in routers_viewer.
        libs =
          with pkgs;
          [ fontconfig openssl zlib ]
          ++ lib.optionals stdenv.hostPlatform.isLinux [
            libGL
            libx11
            libxcursor
            libxi
            libxkbcommon
            libxrandr
            vulkan-loader
            wayland
          ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = libs;

          packages = with pkgs; [
            bashInteractive

            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer

            protobuf
            buf

            cargo-audit
            cargo-codspeed
            cargo-insta
            cargo-nextest
            git-cliff

            just
            pre-commit
            git-lfs
            curl
            unzip

            kubectl
            kubernetes-helm

            pkg-config
            cmake
            perl
            rustPlatform.bindgenHook
          ];

          env = {
            PROTOC = lib.getExe' pkgs.protobuf "protoc";
            OPENSSL_NO_VENDOR = "1";
            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          };

          shellHook = ''
            export LD_LIBRARY_PATH="${lib.makeLibraryPath libs}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

            # `buf generate` needs this plugin and nixpkgs has no derivation for
            # it. Pinned to the workspace's `buffa` version.
            export PATH="$PWD/.cargo-tools/bin:$PATH"
            mkdir -p .cargo-tools && echo "*" > .cargo-tools/.gitignore
            [ -x .cargo-tools/bin/protoc-gen-buffa-packaging ] || \
              cargo install --locked --quiet --root .cargo-tools \
                protoc-gen-buffa-packaging@0.6.0

            [ -d schema/src/proto ] || echo "run 'buf generate' before building"
          '';
        };
      }
    );
}
