#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T0 {
    #[proptest(skip, weight = 2)]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T1 {
    #[proptest(skip, w = 3)]
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T2 {
    #[proptest(skip, w = 3)]
    V1 {
        field: String,
    }
}
