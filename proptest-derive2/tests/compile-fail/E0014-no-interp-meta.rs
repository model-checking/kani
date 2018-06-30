#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0024]
                            //~| [proptest_derive, E0008]
#[proptest ~~~]
#[proptest(skip)]
struct T0;
