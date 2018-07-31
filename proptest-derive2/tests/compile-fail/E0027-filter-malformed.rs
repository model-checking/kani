#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0027]
                            //~| [proptest_derive, E0008]
#[proptest(filter)]
#[proptest(skip)]
struct NonFatal;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
struct T0 {
    #[proptest(filter)]
    field: usize,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
struct T1(
    #[proptest(filter)]
    usize,
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T2 {
    V1 {
        #[proptest(filter)]
        batman: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T3 {
    V1(
        #[proptest(filter)]
        u8
    )
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T4 {
    #[proptest(filter)]
    V1 {
        batman: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
enum T5 {
    #[proptest(filter)]
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter)]
enum T6 {
    V1 {
        batman: u8
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter)]
enum T7 {
    V1(u8)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter = 1)]
struct T8(u8);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0027]
#[proptest(filter(1))]
struct T9(u8);