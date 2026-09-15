#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

echo
echo "Starting output file check..."
echo

# Test for platform
PLATFORM=$(uname -sp)
case "$PLATFORM" in
  "Linux x86_64" | "Darwin i386" | "Darwin arm") ;;
  *)
    echo
    echo "Test only works on Linux or OSX platforms, skipping..."
    echo
    exit 0
    ;;
esac

export RUST_BACKTRACE=1
cd $(dirname $0)

echo "Running single-file check..."
rm -rf *.c
kani --gen-c -Z unstable-options singlefile.rs >& kani.log || \
    { ret=$?; echo "== Failed to run Kani"; cat kani.log; rm kani.log; exit 1; }
rm -f kani.log
if ! [ -e singlefile_*main.c ]
then
    echo "Error: no GotoC file generated. Expected: singlefile_*main.c"
    exit 1
fi

if ! [ -e singlefile_*main.demangled.c ]
then
    echo "Error: no demangled GotoC file generated. Expected singlefile_*main.demangled.c."
    exit 1
fi

echo "Checking that demangling works as expected..."

declare -a PATTERNS=(
    'struct PrettyStruct pretty_function(struct PrettyStruct' # expected demangled struct and function name
    'monomorphize::<usize>(' # monomorphized function name
    'struct ()' # pretty-printed unit struct
    'struct &str' # pretty-printed reference type
    'TestEnum::Variant1' # pretty-printed variant
)

for val in "${PATTERNS[@]}"; do
    if ! grep -Fq "$val" singlefile_*main.demangled.c;
    then
        echo "Error: demangled file singlefile_*main.demangled.c did not contain expected pattern '$val'."
        exit 1
    fi
done

echo "Finished single-file check successfully..."
echo

(cd multifile
echo "Running multi-file check..."
rm -rf build
cargo kani --target-dir build --gen-c -Z unstable-options >& kani.log || \
    { ret=$?; echo "== Failed to run Kani"; cat kani.log; rm kani.log; exit 1; }
rm -f kani.log

# The generated C sits next to the build artifacts, at a path under `--target-dir` that cargo
# picks and has changed before (cargo 1.99 moved artifacts out of `debug/deps`), so search the
# target directory rather than hardcoding the layout.
mangled=$(find build -name 'multifile*main.c' -print -quit)
if [ -z "${mangled}" ]
then
    echo "Error: no GotoC file found under build/. Expected: multifile*main.c"
    exit 1
fi

demangled=$(find build -name 'multifile*main.demangled.c' -print -quit)
if [ -z "${demangled}" ]
then
    echo "Error: no demangled GotoC file found under build/. Expected: multifile*main.demangled.c"
    exit 1
fi

if ! grep -Fq "struct PrettyStruct pretty_function(struct PrettyStruct" "${demangled}";
then
    echo "Error: demangled file ${demangled} did not contain expected demangled struct and function name."
    exit 1
fi
echo "Finished multi-file check successfully..."
)

echo "Finished output file check successfully."
echo
