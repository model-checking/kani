#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Source kani-dependencies to get KISSAT_VERSION
source kani-dependencies

if [ -z "${KISSAT_VERSION:-}" ]; then
  echo "$0: Error: KISSAT_VERSION is not specified"
  exit 1
fi

# Check if the correct Kissat version is already installed
if command -v kissat > /dev/null; then
  if kissat_version=$(kissat --version); then
    if [[ $kissat_version == $KISSAT_VERSION ]]; then
      # Already installed
      exit 0
    else
      echo "Warning: Overriding Kissat version ${kissat_version} with ${KISSAT_VERSION}"
    fi
  fi
fi

# Kissat release
FILE="rel-${KISSAT_VERSION}.tar.gz"
URL="https://github.com/arminbiere/kissat/archive/refs/tags/$FILE"

# The release bundle ships this binary. On macOS, build it for macOS 11.0, the oldest version that rustc
# targets for both macOS targets and that the rest of the bundle supports, instead of the runner's own
# version (which compilers use by default).
if [[ "$(uname)" == "Darwin" ]]; then
  export MACOSX_DEPLOYMENT_TARGET=11.0
fi

set -x

curl -L --remote-name "$URL"
tar -xvzf $FILE
DIR_NAME="kissat-rel-${KISSAT_VERSION}"
cd $DIR_NAME
./configure --safe && make kissat && sudo install build/kissat /usr/local/bin
cd -

# Clean up on success
rm $FILE
rm -rf $DIR_NAME
