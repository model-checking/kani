# The `proptest-derive` crate

The `proptest-derive` crate provides a procedural macro,
`#[derive(Arbitrary)]`, which can be used to automatically generate simple
`Arbitrary` implementations for user-defined types, allowing them to be used
with `any()` and embedded in other `#[derive(Arbitrary)]` types without fuss.

It is recommended to have a basic working understanding of the [`proptest`
crate](/proptest/index.md) before getting into this part of the
documentation.

**This crate is currently somewhat experimental.** Expect rough edges,
particularly in documentation. It is also more likely to see releases with
breaking changes than the main `proptest` crate.
