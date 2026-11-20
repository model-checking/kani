#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Values generated for types with layout niches (rustc_layout_scalar_valid_range, as used by
# std's NonZero and core::time::Nanoseconds) must respect the niche: it is a language-level
# validity invariant. days_left_in_year previously failed on out-of-niche months; the covers
# check the assumption does not over-constrain.
kani autoharness -Z autoharness --output-format=regular niche_probe.rs
