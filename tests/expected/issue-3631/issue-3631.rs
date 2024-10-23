// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(ptr_metadata)]

use std::ptr::NonNull;

trait SampleTrait {
    fn get_value(&self) -> i32;
}

struct SampleStruct {
    value: i32,
}

impl SampleTrait for SampleStruct {
    fn get_value(&self) -> i32 {
        self.value
    }
}

#[cfg(kani)]
#[kani::proof]
fn main() {
    // Create a SampleTrait object from SampleStruct
    let sample_struct = SampleStruct { value: kani::any() };
    let trait_object: &dyn SampleTrait = &sample_struct;

    // Get the raw data pointer and metadata for the trait object
    let trait_ptr = NonNull::new(trait_object as *const dyn SampleTrait as *mut ()).unwrap();
    let metadata = std::ptr::metadata(trait_object);

    // Create NonNull<dyn SampleTrait> from the data pointer and metadata
    let nonnull_trait_object: NonNull<dyn SampleTrait> =
        NonNull::from_raw_parts(trait_ptr, metadata);

    unsafe {
        // Ensure trait method and member is preserved
        kani::assert(
            trait_object.get_value() == nonnull_trait_object.as_ref().get_value(),
            "trait method and member must correctly preserve",
        );
    }
}
