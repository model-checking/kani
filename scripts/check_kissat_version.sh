#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check if kissat has the minimum required version specified in the
# `kani_dependencies` file under kani's root folder

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..
source "${KANI_DIR}/kani-dependencies"

if [ -z "${KISSAT_VERSION:-}" ]; then
  echo "$0: ERROR: KISSAT_VERSION is not set"
  exit 1
fi
cmd="kissat --version"
if kissat_version=$($cmd); then
  # Perform a lexicographic comparison of the version
  if [[ $kissat_version < $KISSAT_VERSION ]]; then
    echo "ERROR: Kissat version is $kissat_version. Expected at least $KISSAT_VERSION."
    exit 1
  fi
else
  echo "ERROR: Couldn't run command '$cmd'"
  exit 1
fi
