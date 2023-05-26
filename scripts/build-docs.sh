#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Build all our documentation and place them under book/ directory.
# The user facing doc is built into book/
# RFCs are placed under book/rfc/

set -eu

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd $SCRIPT_DIR

if [ $(uname -m) = "arm64" ]; then
  if ! $(which xmdbook >/dev/null 2>&1); then
    >&2 echo "Pre-built mdbook binaries for Apple ARM are not available."
    >&2 echo 'Run `cargo install mdbook` and try again.'
    exit 1
  fi
  MDBOOK=mdbook
else
  # Download mdbook release (vs spending time building it via cargo install)
  MDBOOK_VERSION=v0.4.18
  FILE="mdbook-${MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
  URL="https://github.com/rust-lang/mdBook/releases/download/${MDBOOK_VERSION}/$FILE"
  EXPECTED_HASH="d276b0e594d5980de6a7917ce74c348f28d3cb8b353ca4eaae344ae8a4c40bea"
  if [ ! -x mdbook ]; then
      curl -sSL -o "$FILE" "$URL"
      echo "$EXPECTED_HASH $FILE" | sha256sum -c -
      tar zxf $FILE
  fi
  MDBOOK=${SCRIPT_DIR}/mdbook
fi

# Publish bookrunner report into our documentation
KANI_DIR=$SCRIPT_DIR/..
DOCS_DIR=$KANI_DIR/docs
RFC_DIR=$KANI_DIR/rfc
HTML_DIR=$KANI_DIR/build/output/latest/html/

cd $DOCS_DIR

if [ -d $HTML_DIR ]; then
    # Litani run is copied into `src` to avoid deletion by `mdbook`
    cp -r $HTML_DIR src/bookrunner/
    # Replace artifacts by examples under test
    BOOKS_DIR=$KANI_DIR/tests/bookrunner/books
    rm -r src/bookrunner/artifacts
    # Remove any json files that Kani might've left behind due to crash or timeout.
    find $BOOKS_DIR -name '*.json' -exec rm {} \;
    find $BOOKS_DIR -name '*.out' -exec rm {} \;
    cp -r $BOOKS_DIR src/bookrunner/artifacts
    # Update paths in HTML report
    python $KANI_DIR/scripts/ci/update_bookrunner_report.py src/bookrunner/index.html new_index.html
    mv new_index.html src/bookrunner/index.html

    # rm src/bookrunner/run.json
else
    echo "WARNING: Could not find the latest bookrunner run."
fi

echo "Building user documentation..."
# Build the book into ./book/
mkdir -p book
mkdir -p book/rfc
${MDBOOK} build
touch book/.nojekyll

echo "Building RFC book..."
cd $RFC_DIR
${MDBOOK} build -d $KANI_DIR/docs/book/rfc

# Testing of the code in the documentation is done via the usual
# ./scripts/kani-regression.sh script. A note on running just the
# doc tests is in README.md. We don't run them here because
# that would cause CI to run these tests twice.

# We only build the documentation for the `kani` crate.
# Note that all macros are re-exported by `kani`, so they will be included in
# the documentation.
# We also add `--cfg=kani` to the documentation, otherwise it picks up the doc
# from the wrong definitions which are empty.
echo "Building rustdocs..."
cd $KANI_DIR
RUSTFLAGS="--cfg=kani" cargo doc -p kani --no-deps --target-dir docs/book/crates

echo "Finished documentation build successfully."
