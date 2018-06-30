#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T0 {
    V1 {
        #[proptest(strategy = "random $ § § 21 garbage")]
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T1 {
    V1(
        #[proptest(strategy = "random $ § § 21 garbage")]
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T2 {
    #[proptest(strategy = "random $ § § 21 garbage")]
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T3(
    #[proptest(strategy = "random $ § § 21 garbage")]
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T4 {
    V1 {
        #[proptest(value = "random $ § § 21 garbage")]
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T5 {
    V1(
        #[proptest(value = "random $ § § 21 garbage")]
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T6 {
    #[proptest(value = "random $ § § 21 garbage")]
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T7(
    #[proptest(value = "random $ § § 21 garbage")]
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T8 {
    V1 {
        #[proptest(strategy("random $ § § 21 garbage"))]
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T9 {
    V1(
        #[proptest(strategy("random $ § § 21 garbage"))]
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T10 {
    #[proptest(strategy("random $ § § 21 garbage"))]
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T11(
    #[proptest(strategy("random $ § § 21 garbage"))]
    String
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T12 {
    V1 {
        #[proptest(value("random $ § § 21 garbage"))]
        field: u8,
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T13 {
    V1(
        #[proptest(value("random $ § § 21 garbage"))]
        u8,
    ),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T14 {
    #[proptest(value("random $ § § 21 garbage"))]
    field: String,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T15(
    #[proptest(value("random $ § § 21 garbage"))]
    String
);
