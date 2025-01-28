#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
KANI_DIR=$SCRIPT_DIR/..

echo "Running assess scan test:"

cd $KANI_DIR/tests/assess-scan-test-scaffold
cargo kani -Z unstable-options assess scan

# Clean up
(cd foo && cargo clean)
(cd bar && cargo clean)
(cd compile_error && cargo clean)
(cd manifest_error && cargo clean)

# Check for expected files (and clean up)
EXPECTED_FILES=(
    bar/bar.kani-assess.log
    bar/bar.kani-assess-metadata.json
    compile_error/compile_error.kani-assess.log
    compile_error/compile_error.kani-assess-metadata.json
    manifest_error/manifest_error.kani-assess.log
    manifest_error/manifest_error.kani-assess-metadata.json
    foo/foo.kani-assess.log
    foo/foo.kani-assess-metadata.json
)

errors=0
for file in ${EXPECTED_FILES[@]}; do
  if [ -f $KANI_DIR/tests/assess-scan-test-scaffold/$file ]; then
    rm $KANI_DIR/tests/assess-scan-test-scaffold/$file
  else
    errors=1
    echo "Failed to find $file"
  fi
done

echo "Done with assess scan test"
exit $errors
