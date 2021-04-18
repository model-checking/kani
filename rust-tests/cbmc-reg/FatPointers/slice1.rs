fn main() {
    let array = [1, 2, 3, 4, 5, 6];
    let slice: &[u32] = &array;
    assert!(slice[0] == 1);
    assert!(slice[5] == 6);
}
