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

# Publish bookrunner report into our documentation
KANI_DIR=$SCRIPT_DIR/..
HTML_DIR=$KANI_DIR/build/output/latest/html/

if [ -d $HTML_DIR ]; then
    # Litani run is copied into `src` to avoid deletion by `mdbook`
    cp -r $HTML_DIR src/bookrunner/
    # Replace artifacts by examples under test
    BOOKS_DIR=$KANI_DIR/src/test/bookrunner/books
    rm -r src/bookrunner/artifacts
    cp -r $BOOKS_DIR src/bookrunner/artifacts
    # Update paths in HTML report
    python $KANI_DIR/scripts/ci/update_bookrunner_report.py src/bookrunner/index.html new_index.html
    mv new_index.html src/bookrunner/index.html

    # rm src/bookrunner/run.json
else
    echo "WARNING: Could not find the latest bookrunner run."
fi

# Build the book into ./book/
mkdir -p book
./mdbook build
touch book/.nojekyll

# Testing of the code in the documentation is done via the usual
# ./scripts/kani-regression.sh script. A note on running just the
# doc tests is in README.md. We don't run them here because
# that would cause CI to run these tests twice.

echo "Finished documentation build successfully."
