#[macro_use]
extern crate proptest_derive;

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0031]
#[proptest(no_bound)] // TODO: SHOULD BE ALLOWED!
struct T0<T> {
    field: ::std::marker::PhantomData<T>,
}

#[derive(Debug, Arbitrary)] //~ ERROR: 2 errors:
                            //~| [proptest_derive, E0031]
                            //~| [proptest_derive, E0008]
struct T1<T> {
    #[proptest(no_bound)]
    #[proptest(skip)]
    field: ::std::marker::PhantomData<T>,
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0031]
struct T2<T>(
    #[proptest(no_bound)]
    ::std::marker::PhantomData<T>,
);

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0031]
enum T3<T> {
    #[proptest(no_bound)]
    V1(T),
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0031]
enum T4<T> {
    #[proptest(no_bound)]
    V1 {
        field: T
    }
}

#[derive(Debug, Arbitrary)] //~ ERROR: [proptest_derive, E0031]
enum T5<T> {
    V1(
        #[proptest(no_bound)]
        T
    ),
}
