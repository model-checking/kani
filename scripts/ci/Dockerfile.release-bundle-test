# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Note: this file is intended only for testing the kani release bundle

FROM ubuntu:20.04
ENV DEBIAN_FRONTEND=noninteractive \
    DEBCONF_NONINTERACTIVE_SEEN=true
RUN apt-get update && \
    apt-get install -y python3 python3-pip curl ctags && \
    curl -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# This section is roughly (+tests) what "first time setup" will do:
RUN rustup toolchain install nightly-2022-03-23
WORKDIR /tmp/kani
COPY ./tests ./tests
COPY ./kani-latest-x86_64-unknown-linux-gnu.tar.gz ./
RUN \
    tar zxf kani-latest-x86_64-unknown-linux-gnu.tar.gz && \
    ln -s cargo-kani kani-latest/bin/kani && \
    ln -s /root/.rustup/toolchains/nightly-2022-03-23-x86_64-unknown-linux-gnu kani-latest/toolchain && \
    python3 -m pip install cbmc-viewer==2.11 --target kani-latest/pyroot && \
    python3 -m pip install colorama==0.4.3 --target kani-latest/pyroot && \
    echo '[workspace]\nmembers = ["kani","kani_macros","std"]' > kani-latest/library/Cargo.toml && \
    CARGO_ENCODED_RUSTFLAGS=--cfg=kani cargo +nightly-2022-03-23 build --manifest-path kani-latest/library/kani/Cargo.toml -Z unstable-options --out-dir kani-latest/lib --target-dir target && \
    CARGO_ENCODED_RUSTFLAGS=--cfg=kani cargo +nightly-2022-03-23 build --manifest-path kani-latest/library/kani_macros/Cargo.toml -Z unstable-options --out-dir kani-latest/lib --target-dir target && \
    CARGO_ENCODED_RUSTFLAGS=--cfg=kani cargo +nightly-2022-03-23 build --manifest-path kani-latest/library/std/Cargo.toml -Z unstable-options --out-dir kani-latest/lib --target-dir target && \
    rm -rf target

# This section will be set by our shim/proxy before invoking our binaries:
ENV PATH="/tmp/kani/kani-latest/bin:/tmp/kani/kani-latest/pyroot/bin:/root/.rustup/toolchains/nightly-2022-03-23-x86_64-unknown-linux-gnu/bin:${PATH}" \
    PYTHONPATH="/tmp/kani/kani-latest/pyroot"
