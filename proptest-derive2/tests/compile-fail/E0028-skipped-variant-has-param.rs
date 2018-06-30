#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T0 {
    #[proptest(skip, no_params)]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T1 {
    #[proptest(skip, no_params)]
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T2 {
    #[proptest(skip, no_params)]
    V1 {
        field: String,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T3 {
    #[proptest(skip, params = "u8")]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T4 {
    #[proptest(skip, params = "u8")]
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T5 {
    #[proptest(skip, params = "u8")]
    V1 {
        field: String,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T6 {
    #[proptest(skip)]
    V1(
        #[proptest(params = "u8")]
        u8
    )
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T7 {
    #[proptest(skip)]
    V1 {
        #[proptest(params = "u8")]
        field: String,
    }
}
