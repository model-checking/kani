#[kani::proof]
fn another_check() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}
