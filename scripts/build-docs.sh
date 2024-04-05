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

KANI_DIR=$SCRIPT_DIR/..
DOCS_DIR=$KANI_DIR/docs
RFC_DIR=$KANI_DIR/rfc

cd $DOCS_DIR

echo "Building user documentation..."
# Generate benchcomp documentation from source code
mkdir -p gen_src
"${SCRIPT_DIR}/gen_benchcomp_schemas.py" gen_src

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
# We remove build files to avoid false positives in secret scanning alerts.
# More details in: https://github.com/model-checking/kani/issues/2735
echo "Removing build files from rustdocs..."
rm -r docs/book/crates/debug

echo "Finished documentation build successfully."
