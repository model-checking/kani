// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains utilities related to floating-point types

use cbmc::{MachineModel, goto_program::Expr};
use stable_mir::ty::{FloatTy, IntTy, RigidTy, UintTy};

/// This function creates a boolean expression that the given `float_expr` when truncated is in the range of `integral_ty`.
///
/// The expression it generates is in the form:
///
///     `float_expr > lower_bound && float_expr < upper_bound`
///
/// i.e., the comparison is performed in terms of floats
///
/// Generally, the lower bound for an integral type is `MIN - 1` and the upper bound is `MAX + 1`.
/// For example, for `i16`, the lower bound is `i16::MIN - 1` (-32769) and the upper bound is `i16::MAX + 1` (32768)
/// Similarly, for `u8`, the lower bound is `u8::MIN - 1` (-1) and the upper bound is `u8::MAX + 1` (256)
///
/// However, due to the floating-point imprecision, not every value has a representation.
/// For example, while `i16::MIN - 1` (-32769) and `u8::MAX + 1` (256) can be accurately represented as `f32` and `f64`,
/// `i32::MIN - 1` (-2147483649) cannot be represented in `f32` (the closest `f32` value is -2147483648).
/// See https://www.h-schmidt.net/FloatConverter/IEEE754.html
///
/// If we were to just use `MIN - 1`, the resulting expression may exclude values that are actually in range.
/// For example, `float_expr > ((i32::MIN - 1) as f32)` would expand to `float_expr > -2147483649 as f32` which
/// would result in `float_expr > -2147483648.0`. This expression incorrectly exlcudes a valid `i32`
/// value: `i32::MIN` = -2147483648.
///
/// Thus, to determine the lower bound, we need to find the **largest** floating-point value that is
/// less than or equal to `MIN - 1`.
/// For example, for `i32`, the largest such value is `-2147483904.0`
/// Similarly, to determine the upper bound, we need to find the smallest floating-point value that is
/// greater than or equal to `MAX + 1`.
///
/// An alternative approach would be to perform the float-to-int cast with a wider integer and
/// then check if the wider integer value is in the range of the narrower integer value.
/// This seems to be the approach used in MIRI:
/// https://github.com/rust-lang/rust/blob/096277e989d6de11c3077472fc05778e261e7b8e/src/tools/miri/src/helpers.rs#L1003
/// but it's not clear what it does for `i128` and `u128`.
pub fn codegen_in_range_expr(
    float_expr: &Expr,
    float_ty: FloatTy,
    integral_ty: RigidTy,
    mm: &MachineModel,
) -> Expr {
    match float_ty {
        FloatTy::F32 => {
            let (lower, upper) = get_bounds_f32(integral_ty, mm);
            let mut in_range = Expr::bool_true();
            // Avoid unnecessary comparison against -∞ or ∞
            if lower != f32::NEG_INFINITY {
                in_range = in_range.and(float_expr.clone().gt(Expr::float_constant(lower)));
            }
            if upper != f32::INFINITY {
                in_range = in_range.and(float_expr.clone().lt(Expr::float_constant(upper)));
            }
            in_range
        }
        FloatTy::F64 => {
            let (lower, upper) = get_bounds_f64(integral_ty, mm);
            let mut in_range = Expr::bool_true();
            if lower != f64::NEG_INFINITY {
                in_range = in_range.and(float_expr.clone().gt(Expr::double_constant(lower)));
            }
            if upper != f64::INFINITY {
                in_range = in_range.and(float_expr.clone().lt(Expr::double_constant(upper)));
            }
            in_range
        }
        _ => unimplemented!(),
    }
}

