#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A `rustc` wrapper cannot serve `kani-compiler`, so `cargo kani` must not use one.
# See https://github.com/model-checking/kani/issues/2233.

set -eu

MARKER="$(pwd)/wrapper-was-called"
WRAPPER="$(pwd)/fake-wrapper.sh"

rm -f "${MARKER}"

# Records the compiler it was asked to wrap, then behaves like a well-mannered wrapper and execs it.
# A real wrapper (sccache) instead rejects `kani-compiler` outright and fails the build. Cargo also
# wraps plain `rustc` for its target probes, which is fine and expected -- only `kani-compiler`
# reaching a wrapper is the bug.
cat > "${WRAPPER}" <<WRAPPER_EOF
#!/usr/bin/env bash
case "\$1" in
  *kani-compiler) echo "\$1" >> "${MARKER}" ;;
esac
exec "\$@"
WRAPPER_EOF
chmod +x "${WRAPPER}"

# Each case starts from a clean build: a crate that is already up to date is not recompiled, so
# a wrapper would go unnoticed.
fresh() {
    cargo clean --quiet
}

check_marker() {
    if [ -e "${MARKER}" ]; then
        echo "FAILURE ($1): kani-compiler was invoked through the rustc wrapper"
        rm -f "${MARKER}"
    else
        echo "SUCCESS ($1): kani-compiler was not invoked through the rustc wrapper"
    fi
}

echo "--- wrapper set via RUSTC_WRAPPER"
fresh
RUSTC_WRAPPER="${WRAPPER}" cargo kani
check_marker "RUSTC_WRAPPER"

echo "--- wrapper set via CARGO_BUILD_RUSTC_WRAPPER"
fresh
CARGO_BUILD_RUSTC_WRAPPER="${WRAPPER}" cargo kani
check_marker "CARGO_BUILD_RUSTC_WRAPPER"

# The setup the sccache documentation recommends.
echo "--- wrapper set via .cargo/config.toml"
fresh
mkdir -p .cargo
printf '[build]\nrustc-wrapper = "%s"\n' "${WRAPPER}" > .cargo/config.toml
cargo kani
check_marker ".cargo/config.toml"

rm -rf .cargo "${WRAPPER}" "${MARKER}"
