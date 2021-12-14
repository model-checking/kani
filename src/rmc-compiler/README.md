This is a temporary wrapper that can be used to compiler rust into gotoc. This
binary should not be used on its own and it should be used via `rmc` or
`cargo-rmc` commands.

To build:

```
RUSTC_INSTALL_BINDIR="<bin_folder>" RUST_CHECK=1 CFG_RELEASE=<number> CFG_RELEASE_CHANNEL=nightly cargo +nightly-<same-version> build
```

TODO: Figure out how to use the libraries compiled by ./x.py

