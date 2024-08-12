// kani-flags: -Zghost-state -Zaliasing
#![feature(register_tool)]
#![feature(rustc_attrs)]
mod stackfns_ignore;
use stackfns_ignore::*;

macro_rules! initialize {
    () => {
        #[cfg(not(kani))]
        sstate::initialize();
    }
}

macro_rules! initialize_local {
    ($place:ident) => {
        #[cfg(not(kani))]
        initialize_local(std::ptr::addr_of!($place));
    }
}

macro_rules! new_mut_ref_from_value {
    ($reference:ident) => {
        #[cfg(not(kani))]
        new_mut_ref_from_value(std::ptr::addr_of!($reference),
                               $reference);
    }
}

macro_rules! stack_check_ref {
    ($reference:ident) => {
        #[cfg(not(kani))]
        stack_check_ref(std::ptr::addr_of!($reference));
    }
}

macro_rules! new_mut_raw_from_ref {
    ($pointer: ident, $reference: ident) => {
        #[cfg(not(kani))]
        new_mut_raw_from_ref(std::ptr::addr_of!($pointer),
                             std::ptr::addr_of!($reference));
    }
}

macro_rules! new_mut_ref_from_raw {
    ($pointer: ident, $reference: ident) => {
        #[cfg(not(kani))]
        new_mut_ref_from_raw(std::ptr::addr_of!($pointer),
                             std::ptr::addr_of!($reference));
    }
}

macro_rules! stack_check_ptr {
    ($pointer: ident) => {
        #[cfg(not(kani))]
        stack_check_ptr(std::ptr::addr_of!($pointer));
    }
}

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
    raw_pointer = temp_ref as *mut i32;
    initialize_local!(raw_pointer);
    new_mut_ref_from_value!(temp_ref);
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
