#[macro_use]
extern crate proptest_derive;

struct T0;

#[derive(Debug, Arbitrary)] //~ Arbitrary` is not satisfied [E0277]
struct T1 { f0: T0, }
