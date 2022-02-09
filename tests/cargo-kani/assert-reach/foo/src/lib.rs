pub fn foo(x: u32) -> u32 {
    let y = x / 2;
    let z = y * 2;
    if z == x {
        assert!(x % 2 == 0);
    } else {
        assert!(x % 2 == 1)
    }
    z
}
