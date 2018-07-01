#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0001]
                            //~| [proptest_derive, E0008]
#[proptest(skip)]
struct NonFatal<'a>(&'a ());

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0001]
struct T0<'a>(&'a ());

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0001]
enum T1<'a> {
    V0(&'a ())
}
