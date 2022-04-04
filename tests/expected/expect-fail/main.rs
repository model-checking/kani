#[kani::proof]
fn main() {
    let x: i32 = kani::any();
    kani::expect_fail(x == 5, "x is not 5!");
}
