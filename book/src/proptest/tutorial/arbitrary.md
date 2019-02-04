# Defining a canonical `Strategy` for a type

We previously used the function `any` as in `any::<u32>()` to generate a
strategy for all `u32`s. This function works with the trait `Arbitrary`,
which QuickCheck users may be familiar with. In proptest, this trait
is already implemented for most owned types in the standard library,
but you can of course implement it for your own types.

In some cases, where it makes sense to define a canonical strategy, such as in
the [JSON AST example](recursive.md), it is a good idea to implement
`Arbitrary`.

The experimental [`proptest-derive` crate](../../proptest-derive/index.md) can
be used to automate implementing `Arbitrary` in common cases.
