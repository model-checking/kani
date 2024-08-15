// kani-flags: -Zghost-state -Zaliasing

#[kani::proof]
fn main() {
    let mut local: i32 = 0;
    let raw_pointer = &mut local as *mut i32;
    unsafe {
        let ref_from_raw_1 = &mut *raw_pointer;
        *ref_from_raw_1 = 0;
        let ref_from_raw_2 = &mut *raw_pointer;
        *ref_from_raw_2 = 1;
        *ref_from_raw_1 = 2;
    }
}
