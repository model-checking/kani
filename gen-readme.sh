#! /bin/sh

# Generate `README.md` from the crate documentation, plus some extra stuff.

cat readme-prologue.md >README.md
<src/lib.rs grep -E '^//!' | grep -v NOREADME | \
    sed -E 's:^//! ?::g;/```rust/s/,.*//;/ENDREADME/,$d;s/&H2/##/g;s/&NL//g' >>README.md
cat readme-antelogue.md >>README.md
