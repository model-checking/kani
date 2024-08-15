// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

//! Checks that Kani catches instances of delayed UB.

/// Delayed UB via casted mutable pointer write.
#[kani::proof]
fn delayed_ub() {
    unsafe {
        let mut value: u128 = 0;
        // Cast between two pointers of different padding.
        let ptr = &mut value as *mut _ as *mut (u8, u32, u64);
        *ptr = (4, 4, 4);
        let c: u128 = value; // UB: This reads a padding value!
    }
}

/// Delayed UB via transmuted mutable pointer write.
#[kani::proof]
fn delayed_ub_transmute() {
    unsafe {
        let mut value: u128 = 0;
        // Transmute between two pointers of different padding.
        let ptr: *mut (u8, u32, u64) = std::mem::transmute(&mut value as *mut _);
        *ptr = (4, 4, 4);
        let c: u128 = value; // UB: This reads a padding value!
    }
}

static mut VALUE: u128 = 42;

/// Delayed UB via mutable pointer write into a static.
#[kani::proof]
fn delayed_ub_static() {
    unsafe {
        let v_ref = &mut VALUE;
        // Cast reference to static to a pointer of different padding.
        let ptr = &mut VALUE as *mut _ as *mut (u8, u32, u64);
        *ptr = (4, 4, 4);
        assert!(*v_ref > 0); // UB: This reads a padding value!
    }
}

/// Helper to launder the pointer while keeping the address.
unsafe fn launder(ptr: *mut u128) -> *mut u128 {
    let a = ptr;
    let b = a as *const u128;
    let c: *mut i128 = std::mem::transmute(b);
    let d = c as usize;
    let e = d + 1;
    let f = e - 1;
    return f as *mut u128;
}

/// Delayed UB via mutable pointer write with additional laundering.
#[kani::proof]
fn delayed_ub_laundered() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut u128;
        // Pass pointer around in an attempt to remove the association.
        let ptr = launder(ptr) as *mut (u8, u32, u64);
        *ptr = (4, 4, 4);
        assert!(value > 0); // UB: This reads a padding value!
    }
}

/// Delayed UB via mutable pointer write with additional laundering but via closure.
#[kani::proof]
fn delayed_ub_closure_laundered() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut u128;
        // Add extra args to test spread_arg.
        let launder = |arg1: bool, arg2: bool, arg3: bool, ptr: *mut u128| -> *mut u128 {
            let a = ptr;
            let b = a as *const u128;
            let c: *mut i128 = std::mem::transmute(b);
            let d = c as usize;
            let e = d + 1;
            let f = e - 1;
            return f as *mut u128;
        };
        // Pass pointer around in an attempt to remove the association.
        let ptr = launder(false, true, false, ptr) as *mut (u8, u32, u64);
        *ptr = (4, 4, 4);
        assert!(value > 0); // UB: This reads a padding value!
    }
}

/// Delayed UB via mutable pointer write with additional laundering but via closure captures.
#[kani::proof]
fn delayed_ub_closure_capture_laundered() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut u128;
        // Add extra args to test spread_arg.
        let launder = |arg1: bool, arg2: bool, arg3: bool| -> *mut u128 {
            let a = ptr;
            let b = a as *const u128;
            let c: *mut i128 = std::mem::transmute(b);
            let d = c as usize;
            let e = d + 1;
            let f = e - 1;
            return f as *mut u128;
        };
        // Pass pointer around in an attempt to remove the association.
        let ptr = launder(false, true, false) as *mut (u8, u32, u64);
        *ptr = (4, 4, 4);
        assert!(value > 0); // UB: This reads a padding value!
    }
}

/// Delayed UB via mutable pointer write using `copy_nonoverlapping` under the hood.
#[kani::proof]
fn delayed_ub_copy() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut _ as *mut (u8, u32, u64);
        // Use `copy_nonoverlapping` in an attempt to remove the taint.
        std::ptr::write(ptr, (4, 4, 4));
        assert!(value > 0); // UB: This reads a padding value!
    }
}

/// Delayed UB via multiple mutable pointers write using `copy_nonoverlapping` and `copy` under the
/// hood.
#[kani::proof]
fn delayed_ub_double_copy() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut _ as *mut (u8, u32, u64);
        // Use `copy_nonoverlapping` in an attempt to remove the taint.
        std::ptr::write(ptr, (4, 4, 4));
        // Instead of assigning the value into a delayed UB place, copy it from another delayed UB
        // place.
        let mut value_2: u128 = 0; 
        let ptr_2 = &mut value_2 as *mut _ as *mut (u8, u32, u64);
        std::ptr::copy(ptr, ptr_2, 1); // This should not trigger UB since the copy is untyped.
        assert!(value_2 > 0); // UB: This reads a padding value!
    }
}

struct S {
    u: U,
}

struct U {
    value1: u128,
    value2: u64,
    value3: u32,
}

struct Inner<T>(*mut T);

/// Delayed UB via mutable pointer write into inner fields of structs.
#[kani::proof]
fn delayed_ub_structs() {
    unsafe {
        // Create a convoluted struct.
        let mut s: S = S { u: U { value1: 0, value2: 0, value3: 0 } };
        // Get a pointer to an inner field of the struct. Then, cast between two pointers of
        // different padding.
        let inner = Inner(&mut s.u.value2 as *mut _);
        let inner_cast = Inner(inner.0 as *mut (u8, u32));
        let ptr = inner_cast.0;
        *ptr = (4, 4);
        let u: U = s.u; // UB: This reads a padding value inside the inner struct!
    }
}

/// Delayed UB via mutable pointer write into a slice element.
#[kani::proof]
fn delayed_ub_slices() {
    unsafe {
        // Create an array.
        let mut arr = [0u128; 4];
        // Get a pointer to a part of the array.
        let ptr = &mut arr[0..2][0..1][0] as *mut _ as *mut (u8, u32);
        *ptr = (4, 4);
        let arr_copy = arr; // UB: This reads a padding value inside the array!
    }
}

/// Delayed UB via mutable pointer copy, which should be the only delayed UB trigger in this case.
#[kani::proof]
fn delayed_ub_trigger_copy() {
    unsafe {
        let mut value: u128 = 0;
        let ptr = &mut value as *mut _ as *mut u8; // This cast should not be a delayed UB source.
        let mut value_different_padding: (u8, u32, u64)  = (4, 4, 4);
        let ptr_different_padding = &mut value_different_padding as *mut _ as *mut u8;
        std::ptr::copy(ptr_different_padding, ptr, std::mem::size_of::<u128>()); // This is a delayed UB source.
        assert!(value > 0); // UB: This reads a padding value!
    }
}
