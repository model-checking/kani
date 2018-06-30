#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
#[proptest]
struct T0;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
#[proptest]
struct T1();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
#[proptest]
struct T2 {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
struct T3 {
    #[proptest]
    field: Vec<String>,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
struct T4(
    #[proptest]
    usize,
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
#[proptest]
enum T5 {
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
enum T6 {
    #[proptest]
    V0,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
enum T7 {
    V0 {
        #[proptest]
        foo: &'static str,
    },
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
enum T8 {
    V0(#[proptest] bool)
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
enum T9 {
    #[proptest]
    V0(bool),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0014]
enum T10 {
    #[proptest]
    V0 { bar: bool },
}
