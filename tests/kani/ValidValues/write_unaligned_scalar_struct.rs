// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! `ptr::write_unaligned::<bool>` casts `*mut bool` to `*mut Unaligned<bool>`, a packed struct
//! whose layout is the scalar `bool`. Both have the same validity, so the cast is supported.

#[kani::proof]
fn write_unaligned_bool() {
    let mut buf = [0u8; 4];
    let off: usize = kani::any_where(|o: &usize| *o < 4);
    let p = unsafe { buf.as_mut_ptr().add(off) } as *mut bool;
    let v: bool = kani::any();
    unsafe { p.write_unaligned(v) };
    assert_eq!(unsafe { p.read_unaligned() }, v);
}

#[repr(transparent)]
struct Wrapper(bool);

#[kani::proof]
fn cast_to_scalar_wrapper() {
    let mut b = true;
    let p = &mut b as *mut bool as *mut Wrapper;
    unsafe { *p = Wrapper(kani::any()) };
    let _ = b;
}
