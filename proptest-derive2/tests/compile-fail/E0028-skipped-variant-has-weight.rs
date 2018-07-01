#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors 
                            //~| [proptest_derive, E0028]
                            //~| [proptest_derive, E0006]
enum NonFatal {
    #[proptest(skip, weight = 2)]
    V1,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T0 {
    #[proptest(skip, weight = 2)]
    V1,
    V2,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T1 {
    #[proptest(skip, w = 3)]
    V1(u8),
    V2,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0028]
enum T2 {
    #[proptest(skip, w = 3)]
    V1 {
        field: String,
    },
    V2,
}
