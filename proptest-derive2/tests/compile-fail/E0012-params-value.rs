#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0012]
enum T0 {
    #[proptest(params = "String", value = "params")]
    V0(u8),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0012]
enum T1 {
    V0 {
        #[proptest(params = "String", value = "params")]
        field: u8
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0012]
enum T2 {
    #[proptest(params = "String", value = "params")]
    V0(u8),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0012]
enum T3 {
    V0(
        #[proptest(params = "String", value = "params")]
        Vec<u8>,
    )
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0012]
struct T4 {
    #[proptest(params = "String", value = "params")]
    field: Vec<u8>,
}
