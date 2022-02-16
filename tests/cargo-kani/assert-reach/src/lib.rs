#[kani::proof]
fn test() {
    let x = 4;
    let y = foo::foo(x);
    assert!(y == x);
}
