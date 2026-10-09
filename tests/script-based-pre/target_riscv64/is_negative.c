// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// `char` is unsigned on riscv64 and signed on x86_64, so goto-cc compiling this for the host would
// give the wrong answer for a riscv64 harness. target_riscv64.sh checks that Kani refuses it.

int is_negative(char c) { return c < 0; }
