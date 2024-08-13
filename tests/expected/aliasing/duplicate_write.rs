// kani-flags: -Zghost-state -Zaliasing
#![cfg_attr(not(kani), feature(register_tool))]
#![cfg_attr(not(kani), register_tool(kani))]
#![feature(rustc_attrs)]
#![allow(internal_features)]
#![feature(vec_into_raw_parts)]

include!{"./stackfns.txt"}

#[cfg_attr(any(kani), kani::proof)]
fn main() {
    let mut local: i32;
    let temp_ref: &mut i32;
    let raw_pointer: *mut i32;
    let ref_from_raw_1: &mut i32;
    let ref_from_raw_2: &mut i32;
    initialize!();

    local = 0;
    initialize_local!(local);
    temp_ref = &mut local;
    initialize_local!(temp_ref);
    new_mut_ref_from_value!(temp_ref);
    raw_pointer = temp_ref as *mut i32;
    initialize_local!(raw_pointer);
    stack_check_ref!(temp_ref);
    new_mut_raw_from_ref!(raw_pointer, temp_ref);
    unsafe {
        ref_from_raw_1 = &mut *raw_pointer;
        new_mut_ref_from_raw!(ref_from_raw_1, raw_pointer);
        *ref_from_raw_1 = 0;
        stack_check_ref!(ref_from_raw_1);
        ref_from_raw_2 = &mut *raw_pointer;
        stack_check_ptr!(raw_pointer);
        new_mut_ref_from_raw!(ref_from_raw_2, raw_pointer);
        *ref_from_raw_2 = 1;
        stack_check_ref!(ref_from_raw_2);
        *ref_from_raw_1 = 2;
        stack_check_ref!(ref_from_raw_1);
    }
}
