const C: [u32; 5] = [0; 5];

#[allow(unconditional_panic)]
fn test() -> u32 {
    C[10]
}

#[kani::proof]
fn main() {
    test();
}
