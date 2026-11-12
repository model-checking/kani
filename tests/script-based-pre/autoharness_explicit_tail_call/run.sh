#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A function with an explicit tail call (`become`) used to abort the whole autoharness run
# (https://github.com/model-checking/kani/issues/5131). It must now be skipped with a reason,
# while `ok_fn` in the same crate is still verified.
kani autoharness -Z autoharness --output-format=regular explicit_tail_call.rs