const F32_I8_LOWER: [u8; 4] = [0x00, 0x00, 0x01, 0xC3]; // -129.0
const F32_I8_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x43]; // 128.0
const F32_I16_LOWER: [u8; 4] = [0x00, 0x01, 0x00, 0xC7]; // -32769.0
const F32_I16_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x47]; // 32768.0
const F32_I32_LOWER: [u8; 4] = [0x01, 0x00, 0x00, 0xCF]; // -2147483904.0
const F32_I32_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x4F]; // 2147483648.0
const F32_I64_LOWER: [u8; 4] = [0x01, 0x00, 0x00, 0xDF]; // -9223373136366403584.0
const F32_I64_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x5F]; // 9223372036854775808.0
// The next value is determined manually and not via test:
const F32_I128_LOWER: [u8; 4] = [0x01, 0x00, 0x00, 0xFF]; // -170141203742878835383357727663135391744.0
const F32_I128_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x7F]; // 170141183460469231731687303715884105728.0

const F64_I8_LOWER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x60, 0xC0]; // -129.0
const F64_I8_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x40]; // 128.0
const F64_I16_LOWER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0xE0, 0xC0]; // -32769.0
const F64_I16_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x40]; // 32768.0
const F64_I32_LOWER: [u8; 8] = [0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xE0, 0xC1]; // -2147483649.0
const F64_I32_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x41]; // 2147483648.0
const F64_I64_LOWER: [u8; 8] = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0xC3]; // -9223372036854777856.0
const F64_I64_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x43]; // 9223372036854775808.0
// The next value is determined manually and not via test:
const F64_I128_LOWER: [u8; 8] = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0xC7]; // -170141183460469269510619166673045815296.0
const F64_I128_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x47]; // 170141183460469231731687303715884105728.0

const F32_U_LOWER: [u8; 4] = [0x00, 0x00, 0x80, 0xBF]; // -1.0
const F32_U8_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x43]; // 256.0
const F32_U16_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x47]; // 65536.0
const F32_U32_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x4F]; // 4294967296.0
const F32_U64_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x5F]; // 18446744073709551616.0
// The largest f32 value fits in a u128, so there is no upper bound
const F32_U128_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x7F]; // inf

const F64_U_LOWER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0xBF]; // -1.0
const F64_U8_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x40]; // 256.0
const F64_U16_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x40]; // 65536.0
const F64_U32_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x41]; // 4294967296.0
const F64_U64_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x43]; // 18446744073709551616.0
const F64_U128_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x47]; // 340282366920938463463374607431768211456.0

/// upper is the smallest `f32` that after truncation is strictly larger than i<N>::MAX or u<N>::MAX
/// lower is the largest `f32` that after truncation is strictly smaller than i<N>::MIN or u<N>::MIN
///
/// For example, for `u8`, upper is 256.0 because the previous f32 (i.e.
/// `256_f32.next_down()` which is 255.9999847412109375) when truncated is 255.0,
/// which is not strictly larger than `u8::MAX`
///
/// For `i16`, upper is 32768.0 because the previous f32 (i.e.
/// `32768_f32.next_down()`) when truncated is 32767,
/// which is not strictly larger than `i16::MAX`
///
/// Note that all upper bound values are 2^(w-1) which can be precisely
/// represented in f32 (verified using
/// https://www.h-schmidt.net/FloatConverter/IEEE754.html)
/// However, for lower bound values, which should be -2^(w-1)-1 (i.e.
/// i<N>::MIN-1), not all of them can be represented in f32.
/// For instance, for w = 32, -2^(31)-1 = -2,147,483,649, but this number does
/// **not** have an f32 representation, and the next **smaller** number is
/// -2,147,483,904. Note that CBMC for example uses the formula above which
/// leads to bugs, e.g.: https://github.com/diffblue/cbmc/issues/8488
///
/// For all unsigned types, lower is -1.0 because the next higher number, when
/// truncated is -0.0 (or 0.0) which is not strictly smaller than `u<N>::MIN`
fn get_bounds_f32(integral_ty: RigidTy, mm: &MachineModel) -> (f32, f32) {
    match integral_ty {
        RigidTy::Int(int_ty) => get_bounds_f32_int(int_ty, mm),
        RigidTy::Uint(uint_ty) => get_bounds_f32_uint(uint_ty, mm),
        _ => unreachable!(),
    }
}

