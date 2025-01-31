#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# This script automates the process of checking out several git repos, then running
# 'cargo kani assess scan' on them (with '--only-codegen').

# Usage:
#      ./scripts/exps/assess-scan-on-repos.sh
# Or (will clone in ~):
#      ASSESS_SCAN="~/top-100-experiment" ./scripts/exps/assess-scan-on-repos.sh

# To use a different set of packages, the script needs updating (below).

set -eu

THIS_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
# Remove 'scripts/exps/'
KANI_DIR=$THIS_SCRIPT_DIR/../..

# The target repos and crates to analyze
REPO_FILE="$KANI_DIR/tests/remote-target-lists/top-100-crates-2022-12-26.txt"
NAME_FILE="$KANI_DIR/tests/remote-target-lists/top-100-crate-names-2022-12-26.txt"

# Where should we clone the repos?
# 1. If 'ASSESS_SCAN' environment variable is set, use that directory
if [ ${ASSESS_SCAN:-unset} != "unset" ]; then
    mkdir -p $ASSESS_SCAN
    cd $ASSESS_SCAN
# 2. If the current working directory happens to be named 'top-100-experiment' let's do it here
elif [ "${PWD##*/}" == "top-100-experiment" ]; then
    true # nothing to do
# 3. Not finding any other direction, default to doing it in a directory in /tmp
else
    mkdir -p /tmp/top-100-experiment
    cd /tmp/top-100-experiment
fi

echo "Cloning repos into ${PWD}"

REPOS=$(cat $REPO_FILE)
for repo in $REPOS; do
    # The directory that gets checked out is the final name in the url
    dir=${repo##*/}
    # Sometimes there's a '.git' after the end, strip that off
    dir=${dir%.git}
    if [ -d $dir ]; then
        echo "Updating $dir..."
        (cd $dir && git pull)
    else
        echo "Cloning $dir..."
        git clone $repo
    fi
done

# Use release mode to speed up the run.
echo "Build kani on release mode..."
pushd ${KANI_DIR}
cargo build-dev --release
popd

echo "Starting assess scan..."

time cargo kani --only-codegen -Z unstable-options assess scan \
  --filter-packages-file $NAME_FILE \
  --emit-metadata ./scan-results.json

echo "Complete."
