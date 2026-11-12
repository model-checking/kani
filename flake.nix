# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

{
  description = "Kani: a bit-precise model checker for Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      inherit (nixpkgs) lib;
      # The platforms Kani publishes release bundles for.
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      forAllSystems =
        f:
        lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ self.overlays.default ];
            }
          )
        );
    in
    {
      overlays = {
        # Kani's attributes only; expects `rust-bin` from rust-overlay to be present.
        kani = import ./nix/overlay.nix;
        default = lib.composeManyExtensions [
          rust-overlay.overlays.default
          self.overlays.kani
        ];
      };

      packages = forAllSystems (pkgs: {
        inherit (pkgs.kaniPackages)
          cbmc
          kissat
          rust-toolchain
          rustup-shim
          ;
        # The latest recorded release, from its release bundle.
        kani = pkgs.kaniReleases.latest.kani;
        update-versions = pkgs.kani-update-versions;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.kaniPackages.dev-shell;
      });
    };
}
