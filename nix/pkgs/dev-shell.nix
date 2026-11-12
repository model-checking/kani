# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Shell for building and testing Kani from a checkout (`cargo build-dev`, `./scripts/...`).
{
  cbmc,
  cvc5,
  git,
  kissat,
  lib,
  libiconv,
  mkShell,
  python3,
  rust-toolchain,
  rustup-shim,
  stdenv,
  toolchainChannel,
  z3,
}:
mkShell {
  # `rustup-shim` must come before `rust-toolchain` so its `cargo` wins.
  packages = [
    rustup-shim
    rust-toolchain
    cbmc
    cvc5
    git
    python3
    z3
  ]
  ++ lib.optional (kissat != null) kissat
  ++ lib.optional stdenv.hostPlatform.isDarwin libiconv;

  RUSTUP_HOME = "${rustup-shim}/rustup-home";
  RUSTUP_TOOLCHAIN = toolchainChannel;
}
