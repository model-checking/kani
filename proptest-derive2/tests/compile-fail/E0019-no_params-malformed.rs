#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0019]
                            //~| [proptest_derive, E0007]
#[proptest(no_params = 1, value("T0(u8)"))]
struct T0(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0019]
#[proptest(no_params(2))]
struct T1(u8);
