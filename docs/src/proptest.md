# (Experimental) Running Proptests with Kani

**NOTE** This feature is purely experimental, and we do not guarentee
any support. Furthermore, many features that are implemented in
`proptest` are missing and your proptests may not compile.

An experimental feature allows Kani to run Property-Based Tests
written using the [proptest](https://crates.io/crates/proptest)
crate. This feature is supported only for `cargo kani`.

No special annotations are required, but you do need to adjust
`Cargo.toml` so that the proptest import does not conflict with the
one provided by kani. The easiest way to do this will be to put the
proptest import under `[target.'cfg(not(kani))'.dependencies]`.

### Under the Hood: How Kani's `proptest` feature works.

Under the hood, the `proptest` feature works hijacking the import of
the proptest library. This is achieved by with 2 components
1. Proptest proptest import `[target.'cfg(not(kani))'.dependencies]`
   makes the original import invisible to Kani, which runs the
   configuration parameter `kani`.
2. Add in our custom proptest through `-L` and `--extern` while making
   sure to link `proptest` symtabs by adding them to the glob in
   `kani-driver/src/call_cargo.rs`
