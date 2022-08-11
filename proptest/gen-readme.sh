#! /bin/sh
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Modifications Copyright Kani Contributors
# See GitHub history for details.

# Generate `README.md` from the crate documentation, plus some extra stuff.

cat readme-prologue.md >README.md
cat ../book/src/intro.md \
    ../book/src/proptest/getting-started.md \
    ../book/src/proptest/vs-quickcheck.md \
    ../book/src/proptest/limitations.md | \
    grep -v NOREADME | sed 's/^#\+ /#&/' >>README.md
cat readme-antelogue.md >>README.md
