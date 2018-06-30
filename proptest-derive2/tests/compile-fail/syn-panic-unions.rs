#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ proc-macro derive panicked
union Foo { x: usize }

// FIXME: Should be E0002.
// https://github.com/dtolnay/syn/issues/447
