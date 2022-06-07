#[kani::proof]
fn a_check() {
    let v = vec![1, 2, 3];
    assert_eq!(v.len(), 3);
}
