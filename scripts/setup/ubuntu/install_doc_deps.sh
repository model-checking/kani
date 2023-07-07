#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

cargo install mdbook-graphviz
DEBIAN_FRONTEND=noninteractive sudo apt-get install --no-install-recommends --yes graphviz

PYTHON_DEPS=(
  json_schema_for_humans  # Render a Draft-07 JSON schema to HTML
  schema                  # Validate data structures
)

python3 -m pip install "${PYTHON_DEPS[@]}"