fn get_bounds_f64(integral_ty: RigidTy, mm: &MachineModel) -> (f64, f64) {
    match integral_ty {
        RigidTy::Int(int_ty) => get_bounds_f64_int(int_ty, mm),
        RigidTy::Uint(uint_ty) => get_bounds_f64_uint(uint_ty, mm),
        _ => unreachable!(),
    }
}

fn get_bounds_f32_uint(uint_ty: UintTy, mm: &MachineModel) -> (f32, f32) {
    let lower: f32 = f32::from_le_bytes(F32_U_LOWER);
    let upper: f32 = match uint_ty {
        UintTy::U8 => f32::from_le_bytes(F32_U8_UPPER),
        UintTy::U16 => f32::from_le_bytes(F32_U16_UPPER),
        UintTy::U32 => f32::from_le_bytes(F32_U32_UPPER),
        UintTy::U64 => f32::from_le_bytes(F32_U64_UPPER),
        UintTy::U128 => f32::from_le_bytes(F32_U128_UPPER),
        UintTy::Usize => match mm.pointer_width {
            32 => f32::from_le_bytes(F32_U32_UPPER),
            64 => f32::from_le_bytes(F32_U64_UPPER),
            _ => unreachable!(),
        },
    };
    (lower, upper)
}

fn get_bounds_f64_uint(uint_ty: UintTy, mm: &MachineModel) -> (f64, f64) {
    let lower = f64::from_le_bytes(F64_U_LOWER);
    let upper = match uint_ty {
        UintTy::U8 => f64::from_le_bytes(F64_U8_UPPER),
        UintTy::U16 => f64::from_le_bytes(F64_U16_UPPER),
        UintTy::U32 => f64::from_le_bytes(F64_U32_UPPER),
        UintTy::U64 => f64::from_le_bytes(F64_U64_UPPER),
        UintTy::U128 => f64::from_le_bytes(F64_U128_UPPER),
        UintTy::Usize => match mm.pointer_width {
            32 => f64::from_le_bytes(F64_U32_UPPER),
            64 => f64::from_le_bytes(F64_U64_UPPER),
            _ => unreachable!(),
        },
    };
    (lower, upper)
}

fn get_bounds_f32_int(int_ty: IntTy, mm: &MachineModel) -> (f32, f32) {
    let lower = match int_ty {
        IntTy::I8 => f32::from_le_bytes(F32_I8_LOWER),
        IntTy::I16 => f32::from_le_bytes(F32_I16_LOWER),
        IntTy::I32 => f32::from_le_bytes(F32_I32_LOWER),
        IntTy::I64 => f32::from_le_bytes(F32_I64_LOWER),
        IntTy::I128 => f32::from_le_bytes(F32_I128_LOWER),
        IntTy::Isize => match mm.pointer_width {
            32 => f32::from_le_bytes(F32_I32_LOWER),
            64 => f32::from_le_bytes(F32_I64_LOWER),
            _ => unreachable!(),
        },
    };

    let upper = match int_ty {
        IntTy::I8 => f32::from_le_bytes(F32_I8_UPPER),
        IntTy::I16 => f32::from_le_bytes(F32_I16_UPPER),
        IntTy::I32 => f32::from_le_bytes(F32_I32_UPPER),
        IntTy::I64 => f32::from_le_bytes(F32_I64_UPPER),
        IntTy::I128 => f32::from_le_bytes(F32_I128_UPPER),
        IntTy::Isize => match mm.pointer_width {
            32 => f32::from_le_bytes(F32_I32_UPPER),
            64 => f32::from_le_bytes(F32_I64_UPPER),
            _ => unreachable!(),
        },
    };
    (lower, upper)
}

