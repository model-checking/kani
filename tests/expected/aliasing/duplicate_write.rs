// kani-flags: -Zghost-state -Zaliasing

#[kani::proof]
fn main() {
    let mut local: i32;
    let temp_ref: &mut i32;
    let raw_pointer: *mut i32;
    let ref_from_raw_1: &mut i32;
    let ref_from_raw_2: &mut i32;

    local = 0;
    temp_ref = &mut local;
    raw_pointer = temp_ref as *mut i32;
    unsafe {
        ref_from_raw_1 = &mut *raw_pointer;
        *ref_from_raw_1 = 0;
        ref_from_raw_2 = &mut *raw_pointer;
        *ref_from_raw_2 = 1;
        *ref_from_raw_1 = 2;
    }
}
