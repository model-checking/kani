#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
#[proptest(1)]
struct T0;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
#[proptest(1)]
struct T1();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
#[proptest(1)]
struct T2 {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
struct T3 {
    #[proptest(1)]
    field: Vec<String>,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
struct T4(
    #[proptest(1)]
    usize,
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
#[proptest(1)]
enum T5 {
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
enum T6 {
    #[proptest(1)]
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
enum T7 {
    V0 {
        #[proptest(1)]
        foo: &'static str,
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
enum T8 {
    V0(#[proptest(1)] bool)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
enum T9 {
    #[proptest(1)]
    V0(bool),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0016]
enum T10 {
    #[proptest(1)]
    V0 { bar: bool },
}
