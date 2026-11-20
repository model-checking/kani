#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Downcasting into a variant of an uninhabited enum used to crash the compiler and abort the
# whole run (https://github.com/model-checking/kani/issues/3596). It must now only fail the
# harness for `trigger`, while `ok_fn` in the same crate is still verified.
kani autoharness -Z autoharness --output-format=regular uninhabited_enum_downcast.rs
