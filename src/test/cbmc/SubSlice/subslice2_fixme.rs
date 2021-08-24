fn main() {
    let arr = [1, 2, 3];
    // s is a slice (&[i32])
    let [s @ ..] = &arr[1..];
    assert!(s[0] == 2);
    assert!(s[1] == 3);
}
