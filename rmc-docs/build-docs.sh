#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd $SCRIPT_DIR

# Download mdbook release (vs spending time building it via cargo install)
FILE="mdbook-v0.4.12-x86_64-unknown-linux-gnu.tar.gz"
URL="https://github.com/rust-lang/mdBook/releases/download/v0.4.12/$FILE"
if [ ! -x mdbook ]; then
    wget -O "$FILE" "$URL"
    tar zxvf $FILE
fi

# Build the book into ./book/
mkdir -p book
./mdbook build
touch book/.nojekyll

# TODO: Test all the code examples from our documentation
# TODO: Build the dashboard and publish into our documentation

echo "Finished documentation build successfully."
