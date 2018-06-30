#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0020]
#[proptest(skip = 1)]
struct T0(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0020]
#[proptest(skip(2))]
struct T1(u8);
