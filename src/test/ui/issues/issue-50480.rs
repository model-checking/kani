#[derive(Clone, Copy)]
//~^ ERROR the trait `Copy` may not be implemented for this type
struct Foo(N, NotDefined, <i32 as Iterator>::Item, Vec<i32>, String);
//~^ ERROR cannot find type `NotDefined` in this scope
//~| ERROR cannot find type `NotDefined` in this scope
//~| ERROR cannot find type `N` in this scope
//~| ERROR cannot find type `N` in this scope
//~| ERROR `i32` is not an iterator

#[derive(Clone, Copy)]
//~^ ERROR the trait `Copy` may not be implemented for this type
struct Bar<T>(T, N, NotDefined, <i32 as Iterator>::Item, Vec<i32>, String);
//~^ ERROR cannot find type `NotDefined` in this scope
//~| ERROR cannot find type `N` in this scope
//~| ERROR `i32` is not an iterator

fn main() {}
