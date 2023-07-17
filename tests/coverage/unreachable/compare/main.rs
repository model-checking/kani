fn compare(x: u16, y: u16) -> u16 {
    if x >= y {
        1
    } else {
        kani::cover!();
        0
    }
}

#[kani::proof]
fn main() {
    let x: u16 = kani::any();
    let y: u16 = kani::any();
    if x >= y {
        compare(x, y);
    }
}
