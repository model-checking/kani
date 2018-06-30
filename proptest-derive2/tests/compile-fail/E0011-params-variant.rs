#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0011]
enum T0 {
    #[proptest(params = "String")]
    V0(
        #[proptest(no_params)]
        u8
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0011]
enum T1 {
    #[proptest(params = "(u8, u8)")]
    V0 {
        #[proptest(no_params)]
        field: Vec<u8>
    },
}
