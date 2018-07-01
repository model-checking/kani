#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0007]
                            //~| [proptest_derive, E0030]
#[proptest(params = "u8")]
#[proptest(strategy = "1u8..")]
struct A {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct B;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct C();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct D { field: String }

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
struct E(Vec<u8>);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0007]
#[proptest(strategy = "1u8..")]
enum F { V1, V2, }
