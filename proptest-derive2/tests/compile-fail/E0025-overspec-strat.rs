#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
#[proptest(value = "T0(0)", strategy = "(0..6).prop_map(T1)")]
struct T0(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
struct T1 {
    #[proptest(value = "1", strategy = "(0..1).prop_map(T1)")]
    field: u8
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
struct T2(
    #[proptest(value = "1", strategy = "(0..1).prop_map(T1)")]
    u8
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
enum T3 {
    V0 {
        #[proptest(value = "1", strategy = "0..1")]
        field: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0025]
enum T4 {
    V0(
        #[proptest(value = "1", strategy = "0..1")]
        u8
    ),
}
