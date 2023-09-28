use std::marker::PhantomData;

pub struct Foo<R> {
    x: u8,
    _t: PhantomData<R>,
}

#[kani::proof]
fn main() {
    const C: Foo<usize> = Foo {
        x: 0,
        _t: PhantomData,
    };
    assert_eq!(C.x, 0);
}
