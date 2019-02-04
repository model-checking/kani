#! /usr/bin/env bash

# This file is used to generate the docs for the `proptest` crate and deposit
# them in the appropriate place to be hosted on GH pages.
#
# Note that it uses absolute paths.

set -eux

if test "$1" = "nostd"; then
    crate=proptest-nostd
    cargo='cargo +nightly'
    cargoflags='--no-default-features --features=alloc,unstable'
else
    crate=proptest
    cargo=cargo
    cargoflags=''
fi

version=$(<Cargo.toml grep -Fm1 'version = ' | cut -d\" -f2)
if test -z "$version" || echo "$version" | grep -q ' '; then
    echo "Failed to extract version"
    exit 1
fi

if test -d ~/p/misc/altsysrq.github.io/rustdoc/$crate/$version; then
    echo "Docs for this version already built?"
    exit 1
fi

$cargo clean
$cargo doc --no-deps $cargoflags
mkdir -p ~/p/misc/altsysrq.github.io/rustdoc/$crate
cd ~/p/misc/altsysrq.github.io/rustdoc/$crate
if test -e latest; then
    rm latest
    sed -ri~ '/docblock/s?<p>Proptest is?<p><strong>This documentation is for an old version of proptest. <a href="../../latest/proptest">Click here</a> to see the latest version.</strong></p>\
<p>Proptest is?' */$crate/index.html
fi
cp -a ~/p/rs/proptest/target/doc $version
ln -s $version latest
git add $version latest *.*.*/proptest/index.html
git commit -qm "Add $crate $version docs."
