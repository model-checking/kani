#[kani::proof]
fn wrong_coverage_2() {
    let a: u8 = kani::any();
    kani::assume(false);
    assert!(a < 5);
}
