#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Install cbmc-viewer

# Source kani-dependencies to get CBMC_VIEWER_VERSION
source kani-dependencies

if [ -z "${CBMC_VIEWER_VERSION:-}" ]; then
  echo "$0: Error: CBMC_VIEWER_VERSION is not specified"
  exit 1
fi

set -x

# brew doesn't recognize specific versions of viewer
# Build from source, since there's only a macos-12 bottle which doesn't seem to work.
# Install Python 3.12 first while ignoring errors: the system may provide this
# version, which will hinder brew from installing symlinks
brew install python@3.12 || true
brew install -s aws/tap/cbmc-viewer
echo "Installed: $(cbmc-viewer --version)"
