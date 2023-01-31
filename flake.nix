{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    # Note: fenix packages are cached via cachix:
    #       cachix use nix-community
    fenix-flake = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix-flake, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            fenix-flake.overlays.default
          ];
        };
        toolchain = pkgs.fenix.complete.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustc"
          "rustfmt"
        ];
      in
      {
        devShell = pkgs.mkShell {
          CHALK_OVERFLOW_DEPTH = 3000;
          CHALK_SOLVER_MAX_SIZE = 1500;
          RUSTC_WRAPPER="${pkgs.sccache}/bin/sccache";
          buildInputs = with pkgs; [
            toolchain
            cargo-udeps
          ];
        };
      });
}
