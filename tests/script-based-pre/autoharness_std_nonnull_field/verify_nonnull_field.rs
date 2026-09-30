// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Code injected into `core`: the `Arbitrary` impl verify-rust-std gives `NonNull` for its own
/// `NonNull` proofs, and functions whose arguments reach `NonNull` in different ways.
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

    pub struct OptHolder {
        p: Option<NonNull<u8>>,
        len: usize,
    }

    /// Skipped: `Option`'s own impl would build the `NonNull` through the impl above.
    pub fn opt_holder(h: OptHolder) -> usize {
        if h.p.is_some() { h.len } else { 0 }
    }

    pub struct TupleHolder {
        t: (NonNull<u8>, usize),
    }

    /// Skipped: likewise through a tuple.
    pub fn tuple_holder(h: TupleHolder) -> usize {
        h.t.1
    }

    pub struct ArrayHolder {
        a: [NonNull<u8>; 2],
    }

    /// Skipped: likewise through an array.
    pub fn array_holder(h: ArrayHolder) -> bool {
        h.a[0] == h.a[1]
    }

    /// Generated: a `NonNull` argument uses the impl above directly.
    pub fn is_aligned(p: NonNull<u16>) -> bool {
        p.is_aligned()
    }
}
