#! /usr/bin/env bash

# Some additional cargo checks to run manually before publishing.
# These exist because neither appveyor nor travis-ci have an obvious way to do
# builds for foreign architectures.

set -eux

(
    cd proptest
    cargo clean
    cargo +nightly build --target thumbv7em-none-eabihf \
          --no-default-features --features 'alloc unstable'
    cargo clean
    cargo +nightly build --target wasm32-unknown-emscripten \
          --no-default-features --features std
    cargo clean
    cargo +nightly build --target wasm32-unknown-unknown \
          --no-default-features --features std
)
