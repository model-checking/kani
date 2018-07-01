#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0021]
                            //~| [proptest_derive, E0008]
#[proptest(weight)]
#[proptest(skip)]
struct NonFatal;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T0 {
    #[proptest(weight)]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T1 {
    #[proptest(weight("abcd"))]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T2 {
    #[proptest(weight("1.0"))]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T3 {
    #[proptest(weight("true"))]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T4 {
    #[proptest(weight = "true")]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T5 {
    #[proptest(weight = true)]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0021]
enum T6 {
    #[proptest(weight(true))]
    V1
}
