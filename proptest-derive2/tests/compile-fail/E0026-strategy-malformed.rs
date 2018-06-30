#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T1 {
    V1 {
        #[proptest(strategy)]
        batman: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T3 {
    #[proptest(strategy("///"))]
    field: usize,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T4(
    #[proptest(strategy)]
    usize,
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
enum T5 {
    V1 {
        #[proptest(value)]
        batman: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T6 {
    #[proptest(value)]
    field: usize,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0026]
struct T7(
    #[proptest(value)]
    usize,
);
