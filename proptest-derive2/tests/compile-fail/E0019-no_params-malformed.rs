#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0019]
#[proptest(no_params = 1)]
struct T0(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0019]
#[proptest(no_params(2))]
struct T1(u8);
