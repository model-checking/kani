#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0029]
enum T0 {
    #[proptest(no_params)]
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0029]
enum T1 {
    #[proptest(params = "u8")]
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0029]
enum T2 {
    #[proptest(no_params)]
    V0 {},
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0029]
enum T3 {
    #[proptest(params = "u8")]
    V0 {},
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0029]
enum T4 {
    #[proptest(no_params)]
    V0(),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0029]
enum T5 {
    #[proptest(params = "u8")]
    V0(),
}
