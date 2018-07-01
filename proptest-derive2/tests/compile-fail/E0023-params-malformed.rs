#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0023]
                            //~| [proptest_derive, E0008]
#[proptest(skip)]
#[proptest(params)]
struct NonFatal;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0023]
#[proptest(params)]
enum T0 {
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0023]
enum T2 {
    #[proptest(params)]
    V1
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0023]
enum T3 {
    V1 {
        #[proptest(params)]
        field: Box<str>,
    }
}
