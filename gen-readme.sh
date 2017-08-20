#! /bin/sh

# Generate `README.md` from the crate documentation, plus some extra stuff.

echo '# Proptest' >README.md
echo >>README.md
<src/lib.rs grep -E '^//!' | grep -v NOREADME | \
    sed -E 's:^//! ?::g;/```rust/s/,.*//;/ENDREADME/,$d' >>README.md
cat readme-antelogue.md >>README.md
