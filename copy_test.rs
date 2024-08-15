#[repr(C)]
#[derive(kani::Arbitrary)]
struct S(u32, u8); // 5 bytes of data + 3 bytes of padding.

// #[kani::proof]
// fn delayed_ub_double_copy() {
//     unsafe {
//         let mut value: u128 = 0;
//         let ptr = &mut value as *mut _ as *mut (u8, u32, u64);
//         // Use `copy_nonoverlapping` in an attempt to remove the taint.
//         std::ptr::write(ptr, (4, 4, 4));
//         // Instead of assigning the value into a delayed UB place, copy it from another delayed UB
//         // place.
//         let mut value_2: u128 = 0; 
//         let ptr_2 = &mut value_2 as *mut _ as *mut (u8, u32, u64);
//         std::ptr::copy(ptr_2, ptr, 1); // This should not trigger UB since the copy is untyped.
//         assert!(value_2 > 0); // UB: This reads a padding value!
//     }
// }

// #[kani::proof]
// fn delayed_ub_trigger_copy() {
//     unsafe {
//         let mut value: u128 = 0;
//         let ptr = &mut value as *mut _ as *mut u8; // This cast should not be a delayed UB source.
//         let mut value_different_padding: (u8, u32, u64)  = (4, 4, 4);
//         let ptr_different_padding = &mut value_different_padding as *mut _ as *mut u8;
//         std::ptr::copy(ptr_different_padding, ptr, std::mem::size_of::<u128>()); // This is a delayed UB source.
//         assert!(value > 0); // UB: This reads a padding value!
//     }
// }

#[kani::proof]
/// This checks that reading copied uninitialized bytes fails an assertion.
unsafe fn expose_padding_via_copy_convoluted() {
    unsafe fn copy_and_read_helper(from_ptr: *const S, to_ptr: *mut u64) -> u64 {
        // This should not cause UB since `copy` is untyped.
        std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<S>());
        // This reads uninitialized bytes, which is UB.
        let padding: u64 = std::ptr::read(to_ptr);
        padding
    }

    unsafe fn partial_copy_and_read_helper(from_ptr: *const S, to_ptr: *mut u64) -> u32 {
        // This should not cause UB since `copy` is untyped.
        std::ptr::copy(from_ptr as *const u8, to_ptr as *mut u8, std::mem::size_of::<u32>());
        // This does not read uninitialized bytes.
        let not_padding: u32 = std::ptr::read(to_ptr as *mut u32);
        not_padding
    }

    let flag: bool = kani::any();

    let from: S = kani::any();
    let mut to: u64 = kani::any();

    let from_ptr = &from as *const S;
    let to_ptr = &mut to as *mut u64;

    if flag {
        copy_and_read_helper(from_ptr, to_ptr);
    } else {
        partial_copy_and_read_helper(from_ptr, to_ptr);
    }
}
