fn main() {}

#[kani::proof]
fn harness_in_ws_package() {
    assert!(1 + 1 == 2);
}
