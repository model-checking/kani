#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
#[proptest(skip)]
struct A {}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
#[proptest(skip)]
struct B;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
#[proptest(skip)]
struct C();

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
#[proptest(skip)]
struct D { field: String }

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
#[proptest(skip)]
struct E(Vec<u8>);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
#[proptest(skip)]
enum F { V1, V2, }

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
struct G(
    #[proptest(skip)]
    Vec<u8>
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
struct H {
    #[proptest(skip)]
    field: Vec<u8>
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
enum I {
    V0 {
        #[proptest(skip)]
        field: Vec<u8>
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0008]
enum J {
    V0(#[proptest(skip)] Vec<u8>)
}
