// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Code injected into `core`: the `Arbitrary` impl verify-rust-std gives `NonNull` for its own
/// `NonNull` proofs, and two functions whose arguments reach `NonNull` in different ways.
#[cfg(kani)]
kani_core::kani_lib!(core);

#[cfg(kani)]
#[unstable(feature = "kani", issue = "none")]
pub mod verify_nonnull_field {
    use crate::kani;
    use crate::ptr::NonNull;

    impl<T> kani::Arbitrary for NonNull<T> {
        fn any() -> Self {
            let ptr: *mut T = kani::any::<usize>() as *mut T;
            kani::assume(!ptr.is_null());
            NonNull::new(ptr).expect("Non-null pointer expected")
        }
    }

    pub struct Buffer {
        ptr: NonNull<u8>,
        len: usize,
    }

    /// Skipped: a derived `Buffer` would point to no allocation.
    pub fn first(b: Buffer) -> Option<u8> {
        if b.len == 0 { None } else { Some(unsafe { *b.ptr.as_ptr() }) }
    }

    /// Generated: a `NonNull` argument uses the impl above directly.
    pub fn is_aligned(p: NonNull<u16>) -> bool {
        p.is_aligned()
    }
}
