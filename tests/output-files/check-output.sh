#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

echo
echo "Starting output file check..."
echo

# Test for platform
PLATFORM=$(uname -sp)
if [[ $PLATFORM == "Linux x86_64" ]]
then
  TARGET="x86_64-unknown-linux-gnu"
elif [[ $PLATFORM == "Darwin i386" ]]
then
  TARGET="x86_64-apple-darwin"
elif [[ $PLATFORM == "Darwin arm" ]]
then
  TARGET="aarch64-apple-darwin"
else
  echo
  echo "Test only works on Linux or OSX platforms, skipping..."
  echo
  exit 0
fi

cd $(dirname $0)

echo "Running single-file check..."
rm -rf *.c
RUST_BACKTRACE=1 kani --gen-c --enable-unstable singlefile.rs --quiet
if ! [ -e singlefile.for-main.c ]
then
    echo "Error: no GotoC file generated. Expected: singlefile.for-main.c"
    exit 1
fi

if ! [ -e singlefile.for-main.demangled.c ]
then
    echo "Error: no demangled GotoC file generated. Expected singlefile.for-main.demangled.c."
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
    if ! grep -Fq "$val" singlefile.for-main.demangled.c;
    then
        echo "Error: demangled file singlefile.for-main.demangled.c did not contain expected pattern '$val'."
        exit 1
    fi
done

echo "Finished single-file check successfully..."
echo

(cd multifile
echo "Running multi-file check..."
rm -rf build
RUST_BACKTRACE=1 cargo kani --target-dir build --gen-c --enable-unstable --quiet
cd build/kani/${TARGET}/debug/deps/

mangled=$(ls multifile*.for-main.c)
if ! [ -e "${mangled}" ]
then
    echo "Error: no GotoC file found. Expected: build/${TARGET}/debug/deps/multifile*.for-main.c"
    exit 1
fi

demangled=$(ls multifile*.for-main.demangled.c)
if ! [ -e "${demangled}" ]
then
    echo "Error: no demangled GotoC file found. Expected build/${TARGET}/debug/deps/multifile*.for-main.demangled.c."
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
