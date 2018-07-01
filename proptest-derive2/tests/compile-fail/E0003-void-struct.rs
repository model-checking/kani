#![feature(never_type)]

#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0003]
                            //~| [proptest_derive, E0008]
struct NonFatal {
    #[proptest(skip)]
    x: !,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0003]
struct Ty0 { x: ! }

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0003]
struct Ty1 {
    x: usize,
    y: !,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0003]
struct Ty2 {
    x: (!, usize),
    y: bool,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0003]
struct Ty3 {
    x: [!; 1]
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0003]
struct Ty4 {
    x: [::std::string::ParseError; 1],
}
