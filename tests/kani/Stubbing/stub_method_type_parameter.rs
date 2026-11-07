// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

trait GenericMethod<T> {
    fn pick<U: Copy, V: Copy>(&self, x: T, y: U, z: V) -> bool;
}
struct Target;
struct Untouched;
impl GenericMethod<u8> for Target {
    fn pick<U: Copy, V: Copy>(&self, _: u8, _: U, _: V) -> bool {
        false
    }
}
impl GenericMethod<u64> for Target {
    fn pick<U: Copy, V: Copy>(&self, _: u64, _: U, _: V) -> bool {
        false
    }
}
impl GenericMethod<u8> for Untouched {
    fn pick<U: Copy, V: Copy>(&self, _: u8, _: U, _: V) -> bool {
        false
    }
}
fn replacement<U: Copy, V: Copy>(_: &Target, _: u8, _: U, _: V) -> bool {
    true
}

#[kani::proof]
#[kani::stub(<Target as GenericMethod<u8>>::pick, replacement)]
fn generic_method_positive() {
    assert!(Target.pick(0u8, kani::any::<u8>(), 1u64));
    assert!(Target.pick(0u8, kani::any::<u64>(), 1u8));
    assert!(!Target.pick(0u64, 1u8, 2u8));
    assert!(!Untouched.pick(0u8, 1u8, 2u8));
}

#[kani::proof]
#[kani::should_panic]
#[kani::stub(<Target as GenericMethod<u8>>::pick, replacement)]
fn generic_method_negative() {
    assert!(!Target.pick(0u8, 1u8, 2u64));
}

trait ConstMethod {
    fn length<const N: usize>(&self) -> usize;
}
impl ConstMethod for Target {
    fn length<const N: usize>(&self) -> usize {
        unreachable!()
    }
}
fn replacement_length<const N: usize>(_: &Target) -> usize {
    N
}

#[kani::proof]
#[kani::stub(<Target as ConstMethod>::length, replacement_length)]
fn method_const_arguments() {
    assert_eq!(Target.length::<0>(), 0);
    assert_eq!(Target.length::<3>(), 3);
}

trait Borrow<'a, T> {
    fn borrow<U: Copy>(&self, input: &'a T, extra: U) -> bool;
}
impl<'a> Borrow<'a, u8> for Target {
    fn borrow<U: Copy>(&self, _: &'a u8, _: U) -> bool {
        unreachable!()
    }
}
fn replacement_borrow<'a: 'a, U: Copy>(_: &Target, _: &'a u8, _: U) -> bool {
    true
}

#[kani::proof]
#[kani::stub(<Target as Borrow<'static, u8>>::borrow, replacement_borrow)]
fn explicit_parent_lifetime() {
    static INPUT: u8 = 1;
    assert!(<Target as Borrow<'static, u8>>::borrow(&Target, &INPUT, 2u64));
}

#[kani::proof]
#[kani::stub(<Target as Borrow<u8>>::borrow, replacement_borrow)]
fn elided_parent_lifetime() {
    assert!(<Target as Borrow<u8>>::borrow(&Target, &1, 2u64));
}

trait BorrowMethod {
    fn early<'a: 'a, U: Copy>(&self, value: &'a u8, extra: U) -> bool;
    fn late<'a, U: Copy>(&self, value: &'a u8, extra: U) -> bool;
}
impl BorrowMethod for Target {
    fn early<'a: 'a, U: Copy>(&self, _: &'a u8, _: U) -> bool {
        unreachable!()
    }
    fn late<'a, U: Copy>(&self, _: &'a u8, _: U) -> bool {
        unreachable!()
    }
}
fn replace_early<'a: 'a, U: Copy>(_: &Target, _: &'a u8, _: U) -> bool {
    true
}
fn replace_late<'a, U: Copy>(_: &Target, _: &'a u8, _: U) -> bool {
    true
}
#[kani::proof]
#[kani::stub(<Target as BorrowMethod>::early, replace_early)]
fn early_bound_method_lifetime() {
    assert!(Target.early(&1, 2u64));
}
#[kani::proof]
#[kani::stub(<Target as BorrowMethod>::late, replace_late)]
fn late_bound_method_lifetime() {
    assert!(Target.late(&1, 2u64));
}
