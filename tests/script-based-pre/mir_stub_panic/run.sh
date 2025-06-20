# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

echo "--VERIFYING panic.rs--"
RUSTFLAGS="--emit mir" kani panic.rs
echo "--READING MIR for panic.rs--"
cat panic__* | sed 's/^/ panic | /'

echo "--VERIFYING option.rs--"
RUSTFLAGS="--emit mir" kani option.rs
echo "--READING MIR for option.rs--"
cat option__* | sed 's/^/ option | /'

echo "--VERIFYING result.rs--"
RUSTFLAGS="--emit mir" kani result.rs
echo "--READING MIR for result.rs--"
cat result__* | sed 's/^/ result | /'