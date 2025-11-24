{
  description = "Beat Collector App";

  inputs = {
    nixpkgs.url = "nixpkgs";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # SeaORM CLI for entity generation (matching SeaORM 1.1.x)
        sea-orm-cli = pkgs.rustPlatform.buildRustPackage rec {
          pname = "sea-orm-cli";
          version = "1.1.19";

          src = pkgs.fetchCrate {
            inherit pname version;
            sha256 = "sha256-dsise5MDhR4pcD3ZWDUzTG0Q4Fg/VdKw2Q59/g6BabA=";
          };

          cargoHash = "sha256-38KIJYwRvVmChGSJwaRRWbb/HPuuTp/qnvXpo3xjRpE=";

          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ];

          meta = with pkgs.lib; {
            description = "Command line utility for SeaORM";
            homepage = "https://www.sea-ql.org/SeaORM/";
            license = with licenses; [
              mit
              asl20
            ];
          };
        };

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            # Rust toolchain
            rustToolchain
            pkgs.cargo-watch

            # SeaORM CLI for entity generation
            sea-orm-cli

            # Cargo Tarpaulin for test coverage
            pkgs.cargo-tarpaulin

            # Node.js for TailwindCSS
            pkgs.nodejs

            # Docker for PostgreSQL and Redis
            pkgs.docker
            pkgs.docker-compose

            # System dependencies
            pkgs.pkg-config
            pkgs.openssl
          ];

          shellHook = ''
            echo "Beat Collector development environment"
            echo "Run ./dev.sh to start the development server"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "beat-collector";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ rustToolchain ];
        };
      }
    );
}
