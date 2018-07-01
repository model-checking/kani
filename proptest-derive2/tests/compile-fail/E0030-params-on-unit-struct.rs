#[macro_use]
extern crate proptest_derive;

// It happens that no other error will follow E0030 so this is not as proper
// a check that we wanted to ensure that E0030 is non-fatal.

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors
                            //~| [proptest_derive, E0008]
                            //~| [proptest_derive, E0030]
#[proptest(params = "u8")]
#[proptest(skip)]
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
