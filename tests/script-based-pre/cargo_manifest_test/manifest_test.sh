#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test if Kani can correctly check if package in the workspace when
# manifest-path presents.

cargo kani --manifest-path=add/Cargo.toml --package add --debug
