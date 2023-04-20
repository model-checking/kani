pub fn api() {}

#[kani::proof]
fn harness_in_lib_package() {
    assert!(1 + 1 == 2);
}