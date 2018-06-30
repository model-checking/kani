#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ 2 errors:
                            //~| [proptest_derive, E0015]
                            //~| [proptest_derive, E0008]
#[proptest = 1]
#[proptest(skip)]
struct T0;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
#[proptest = 1]
struct T1();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
#[proptest = 1]
struct T2 {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
struct T3 {
    #[proptest = 1]
    field: Vec<String>,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
struct T4(
    #[proptest = 1]
    usize,
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
#[proptest = 1]
enum T5 {
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
enum T6 {
    #[proptest = 1]
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
enum T7 {
    V0 {
        #[proptest = 1]
        foo: &'static str,
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
enum T8 {
    V0(#[proptest = 1] bool)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
enum T9 {
    #[proptest = 1]
    V0(bool),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0015]
enum T10 {
    #[proptest = 1]
    V0 { bar: bool },
}
