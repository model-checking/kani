#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Fn-bounded type parameters instantiate with nondeterministic function items
# (fresh nondet result per call = over-approximation of every closure); Iterator-bounded
# ones with std::vec::IntoIter over unbounded nondeterministic vectors.
cargo kani autoharness -Z autoharness --output-format=regular
