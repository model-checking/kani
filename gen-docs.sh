#! /usr/bin/env bash

# This file is used to generate the docs for the `proptest` crate and deposit
# them in the appropriate place to be hosted on GH pages.
#
# Note that it uses absolute paths.

set -eux

version=$(<Cargo.toml grep -Fm1 'version = ' | cut -d\" -f2)
if test -z "$version" || echo "$version" | grep -q ' '; then
    echo "Failed to extract version"
    exit 1
fi

if test -d ~/p/misc/altsysrq.github.io/rustdoc/proptest/$version; then
    echo "Docs for this version already built?"
    exit 1
fi

cargo clean
cargo doc --no-deps
cd ~/p/misc/altsysrq.github.io/rustdoc/proptest
rm latest
sed -ri~ '/docblock/s?<p>Proptest is?<p><strong>This documentation is for an old version of proptest. <a href="../../latest/proptest">Click here</a> to see the latest version.</strong></p>\
<p>Proptest is?' */proptest/index.html
cp -a ~/p/rs/proptest/target/doc $version
ln -s $version latest
git add $version latest *.*.*/proptest/index.html
git commit -qm "Add proptest $version docs."
