#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T0 {
    V1 {
        #[proptest(filter = "random garbage")]
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T1 {
    V1(
        #[proptest(filter = "random garbage")]
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
struct T2 {
    #[proptest(filter = "random garbage")]
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
struct T3(
    #[proptest(filter = "random garbage")]
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T4 {
    V1 {
        #[proptest(filter("random garbage"))]
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T5 {
    V1(
        #[proptest(filter("random garbage"))]
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
struct T6 {
    #[proptest(filter("random garbage"))]
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
struct T7(
    #[proptest(filter("random garbage"))]
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T8 {
    #[proptest(filter = "random garbage")]
    V1 {
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T9 {
    #[proptest(filter = "random garbage")]
    V1(
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter = "random garbage")]
struct T10 {
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter = "random garbage")]
struct T11(
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T12 {
    #[proptest(filter("random garbage"))]
    V1 {
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T13 {
    #[proptest(filter("random garbage"))]
    V1(
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter("random garbage"))]
struct T14 {
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter("random garbage"))]
struct T15(
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter = "random garbage")]
enum T16 {
    V1 {
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter = "random garbage")]
enum T17 {
    V1(
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter("random garbage"))]
enum T18 {
    V1 {
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter("random garbage"))]
enum T19 {
    V1(
        u8,
    ),
}