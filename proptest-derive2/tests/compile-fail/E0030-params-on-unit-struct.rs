#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0030]
#[proptest(params = "u8")]
struct T0;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0030]
#[proptest(no_params)]
struct T1;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0030]
#[proptest(params = "u8")]
struct T2 {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0030]
#[proptest(no_params)]
struct T3 {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0030]
#[proptest(params = "u8")]
struct T4();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0030]
#[proptest(no_params)]
struct T5();
