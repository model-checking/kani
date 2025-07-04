#[kani::proof]
fn foo() {
    std::debug_assert!(false, "will fail");
    std::assert!(false, "will fail");
    std::debug_assert!(false, "not reached");
}
