{
  description = "Podcast RSS feed for RaiPlay sound";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";

    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = (import nixpkgs) { inherit system; };

        toolchain = fenix.packages.${system}.stable.toolchain;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };

        package = rustPlatform.buildPackage {
          pname = "sarasara";
          version = "0.1.0";
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
        };
      in
      {
        # defaultPackage = naersk'.buildPackage {
        #   src = ./.;
        #   nativeBuildInputs = with pkgs; [ pkg-config ];
        #   buildInputs = with pkgs; [ openssl ];
        # };

        devShell = pkgs.mkShell {
          buildInputs = [
            toolchain
            pkgs.rust-analyzer
            pkgs.lldb

            # openssl
            # pkg-config

            # rustc
            # cargo
            # rust-analyzer
            # cargo-watch
            # rustfmt
            # pkg-config
            # openssl
            # rnix-lsp
          ];
        };
      }
    );
}
