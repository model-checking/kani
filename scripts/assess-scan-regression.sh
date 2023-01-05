#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..

echo "Running assess scan test:"

cd $KANI_DIR/tests/assess-scan-test-scaffold
cargo kani --enable-unstable assess scan

# Clean up
(cd foo && cargo clean)
(cd bar && cargo clean)

# Check for expected files (and clean up)
EXPECTED_FILES=(
    bar/bar.kani-assess-metadata.json
    foo/foo.kani-assess-metadata.json
    bar/bar.kani-assess.log
    foo/foo.kani-assess.log
)
for file in ${EXPECTED_FILES[@]}; do
  if [ -f $KANI_DIR/tests/assess-scan-test-scaffold/$file ]; then
    rm $KANI_DIR/tests/assess-scan-test-scaffold/$file
  else
    echo "Failed to find $file" && exit 1
  fi
done

echo "Done with assess scan test"
