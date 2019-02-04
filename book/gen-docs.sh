#! /usr/bin/env bash

# This file is used to generate the Proptest Book and deposit it in the
# appropriate place to be hosted on GH pages.
#
# Note that it uses absolute paths.

set -eux

mdbook build
cd ~/p/misc/altsysrq.github.io/
git rm -rf proptest-book
rm -rf proptest-book
cp -a ~/p/rs/proptest/book/book proptest-book
git add proptest-book
git commit -qm 'Update proptest-book.'
