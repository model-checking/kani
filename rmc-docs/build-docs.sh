#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd $SCRIPT_DIR

# Download mdbook release (vs spending time building it via cargo install)
FILE="mdbook-v0.4.12-x86_64-unknown-linux-gnu.tar.gz"
URL="https://github.com/rust-lang/mdBook/releases/download/v0.4.12/$FILE"
EXPECTED_HASH="2a0953c50d8156e84f193f15a506ef0adbac66f1942b794de5210ca9ca73dd33"
if [ ! -x mdbook ]; then
    curl -sSL -o "$FILE" "$URL"
    echo "$EXPECTED_HASH $FILE" | sha256sum -c -
    tar zxf $FILE
fi

# Build the book into ./book/
mkdir -p book
./mdbook build
touch book/.nojekyll

# TODO: Test all the code examples from our documentation
# TODO: Build the dashboard and publish into our documentation

echo "Finished documentation build successfully."
