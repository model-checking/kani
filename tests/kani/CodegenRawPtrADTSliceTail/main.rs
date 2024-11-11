// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Test codegen for a raw pointer to a struct whose last field is a slice

#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]

// https://github.com/model-checking/kani/issues/3615
mod issue_3615 {
    use std::ptr;

    #[derive(Clone, Copy, kani::Arbitrary)]
    struct Wrapper<T: ?Sized> {
        _size: usize,
        _value: T,
    }

    #[kani::proof]
    pub fn from_raw_parts_for_slices() {
        let var: Wrapper<[u64; 4]> = kani::any();
        let fat_ptr: *const Wrapper<[u64]> = &var as *const _;
        let (thin_ptr, _) = fat_ptr.to_raw_parts();
        let new_size: usize = kani::any();
        let _new_ptr: *const Wrapper<[u64]> = ptr::from_raw_parts(thin_ptr, new_size);
    }

    #[kani::proof]
    pub fn from_raw_parts_for_slices_nested() {
        let var: Wrapper<Wrapper<[u8; 4]>> = kani::any();
        let fat_ptr: *const Wrapper<Wrapper<[u8]>> = &var as *const _;
        let (thin_ptr, _) = fat_ptr.to_raw_parts();
        let new_size: usize = kani::any();
        let _new_ptr: *const Wrapper<Wrapper<[u8]>> = ptr::from_raw_parts(thin_ptr, new_size);
    }
}

// https://github.com/model-checking/kani/issues/3638
mod issue_3638 {
    use std::ptr::NonNull;

    #[derive(kani::Arbitrary)]
    struct Wrapper<T: ?Sized>(usize, T);

    #[cfg(kani)]
    #[kani::proof]
    fn main() {
        // Create a SampleTrait object from SampleStruct
        let original: Wrapper<[u8; 10]> = kani::any();
        let slice: &Wrapper<[u8]> = &original;

        // Get the raw data pointer and metadata for the trait object
        let slice_ptr = NonNull::new(slice as *const _ as *mut ()).unwrap();
        let metadata = std::ptr::metadata(slice);

        // Create NonNull<dyn SampleTrait> from the data pointer and metadata
        let _: NonNull<Wrapper<[u8]>> = NonNull::from_raw_parts(slice_ptr, metadata);
    }
}
