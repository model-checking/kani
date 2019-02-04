# `no_std` Support

Proptest has partial support for being used in `no_std` contexts.

You will need a nightly compiler version. In your `Cargo.toml`, adjust the
Proptest dependency to look something like this:

```toml
[dev-dependencies.proptest]
version = "proptestVersion"

# Opt out of the `std` feature
default-features = false

# alloc: Use the `alloc` crate directly. Proptest has a hard requirement on
# memory allocation, so either this or `std` is needed.
# unstable: Enable use of nightly-only compiler features.
features = ["alloc", "unstable"]
```

Some APIs are not available in the `no_std` build. This includes functionality
which necessarily needs `std` such as failure persistence and forking, as well
as features depending on other crates which do not support `no_std` usage, such
as regex support.
