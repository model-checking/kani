#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that `cargo kani list --output <DIR>` creates the directory, writes the
# results into it, and leaves no `kani-list.*` file in the current directory (#5124).

set -eu
rm -rf kani-list.json kani-list.md target/list-output

cargo kani list --format json --output target/list-output
cargo kani list --format markdown --output target/list-output

for default_file in kani-list.json kani-list.md; do
    if [ -e "$default_file" ]; then
        echo "Test failed: $default_file was written to the current directory"
        exit 1
    fi
done

cat target/list-output/kani-list.json target/list-output/kani-list.md
