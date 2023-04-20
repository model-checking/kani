fn main() {}

#[kani::proof]
fn harness_in_bin_package() {
    assert!(1 + 1 == 2);
}