fn get_bounds_f64_int(int_ty: IntTy, mm: &MachineModel) -> (f64, f64) {
    let lower = match int_ty {
        IntTy::I8 => f64::from_le_bytes(F64_I8_LOWER),
        IntTy::I16 => f64::from_le_bytes(F64_I16_LOWER),
        IntTy::I32 => f64::from_le_bytes(F64_I32_LOWER),
        IntTy::I64 => f64::from_le_bytes(F64_I64_LOWER),
        IntTy::I128 => f64::from_le_bytes(F64_I128_LOWER),
        IntTy::Isize => match mm.pointer_width {
            32 => f64::from_le_bytes(F64_I32_LOWER),
            64 => f64::from_le_bytes(F64_I64_LOWER),
            _ => unreachable!(),
        },
    };
    let upper = match int_ty {
        IntTy::I8 => f64::from_le_bytes(F64_I8_UPPER),
        IntTy::I16 => f64::from_le_bytes(F64_I16_UPPER),
        IntTy::I32 => f64::from_le_bytes(F64_I32_UPPER),
        IntTy::I64 => f64::from_le_bytes(F64_I64_UPPER),
        IntTy::I128 => f64::from_le_bytes(F64_I128_UPPER),
        IntTy::Isize => match mm.pointer_width {
            32 => f64::from_le_bytes(F64_I32_UPPER),
            64 => f64::from_le_bytes(F64_I64_UPPER),
            _ => unreachable!(),
        },
    };
    (lower, upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use num::BigInt;
    use num::FromPrimitive;

    macro_rules! check_lower_f32 {
        ($val:ident, $min:expr) => {
            let f = f32::from_le_bytes($val);
            assert!(BigInt::from_f32(f.trunc()).unwrap() < BigInt::from($min));
            assert!(BigInt::from_f32(f.next_up().trunc()).unwrap() >= BigInt::from($min));
        };
    }

    macro_rules! check_upper_f32 {
        ($val:ident, $max:expr) => {
            let f = f32::from_le_bytes($val);
            assert!(BigInt::from_f32(f.trunc()).unwrap() > BigInt::from($max));
            assert!(BigInt::from_f32(f.next_down().trunc()).unwrap() <= BigInt::from($max));
        };
    }

    #[test]
    fn check_f32_bounds() {
        // check that the bounds are correct, i.e. that for lower (upper) bounds:
        //   1. the value when truncated is strictly smaller (larger) than i<N>::MIN or u<N>::MIN (i<N>::MAX or u<N>::MAX)
        //   2. the next higher (lower) value when truncated is greater (smaller) than or equal to i<N>::MIN or u<N>::MIN (i<N>::MAX or u<N>::MAX)

        check_lower_f32!(F32_U_LOWER, u8::MIN);

        check_upper_f32!(F32_U8_UPPER, u8::MAX);
        check_upper_f32!(F32_U16_UPPER, u16::MAX);
        check_upper_f32!(F32_U32_UPPER, u32::MAX);
        check_upper_f32!(F32_U64_UPPER, u64::MAX);
        // 128 is not needed because the upper bounds is infinity
        // Instead, check that `u128::MAX` is larger than the largest f32 value
        assert!(f32::MAX < u128::MAX as f32);

        check_lower_f32!(F32_I8_LOWER, i8::MIN);
        check_lower_f32!(F32_I16_LOWER, i16::MIN);
        check_lower_f32!(F32_I32_LOWER, i32::MIN);
        check_lower_f32!(F32_I64_LOWER, i64::MIN);
        check_lower_f32!(F32_I128_LOWER, i128::MIN);

        check_upper_f32!(F32_I8_UPPER, i8::MAX);
        check_upper_f32!(F32_I16_UPPER, i16::MAX);
        check_upper_f32!(F32_I32_UPPER, i32::MAX);
        check_upper_f32!(F32_I64_UPPER, i64::MAX);
        check_upper_f32!(F32_I128_UPPER, i128::MAX);
    }

    macro_rules! check_lower_f64 {
        ($val:ident, $min:expr) => {
            let f = f64::from_le_bytes($val);
            assert!(BigInt::from_f64(f.trunc()).unwrap() < BigInt::from($min));
            assert!(BigInt::from_f64(f.next_up().trunc()).unwrap() >= BigInt::from($min));
        };
    }

    macro_rules! check_upper_f64 {
        ($val:ident, $max:expr) => {
            let f = f64::from_le_bytes($val);
            assert!(BigInt::from_f64(f.trunc()).unwrap() > BigInt::from($max));
            assert!(BigInt::from_f64(f.next_down().trunc()).unwrap() <= BigInt::from($max));
        };
    }

    #[test]
    fn check_f64_bounds() {
        // check that the bounds are correct, i.e. that for lower (upper) bounds:
        //   1. the value when truncated is strictly smaller (larger) than {i, u}<N>::MIN ({i, u}<N>::MAX)
        //   2. the next higher (lower) value when truncated is greater (smaller) than or equal to {i, u}<N>::MIN ({i, u}<N>::MAX)

        check_lower_f64!(F64_U_LOWER, u8::MIN);

        check_upper_f64!(F64_U8_UPPER, u8::MAX);
        check_upper_f64!(F64_U16_UPPER, u16::MAX);
        check_upper_f64!(F64_U32_UPPER, u32::MAX);
        check_upper_f64!(F64_U64_UPPER, u64::MAX);
        check_upper_f64!(F64_U128_UPPER, u128::MAX);

        check_lower_f64!(F64_I8_LOWER, i8::MIN);
        check_lower_f64!(F64_I16_LOWER, i16::MIN);
        check_lower_f64!(F64_I32_LOWER, i32::MIN);
        check_lower_f64!(F64_I64_LOWER, i64::MIN);
        check_lower_f64!(F64_I128_LOWER, i128::MIN);

        check_upper_f64!(F64_I8_UPPER, i8::MAX);
        check_upper_f64!(F64_I16_UPPER, i16::MAX);
        check_upper_f64!(F64_I32_UPPER, i32::MAX);
        check_upper_f64!(F64_I64_UPPER, i64::MAX);
        check_upper_f64!(F64_I128_UPPER, i128::MAX);
    }

    macro_rules! find_upper_fn {
        ($fn_name:ident, $float_ty:ty) => {
            fn $fn_name(start: u128) -> $float_ty {
                let mut current = start + 1;
                let mut f = current as $float_ty;

                while f.trunc() as u128 <= start {
                    let f1 = (current + 1) as $float_ty;
                    let f2 = f.next_up();
                    f = if f1 > f2 { f1 } else { f2 };
                    current = f as u128;
                }
                f
            }
        };
    }

    macro_rules! find_lower_fn {
        ($fn_name:ident, $float_ty:ty) => {
            fn $fn_name(start: i128) -> $float_ty {
                let mut current = start - 1;
                let mut f = current as $float_ty;

                while f.trunc() >= start as $float_ty {
                    let f1 = (current - 1) as $float_ty;
                    let f2 = f.next_down();
                    f = if f1 < f2 { f1 } else { f2 };
                    current = f as i128;
                }
                f
            }
        };
    }

    find_lower_fn!(find_lower_f32, f32);
    find_upper_fn!(find_upper_f32, f32);
    find_lower_fn!(find_lower_f64, f64);
    find_upper_fn!(find_upper_f64, f64);

    macro_rules! find_and_print {
        (f32, $var_name:expr, $fn_name:ident, $start:expr) => {
            let f = $fn_name($start);
            let bytes = f.to_le_bytes();
            println!("const {}: [u8; 4] = [0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}]; // {f:.1}", $var_name, bytes[0], bytes[1], bytes[2], bytes[3]);
        };
        (f64, $var_name:expr, $fn_name:ident, $start:expr) => {
            let f = $fn_name($start);
            let bytes = f.to_le_bytes();
            println!("const {}: [u8; 8] = [0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}, 0x{:02X}]; // {f:.1}", $var_name, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]);
        };
    }

    #[test]
    /// This test generates most of the bounds. To run it do:
    /// `cargo test -p kani-compiler generate_bounds -- --nocapture`
    fn generate_bounds() {
        find_and_print!(f32, "F32_I8_LOWER", find_lower_f32, i8::MIN as i128);
        find_and_print!(f32, "F32_I8_UPPER", find_upper_f32, i8::MAX as u128);
        find_and_print!(f32, "F32_I16_LOWER", find_lower_f32, i16::MIN as i128);
        find_and_print!(f32, "F32_I16_UPPER", find_upper_f32, i16::MAX as u128);
        find_and_print!(f32, "F32_I32_LOWER", find_lower_f32, i32::MIN as i128);
        find_and_print!(f32, "F32_I32_UPPER", find_upper_f32, i32::MAX as u128);
        find_and_print!(f32, "F32_I64_LOWER", find_lower_f32, i64::MIN as i128);
        find_and_print!(f32, "F32_I64_UPPER", find_upper_f32, i64::MAX as u128);
        // cannot use because of overflow
        //find_and_print!(f32, "F32_I128_LOWER", find_lower_f32, i128::MIN as i128);
        find_and_print!(f32, "F32_I128_UPPER", find_upper_f32, i128::MAX as u128);
        println!();
        find_and_print!(f64, "F64_I8_LOWER", find_lower_f64, i8::MIN as i128);
        find_and_print!(f64, "F64_I8_UPPER", find_upper_f64, i8::MAX as u128);
        find_and_print!(f64, "F64_I16_LOWER", find_lower_f64, i16::MIN as i128);
        find_and_print!(f64, "F64_I16_UPPER", find_upper_f64, i16::MAX as u128);
        find_and_print!(f64, "F64_I32_LOWER", find_lower_f64, i32::MIN as i128);
        find_and_print!(f64, "F64_I32_UPPER", find_upper_f64, i32::MAX as u128);
        find_and_print!(f64, "F64_I64_LOWER", find_lower_f64, i64::MIN as i128);
        find_and_print!(f64, "F64_I64_UPPER", find_upper_f64, i64::MAX as u128);
        // cannot use because of overflow
        //find_and_print!(f64, "F64_I128_LOWER", find_lower_f64, i128::MIN as i128);
        find_and_print!(f64, "F64_I128_UPPER", find_upper_f64, i128::MAX as u128);
        println!();
        find_and_print!(f32, "F32_U_LOWER", find_lower_f32, u8::MIN as i128);
        find_and_print!(f32, "F32_U8_UPPER", find_upper_f32, u8::MAX as u128);
        find_and_print!(f32, "F32_U16_UPPER", find_upper_f32, u16::MAX as u128);
        find_and_print!(f32, "F32_U32_UPPER", find_upper_f32, u32::MAX as u128);
        find_and_print!(f32, "F32_U64_UPPER", find_upper_f32, u64::MAX as u128);
        // cannot use because of overflow
        //find_and_print!(f32, "F32_U128_UPPER", find_upper_f32, u128::MAX as u128);
        println!();
        find_and_print!(f64, "F64_U_LOWER", find_lower_f64, u8::MIN as i128);
        find_and_print!(f64, "F64_U8_UPPER", find_upper_f64, u8::MAX as u128);
        find_and_print!(f64, "F64_U16_UPPER", find_upper_f64, u16::MAX as u128);
        find_and_print!(f64, "F64_U32_UPPER", find_upper_f64, u32::MAX as u128);
        find_and_print!(f64, "F64_U64_UPPER", find_upper_f64, u64::MAX as u128);
        // cannot use because of overflow
        //find_and_print!(f64, "F64_U128_UPPER", find_upper_f64, u128::MAX as u128);
    }
}
