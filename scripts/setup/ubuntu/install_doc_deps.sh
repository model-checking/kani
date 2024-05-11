#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

cargo install mdbook-graphviz
DEBIAN_FRONTEND=noninteractive sudo apt-get install --no-install-recommends --yes graphviz
