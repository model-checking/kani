pub fn foo(x: u32) -> u32 {
    let y = x / 2;
    let z = y * 2;
    if x % 2 == 0 {
        assert!(z == x);
    } else {
        assert!(z == x - 1);
    }
    z
}
