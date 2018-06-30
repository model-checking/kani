#[macro_use]
extern crate proptest_derive;

// Show non fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0020]
                            //~| [proptest_derive, E0007]
#[proptest(skip = 1, value = "T0(1)")]
struct T0(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0020]
#[proptest(skip(2))]
struct T1(u8);
