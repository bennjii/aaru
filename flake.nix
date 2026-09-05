{
  description = "Routers — Rust-Based Routing Tooling for System-Agnostic Maps";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };

  # Rust toolchain with the wasm targets needed to build the routers_wasm
  # component; nixpkgs' `rustc` ships no wasm std.
  inputs.fenix = {
    url = "github:nix-community/fenix";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, flake-utils, fenix, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        # A single stable toolchain carrying rust-src plus the wasm targets the
        # component build and its `-Z build-std` fallback need.
        fenixPkgs = fenix.packages.${system};
        rustToolchain = fenixPkgs.combine [
          fenixPkgs.stable.rustc
          fenixPkgs.stable.cargo
          fenixPkgs.stable.clippy
          fenixPkgs.stable.rustfmt
          fenixPkgs.stable.rust-src
          fenixPkgs.targets.wasm32-unknown-unknown.stable.rust-std
          fenixPkgs.targets.wasm32-wasip1.stable.rust-std
          fenixPkgs.targets.wasm32-wasip2.stable.rust-std
        ];

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

        # The GKE component is for `kubectl` and `helm`, which call it as an
        # exec credential plugin.
        gcloud = pkgs.google-cloud-sdk.withExtraComponents [
          pkgs.google-cloud-sdk.components.gke-gcloud-auth-plugin
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = libs;

          packages = with pkgs; [
            bashInteractive

            # Rust toolchain (rustc/cargo/clippy/rustfmt + wasm targets).
            rustToolchain
            rust-analyzer

            # WebAssembly component toolchain (libs/routers_wasm): build the
            # component, transpile consumers with jco (via pnpm dlx), run it
            # under wasmtime, optimise with wasm-opt.
            wasm-tools
            cargo-component
            wasmtime
            binaryen
            nodejs_22
            pnpm

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

            # The infrastructure itself lives in routers-org/infrastructure;
            # gcloud here is for kubectl, helm and pushing to Artifact Registry.
            gcloud

            pkg-config
            cmake
            perl
            rustPlatform.bindgenHook
          ];

          env = {
            PROTOC = lib.getExe' pkgs.protobuf "protoc";
            OPENSSL_NO_VENDOR = "1";
            # Matches the toolchain above (rust-analyzer + `-Z build-std`).
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
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
