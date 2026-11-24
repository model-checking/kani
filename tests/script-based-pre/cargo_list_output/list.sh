#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check where `cargo kani list` writes its results (#5124):
# - by default, to `<target-dir>/kani/`, wherever the shell is,
# - with `--output <DIR>`, to that directory, which Kani creates if needed.
# No run may leave a `kani-list.*` file in the package root or in `src/`.

set -eu
rm -rf kani-list.json kani-list.md src/kani-list.json target/list-output target/other

(cd src && cargo kani list --format json)
cargo kani --target-dir target/other list --format json
cargo kani list --format json --output target/list-output
cargo kani list --format markdown --output target/list-output

for stray_file in kani-list.json kani-list.md src/kani-list.json; do
    if [ -e "$stray_file" ]; then
        echo "Test failed: $stray_file was written outside the target directory"
        exit 1
    fi
done

cat target/kani/kani-list.json target/list-output/kani-list.json target/list-output/kani-list.md
