#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T0 {
    #[proptest(skip, strategy = "(0..10).prop_map(T0::V1)")]
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T1 {
    #[proptest(
        skip,
        strategy = "(0..10).prop_map(|field| T0::V1 { field })"
    )]
    V1 {
        field: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T2 {
    #[proptest(skip)]
    V1(
        #[proptest(strategy = "0..10")]
        u8
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T3 {
    #[proptest(skip)]
    V1 {
        #[proptest(strategy = "0..10")]
        field: u8
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T4 {
    #[proptest(skip, value = "T0::V1(1)")]
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T5 {
    #[proptest(skip, value = "T0::V1 { field: 3 }")]
    V1 {
        field: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T6 {
    #[proptest(skip)]
    V1(
        #[proptest(value = "42")]
        u8
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T7 {
    #[proptest(skip)]
    V1 {
        #[proptest(value = "1337")]
        field: usize
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T8 {
    #[proptest(skip)]
    V1 {
        #[proptest(value("1337"))]
        field: usize
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T9 {
    #[proptest(skip)]
    V1 {
        #[proptest(value(1337))]
        field: usize
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T10 {
    #[proptest(skip)]
    V1 {
        #[proptest(value = 1337)]
        field: usize
    },
}
