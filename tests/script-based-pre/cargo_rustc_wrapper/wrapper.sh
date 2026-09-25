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

RUSTC_WRAPPER="${WRAPPER}" cargo kani

if [ -e "${MARKER}" ]; then
    echo "FAILURE: kani-compiler was invoked through the rustc wrapper"
else
    echo "SUCCESS: kani-compiler was not invoked through the rustc wrapper"
fi

rm -f "${WRAPPER}" "${MARKER}"
