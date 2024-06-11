#[kani::proof]
fn main() {
    let v: Vec<u8> = Vec::with_capacity(10);
    let undef = unsafe { *v.as_ptr().add(5) }; //~ ERROR: uninitialized
    let x = undef + 1;
}
