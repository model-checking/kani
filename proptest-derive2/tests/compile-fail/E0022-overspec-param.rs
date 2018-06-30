#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0022]
#[proptest(no_params, params = "u8")]
enum T0 {
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0022]
#[proptest(no_params, params = "u8")]
struct T1 {
    field: String,
}
