#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Test building a crate that has the Kani library as a dependency

set -e

rm -rf target

set -e
cargo build

rm -rf target
