#[kani::requires(x != 0)]
fn foo<T: std::cmp::PartialEq<i32>>(x: T) {
    assert_ne!(x, 0);
    foo(x);
}

#[kani::proof_for_contract(foo)]
fn foo_harness() {
    let input: i32 = kani::any();
    foo(input);
}
