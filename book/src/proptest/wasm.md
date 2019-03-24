# Web Assembly support

As of 0.9.2, it is possible to compile proptest on `wasm` targets. Please note
that this is **highly experimental** and has not been subject to any
substantial amount of testing.

In `cargo.toml`, write something like

```toml
[dev-dependencies.proptest]
version = "$proptestVersion"
# The default feature set includes things like process forking which are not
# supported in Web Assembly.
default-features = false
# Enable using the `std` crate.
features = ["std"]
```

A few APIs are unavailable on `wasm` targets (beyond those which are removed by
deselecting certain default features):

- Numeric strategies for `i128` and `u128`.

- The `Arbitrary` implementation for `std::env::VarError`.
