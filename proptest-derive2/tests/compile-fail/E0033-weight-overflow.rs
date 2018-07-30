#[macro_use]
extern crate proptest_derive;

// Show non-fatal:
#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0033]
                            //~| [proptest_derive, E0008]
enum T0<#[proptest(skip)] T> {
    #[proptest(weight = 4294967290)]
    V0(T),
    #[proptest(weight = 5)]
    V1,
    V2,
}
