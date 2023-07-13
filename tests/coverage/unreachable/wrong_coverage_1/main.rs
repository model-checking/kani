#[kani::proof]
fn wrong_coverage_1() {
    let x: u8 = kani::any();
    if x > 5 {
        if x < 2 {
            let y = x;
        }
    } else {
        assert!(x < 10);
    }
}
