#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

echo "--- functions over SVE tuples get harnesses and compile"
echo "--- passing a tuple through verifies"
echo "--- an unsupported SVE operation fails its harness, not the run"

# SVE is only available on aarch64.
case "$(uname -m)" in
    aarch64 | arm64) ;;
    *) exit 0 ;;
esac

kani autoharness -Z autoharness sve_tuple.rs --output-format=terse > out.log 2>&1 || true

grep -qE '^\| sve_tuple +\| pass_x2 +\| #\[kani::proof\] +\| Success' out.log
grep -qE '^\| sve_tuple +\| pass_x4 +\| #\[kani::proof\] +\| Success' out.log
grep -q 'sve_tuple_get is not currently supported by Kani' out.log
grep -qE '^\| sve_tuple +\| first +\| #\[kani::proof\] +\| Failure' out.log
# The load reaches no further than its predicate, the first LLVM intrinsic it calls.
grep -q 'call to LLVM intrinsic `llvm.aarch64.sve.ld4.sret.nxv4i32` (1)' out.log
grep -q 'call to LLVM intrinsic `llvm.aarch64.sve.ptrue.nxv4i1` is not currently supported' out.log
grep -qE '^\| sve_tuple +\| load +\| #\[kani::proof\] +\| Failure' out.log
if grep -qE 'panicked|internal compiler error' out.log; then
    echo "unexpected compiler crash"
    exit 1
fi

rm -f out.log
