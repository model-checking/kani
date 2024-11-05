// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module handles intrinsics
use super::typ;
use super::{PropertyClass, bb_label};
use crate::codegen_cprover_gotoc::codegen::ty_stable::pointee_type_stable;
use crate::codegen_cprover_gotoc::{GotocCtx, utils};
use crate::intrinsics::Intrinsic;
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::MachineModel;
use cbmc::goto_program::{
    ArithmeticOverflowResult, BinaryOperator, BuiltinFn, Expr, Location, Stmt, Type,
};
use rustc_middle::ty::ParamEnv;
use rustc_middle::ty::layout::ValidityRequirement;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{BasicBlockIdx, Operand, Place};
use stable_mir::ty::{FloatTy, GenericArgs, IntTy, RigidTy, Span, Ty, TyKind, UintTy};
use tracing::debug;

pub struct SizeAlign {
    pub size: Expr,
    pub align: Expr,
}

enum VTableInfo {
    Size,
    Align,
}

impl GotocCtx<'_> {
    fn binop<F: FnOnce(Expr, Expr) -> Expr>(
        &mut self,
        place: &Place,
        mut fargs: Vec<Expr>,
        f: F,
        loc: Location,
    ) -> Stmt {
        let arg1 = fargs.remove(0);
        let arg2 = fargs.remove(0);
        let expr = f(arg1, arg2);
        self.codegen_expr_to_place_stable(place, expr, loc)
    }

    /// Given a call to an compiler intrinsic, generate the call and the `goto` terminator
    /// Note that in some cases, the intrinsic might never return (e.g. `panic`) in which case
    /// there is no terminator.
    pub fn codegen_funcall_of_intrinsic(
        &mut self,
        instance: Instance,
        args: &[Operand],
        destination: &Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        if let Some(target) = target {
            let loc = self.codegen_span_stable(span);
            let fargs = args.iter().map(|arg| self.codegen_operand_stable(arg)).collect::<Vec<_>>();
            Stmt::block(
                vec![
                    self.codegen_intrinsic(instance, fargs, destination, span),
                    Stmt::goto(bb_label(target), loc),
                ],
                loc,
            )
        } else {
            self.codegen_never_return_intrinsic(instance, span)
        }
    }

    /// Handles codegen for non returning intrinsics
    /// Non returning intrinsics are not associated with a destination
    pub fn codegen_never_return_intrinsic(&mut self, instance: Instance, span: Span) -> Stmt {
        let intrinsic = instance.intrinsic_name().unwrap();

        debug!("codegen_never_return_intrinsic:\n\tinstance {:?}\n\tspan {:?}", instance, span);

        match intrinsic.as_str() {
            "abort" => {
                self.codegen_fatal_error(PropertyClass::Assertion, "reached intrinsic::abort", span)
            }
            // Transmuting to an uninhabited type is UB.
            "transmute" => self.codegen_fatal_error(
                PropertyClass::SafetyCheck,
                "transmuting to uninhabited type has undefined behavior",
                span,
            ),
            _ => self.codegen_fatal_error(
                PropertyClass::UnsupportedConstruct,
                &format!("Unsupported intrinsic {intrinsic}"),
                span,
            ),
        }
    }

    /// c.f. `rustc_codegen_llvm::intrinsic` `impl IntrinsicCallMethods<'tcx>
    /// for Builder<'a, 'll, 'tcx>` `fn codegen_intrinsic_call` c.f.
    /// <https://doc.rust-lang.org/std/intrinsics/index.html>
    ///
    /// ### A note on type checking
    ///
    /// The backend/codegen generally assumes that at this point arguments have
    /// been type checked and that the given intrinsic is safe to call with the
    /// provided arguments. However in rare cases the intrinsics type signature
    /// is too permissive or has to be liberal because the types are enforced by
    /// the specific code gen/backend. In such cases we handle the type checking
    /// here. The type constraints enforced here must be at least as strict as
    /// the assertions made in in the builder functions in
    /// [`Expr`].
    fn codegen_intrinsic(
        &mut self,
        instance: Instance,
        mut fargs: Vec<Expr>,
        place: &Place,
        span: Span,
    ) -> Stmt {
        let intrinsic_name = instance.intrinsic_name().unwrap();
        let intrinsic_str = intrinsic_name.as_str();
        let loc = self.codegen_span_stable(span);
        debug!(?instance, "codegen_intrinsic");
        debug!(?fargs, "codegen_intrinsic");
        debug!(?place, "codegen_intrinsic");
        debug!(?span, "codegen_intrinsic");
        let sig = instance.ty().kind().fn_sig().unwrap().skip_binder();
        let ret_ty = sig.output();
        let farg_types = sig.inputs();
        let cbmc_ret_ty = self.codegen_ty_stable(ret_ty);

        // Codegens a simple intrinsic: ie. one which maps directly to a matching goto construct
        // We need to use this macro form because of a known limitation in rust
        // `codegen_simple_intrinsic!(self.get_sqrt(), Type::float())` gives the error message:
        //   error[E0499]: cannot borrow `*self` as mutable more than once at a time
        //    --> src/librustc_codegen_llvm/gotoc/intrinsic.rs:76:63
        //    |
        // 76 |                 codegen_simple_intrinsic!(self.get_sqrt(), Type::double())
        //    |                 ---- ------------------------                 ^^^^ second mutable borrow occurs here
        //    |                 |    |
        //    |                 |    first borrow later used by call
        //    |                 first mutable borrow occurs here

        //  To solve this, we need to store the `self.get_sqrt()` into a temporary variable.
        //  Using the macro form allows us to keep the call as a oneliner, while still making rust happy.
        //  TODO: https://github.com/model-checking/kani/issues/5
        macro_rules! codegen_simple_intrinsic {
            ($f:ident) => {{
                let mm = self.symbol_table.machine_model();
                let casted_fargs =
                    Expr::cast_arguments_to_target_equivalent_function_parameter_types(
                        &BuiltinFn::$f.as_expr(),
                        fargs,
                        mm,
                    );
                let expr = BuiltinFn::$f.call(casted_fargs, loc);
                self.codegen_expr_to_place_stable(place, expr, loc)
            }};
        }

        // Intrinsics which encode a division operation with overflow check
        macro_rules! codegen_op_with_div_overflow_check {
            ($f:ident) => {{
                let a = fargs.remove(0);
                let b = fargs.remove(0);
                let div_does_not_overflow = self.div_does_not_overflow(a.clone(), b.clone());
                let div_overflow_check = self.codegen_assert_assume(
                    div_does_not_overflow,
                    PropertyClass::ArithmeticOverflow,
                    format!("attempt to compute {} which would overflow", intrinsic_str).as_str(),
                    loc,
                );
                let res = a.$f(b);
                let expr_place = self.codegen_expr_to_place_stable(place, res, loc);
                Stmt::block(vec![div_overflow_check, expr_place], loc)
            }};
        }

        // Intrinsics which encode a simple wrapping arithmetic operation
        macro_rules! codegen_wrapping_op {
            ($f:ident) => {{ codegen_intrinsic_binop!($f) }};
        }

        // Intrinsics which encode a simple binary operation
        macro_rules! codegen_intrinsic_binop {
            ($f:ident) => {{ self.binop(place, fargs, |a, b| a.$f(b), loc) }};
        }

        // Intrinsics which encode a simple binary operation which need a machine model
        macro_rules! codegen_intrinsic_binop_with_mm {
            ($f:ident) => {{
                let arg1 = fargs.remove(0);
                let arg2 = fargs.remove(0);
                let expr = arg1.$f(arg2, self.symbol_table.machine_model());
                self.codegen_expr_to_place_stable(place, expr, loc)
            }};
        }

        // Intrinsics which encode count intrinsics (ctlz, cttz)
        // The `allow_zero` flag determines if calling these builtins with 0 causes UB
        macro_rules! codegen_count_intrinsic {
            ($builtin: ident, $allow_zero: expr) => {{
                let arg = fargs.remove(0);
                self.codegen_expr_to_place_stable(place, arg.$builtin($allow_zero), loc)
            }};
        }

        // Intrinsics which encode a value known during compilation
        macro_rules! codegen_intrinsic_const {
            () => {{
                let place_ty = self.place_ty_stable(&place);
                let stable_instance = instance;
                let alloc = stable_instance.try_const_eval(place_ty).unwrap();
                // We assume that the intrinsic has type checked at this point, so
                // we can use the place type as the expression type.
                let expr = self.codegen_allocation(&alloc, place_ty, loc);
                self.codegen_expr_to_place_stable(&place, expr, loc)
            }};
        }

        macro_rules! codegen_size_align {
            ($which: ident) => {{
                let args = instance_args(&instance);
                // The type `T` that we'll compute the size or alignment.
                let target_ty = args.0[0].expect_ty();
                let arg = fargs.remove(0);
                let size_align = self.size_and_align_of_dst(*target_ty, arg);
                self.codegen_expr_to_place_stable(place, size_align.$which, loc)
            }};
        }

        // Most atomic intrinsics do:
        //   1. Perform an operation on a primary argument (e.g., addition)
        //   2. Return the previous value of the primary argument
        // The primary argument is always passed by reference. In a sequential
        // context, atomic orderings can be ignored.
        //
        // Atomic binops are transformed as follows:
        // -------------------------
        // var = atomic_op(var1, var2)
        // -------------------------
        // unsigned char tmp;
        // tmp = *var1;
        // *var1 = op(*var1, var2);
        // var = tmp;
        // -------------------------
        //
        // In fetch functions of atomic_ptr such as https://doc.rust-lang.org/std/sync/atomic/struct.AtomicPtr.html#method.fetch_byte_add,
        // the type of var2 can be pointer (invalid_mut).
        // In such case, atomic binops are transformed as follows to avoid typecheck failure.
        // -------------------------
        // var = atomic_op(var1, var2)
        // -------------------------
        // unsigned char tmp;
        // tmp = *var1;
        // *var1 = (typeof var1)op((size_t)*var1, (size_t)var2);
        // var = tmp;
        // -------------------------
        //
        // Note: Atomic arithmetic operations wrap around on overflow.
        macro_rules! codegen_atomic_binop {
            ($op: ident) => {{
                let loc = self.codegen_span_stable(span);
                self.store_concurrent_construct(intrinsic_str, loc);
                let var1_ref = fargs.remove(0);
                let var1 = var1_ref.dereference();
                let (tmp, decl_stmt) =
                    self.decl_temp_variable(var1.typ().clone(), Some(var1.to_owned()), loc);
                let var2 = fargs.remove(0);
                let op_expr = if var2.typ().is_pointer() {
                    (var1.clone().cast_to(Type::c_size_t()))
                        .$op(var2.cast_to(Type::c_size_t()))
                        .with_location(loc)
                        .cast_to(var1.typ().clone())
                } else {
                    (var1.clone()).$op(var2).with_location(loc)
                };
                let assign_stmt = (var1.clone()).assign(op_expr, loc);
                let res_stmt = self.codegen_expr_to_place_stable(place, tmp.clone(), loc);
                Stmt::atomic_block(vec![decl_stmt, assign_stmt, res_stmt], loc)
            }};
        }

        macro_rules! unstable_codegen {
            ($($tt:tt)*) => {{
                let expr = self.codegen_unimplemented_expr(
                    &format!("'{}' intrinsic", intrinsic_str),
                    cbmc_ret_ty,
                    loc,
                    "https://github.com/model-checking/kani/issues/new/choose",
                );
                self.codegen_expr_to_place_stable(place, expr, loc)
            }};
        }

        let intrinsic = Intrinsic::from_instance(&instance);

        match intrinsic {
            Intrinsic::AddWithOverflow => {
                self.codegen_op_with_overflow(BinaryOperator::OverflowResultPlus, fargs, place, loc)
            }
            Intrinsic::ArithOffset => self.codegen_arith_offset(fargs, place, loc),
            Intrinsic::AssertInhabited => {
                self.codegen_assert_intrinsic(instance, intrinsic_str, span)
            }
            Intrinsic::AssertMemUninitializedValid => {
                self.codegen_assert_intrinsic(instance, intrinsic_str, span)
            }
            Intrinsic::AssertZeroValid => {
                self.codegen_assert_intrinsic(instance, intrinsic_str, span)
            }
            // https://doc.rust-lang.org/core/intrinsics/fn.assume.html
            // Informs the optimizer that a condition is always true.
            // If the condition is false, the behavior is undefined.
            Intrinsic::Assume => self.codegen_assert_assume(
                fargs.remove(0).cast_to(Type::bool()),
                PropertyClass::Assume,
                "assumption failed",
                loc,
            ),
            Intrinsic::AtomicAnd(_) => codegen_atomic_binop!(bitand),
            Intrinsic::AtomicCxchg(_) | Intrinsic::AtomicCxchgWeak(_) => {
                self.codegen_atomic_cxchg(intrinsic_str, fargs, place, loc)
            }

            Intrinsic::AtomicFence(_) => self.codegen_atomic_noop(intrinsic_str, loc),
            Intrinsic::AtomicLoad(_) => self.codegen_atomic_load(intrinsic_str, fargs, place, loc),
            Intrinsic::AtomicMax(_) => codegen_atomic_binop!(max),
            Intrinsic::AtomicMin(_) => codegen_atomic_binop!(min),
            Intrinsic::AtomicNand(_) => codegen_atomic_binop!(bitnand),
            Intrinsic::AtomicOr(_) => codegen_atomic_binop!(bitor),
            Intrinsic::AtomicSingleThreadFence(_) => self.codegen_atomic_noop(intrinsic_str, loc),
            Intrinsic::AtomicStore(_) => {
                self.codegen_atomic_store(intrinsic_str, fargs, place, loc)
            }
            Intrinsic::AtomicUmax(_) => codegen_atomic_binop!(max),
            Intrinsic::AtomicUmin(_) => codegen_atomic_binop!(min),
            Intrinsic::AtomicXadd(_) => codegen_atomic_binop!(plus),
            Intrinsic::AtomicXchg(_) => self.codegen_atomic_store(intrinsic_str, fargs, place, loc),
            Intrinsic::AtomicXor(_) => codegen_atomic_binop!(bitxor),
            Intrinsic::AtomicXsub(_) => codegen_atomic_binop!(sub),
            Intrinsic::Bitreverse => {
                self.codegen_expr_to_place_stable(place, fargs.remove(0).bitreverse(), loc)
            }
            // black_box is an identity function that hints to the compiler
            // to be maximally pessimistic to limit optimizations
            Intrinsic::BlackBox => self.codegen_expr_to_place_stable(place, fargs.remove(0), loc),
            Intrinsic::Breakpoint => Stmt::skip(loc),
            Intrinsic::Bswap => {
                self.codegen_expr_to_place_stable(place, fargs.remove(0).bswap(), loc)
            }
            Intrinsic::CeilF32 => codegen_simple_intrinsic!(Ceilf),
            Intrinsic::CeilF64 => codegen_simple_intrinsic!(Ceil),
            Intrinsic::CompareBytes => self.codegen_compare_bytes(fargs, place, loc),
            Intrinsic::Copy => {
                self.codegen_copy(intrinsic_str, false, fargs, farg_types, Some(place), loc)
            }
            Intrinsic::CopySignF32 => codegen_simple_intrinsic!(Copysignf),
            Intrinsic::CopySignF64 => codegen_simple_intrinsic!(Copysign),
            Intrinsic::CosF32 => codegen_simple_intrinsic!(Cosf),
            Intrinsic::CosF64 => codegen_simple_intrinsic!(Cos),
            Intrinsic::Ctlz => codegen_count_intrinsic!(ctlz, true),
            Intrinsic::CtlzNonZero => codegen_count_intrinsic!(ctlz, false),
            Intrinsic::Ctpop => self.codegen_ctpop(place, span, fargs.remove(0), farg_types[0]),
            Intrinsic::Cttz => codegen_count_intrinsic!(cttz, true),
            Intrinsic::CttzNonZero => codegen_count_intrinsic!(cttz, false),
            Intrinsic::DiscriminantValue => {
                let sig = instance.ty().kind().fn_sig().unwrap().skip_binder();
                let ty = pointee_type_stable(sig.inputs()[0]).unwrap();
                let e = self.codegen_get_discriminant(fargs.remove(0).dereference(), ty, ret_ty);
                self.codegen_expr_to_place_stable(place, e, loc)
            }
            Intrinsic::ExactDiv => self.codegen_exact_div(fargs, place, loc),
            Intrinsic::Exp2F32 => codegen_simple_intrinsic!(Exp2f),
            Intrinsic::Exp2F64 => codegen_simple_intrinsic!(Exp2),
            Intrinsic::ExpF32 => codegen_simple_intrinsic!(Expf),
            Intrinsic::ExpF64 => codegen_simple_intrinsic!(Exp),
            Intrinsic::FabsF32 => codegen_simple_intrinsic!(Fabsf),
            Intrinsic::FabsF64 => codegen_simple_intrinsic!(Fabs),
            Intrinsic::FaddFast => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(plus);
                self.add_finite_args_checks(intrinsic_str, fargs_clone, binop_stmt, span)
            }
            Intrinsic::FdivFast => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(div);
                self.add_finite_args_checks(intrinsic_str, fargs_clone, binop_stmt, span)
            }
            Intrinsic::FloatToIntUnchecked => self.codegen_float_to_int_unchecked(
                intrinsic_str,
                fargs.remove(0),
                farg_types[0],
                place,
                ret_ty,
                loc,
            ),
            Intrinsic::FloorF32 => codegen_simple_intrinsic!(Floorf),
            Intrinsic::FloorF64 => codegen_simple_intrinsic!(Floor),
            Intrinsic::FmafF32 => codegen_simple_intrinsic!(Fmaf),
            Intrinsic::FmafF64 => codegen_simple_intrinsic!(Fma),
            Intrinsic::FmulFast => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(mul);
                self.add_finite_args_checks(intrinsic_str, fargs_clone, binop_stmt, span)
            }
            Intrinsic::Forget => Stmt::skip(loc),
            Intrinsic::FsubFast => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(sub);
                self.add_finite_args_checks(intrinsic_str, fargs_clone, binop_stmt, span)
            }
            Intrinsic::IsValStaticallyKnown => {
                // Returning false is sound according do this intrinsic's documentation:
                // https://doc.rust-lang.org/nightly/std/intrinsics/fn.is_val_statically_known.html
                self.codegen_expr_to_place_stable(place, Expr::c_false(), loc)
            }
            Intrinsic::Likely => self.codegen_expr_to_place_stable(place, fargs.remove(0), loc),
            Intrinsic::Log10F32 => codegen_simple_intrinsic!(Log10f),
            Intrinsic::Log10F64 => codegen_simple_intrinsic!(Log10),
            Intrinsic::Log2F32 => codegen_simple_intrinsic!(Log2f),
            Intrinsic::Log2F64 => codegen_simple_intrinsic!(Log2),
            Intrinsic::LogF32 => codegen_simple_intrinsic!(Logf),
            Intrinsic::LogF64 => codegen_simple_intrinsic!(Log),
            Intrinsic::MaxNumF32 => codegen_simple_intrinsic!(Fmaxf),
            Intrinsic::MaxNumF64 => codegen_simple_intrinsic!(Fmax),
            Intrinsic::MinAlignOf => codegen_intrinsic_const!(),
            Intrinsic::MinAlignOfVal => codegen_size_align!(align),
            Intrinsic::MinNumF32 => codegen_simple_intrinsic!(Fminf),
            Intrinsic::MinNumF64 => codegen_simple_intrinsic!(Fmin),
            Intrinsic::MulWithOverflow => {
                self.codegen_op_with_overflow(BinaryOperator::OverflowResultMult, fargs, place, loc)
            }
            Intrinsic::NearbyIntF32 => codegen_simple_intrinsic!(Nearbyintf),
            Intrinsic::NearbyIntF64 => codegen_simple_intrinsic!(Nearbyint),
            Intrinsic::NeedsDrop => codegen_intrinsic_const!(),
            Intrinsic::PowF32 => codegen_simple_intrinsic!(Powf),
            Intrinsic::PowF64 => codegen_simple_intrinsic!(Pow),
            Intrinsic::PowIF32 => codegen_simple_intrinsic!(Powif),
            Intrinsic::PowIF64 => codegen_simple_intrinsic!(Powi),
            Intrinsic::PrefAlignOf => codegen_intrinsic_const!(),
            Intrinsic::PtrGuaranteedCmp => self.codegen_ptr_guaranteed_cmp(fargs, place, loc),
            Intrinsic::PtrOffsetFrom => self.codegen_ptr_offset_from(fargs, place, loc),
            Intrinsic::PtrOffsetFromUnsigned => {
                self.codegen_ptr_offset_from_unsigned(fargs, place, loc)
            }
            Intrinsic::RawEq => self.codegen_intrinsic_raw_eq(instance, fargs, place, loc),
            Intrinsic::RetagBoxToRaw => self.codegen_retag_box_to_raw(fargs, place, loc),
            Intrinsic::RintF32 => codegen_simple_intrinsic!(Rintf),
            Intrinsic::RintF64 => codegen_simple_intrinsic!(Rint),
            Intrinsic::RotateLeft => codegen_intrinsic_binop!(rol),
            Intrinsic::RotateRight => codegen_intrinsic_binop!(ror),
            Intrinsic::RoundF32 => codegen_simple_intrinsic!(Roundf),
            Intrinsic::RoundF64 => codegen_simple_intrinsic!(Round),
            Intrinsic::SaturatingAdd => codegen_intrinsic_binop_with_mm!(saturating_add),
            Intrinsic::SaturatingSub => codegen_intrinsic_binop_with_mm!(saturating_sub),
            Intrinsic::SinF32 => codegen_simple_intrinsic!(Sinf),
            Intrinsic::SinF64 => codegen_simple_intrinsic!(Sin),
            Intrinsic::SimdAdd => self.codegen_simd_op_with_overflow(
                Expr::plus,
                Expr::add_overflow_p,
                fargs,
                intrinsic_str,
                place,
                loc,
            ),
            Intrinsic::SimdAnd => codegen_intrinsic_binop!(bitand),
            // TODO: `simd_rem` doesn't check for overflow cases for floating point operands.
            // <https://github.com/model-checking/kani/pull/2645>
            Intrinsic::SimdDiv | Intrinsic::SimdRem => {
                self.codegen_simd_div_with_overflow(fargs, intrinsic_str, place, loc)
            }
            Intrinsic::SimdEq => {
                self.codegen_simd_cmp(Expr::vector_eq, fargs, place, span, farg_types, ret_ty)
            }
            Intrinsic::SimdExtract => {
                self.codegen_intrinsic_simd_extract(fargs, place, farg_types, ret_ty, span)
            }
            Intrinsic::SimdGe => {
                self.codegen_simd_cmp(Expr::vector_ge, fargs, place, span, farg_types, ret_ty)
            }
            Intrinsic::SimdGt => {
                self.codegen_simd_cmp(Expr::vector_gt, fargs, place, span, farg_types, ret_ty)
            }
            Intrinsic::SimdInsert => {
                self.codegen_intrinsic_simd_insert(fargs, place, cbmc_ret_ty, farg_types, span, loc)
            }
            Intrinsic::SimdLe => {
                self.codegen_simd_cmp(Expr::vector_le, fargs, place, span, farg_types, ret_ty)
            }
            Intrinsic::SimdLt => {
                self.codegen_simd_cmp(Expr::vector_lt, fargs, place, span, farg_types, ret_ty)
            }
            Intrinsic::SimdMul => self.codegen_simd_op_with_overflow(
                Expr::mul,
                Expr::mul_overflow_p,
                fargs,
                intrinsic_str,
                place,
                loc,
            ),
            Intrinsic::SimdNe => {
                self.codegen_simd_cmp(Expr::vector_neq, fargs, place, span, farg_types, ret_ty)
            }
            Intrinsic::SimdOr => codegen_intrinsic_binop!(bitor),
            Intrinsic::SimdShl | Intrinsic::SimdShr => {
                self.codegen_simd_shift_with_distance_check(fargs, intrinsic_str, place, loc)
            }
            Intrinsic::SimdShuffle(stripped) => {
                let n: u64 = self.simd_shuffle_length(stripped.as_str(), farg_types, span);
                self.codegen_intrinsic_simd_shuffle(fargs, place, farg_types, ret_ty, n, span)
            }
            Intrinsic::SimdSub => self.codegen_simd_op_with_overflow(
                Expr::sub,
                Expr::sub_overflow_p,
                fargs,
                intrinsic_str,
                place,
                loc,
            ),
            Intrinsic::SimdXor => codegen_intrinsic_binop!(bitxor),
            Intrinsic::SizeOfVal => codegen_size_align!(size),
            Intrinsic::SqrtF32 => codegen_simple_intrinsic!(Sqrtf),
            Intrinsic::SqrtF64 => codegen_simple_intrinsic!(Sqrt),
            Intrinsic::SubWithOverflow => self.codegen_op_with_overflow(
                BinaryOperator::OverflowResultMinus,
                fargs,
                place,
                loc,
            ),
            Intrinsic::Transmute => self.codegen_intrinsic_transmute(fargs, ret_ty, place, loc),
            Intrinsic::TruncF32 => codegen_simple_intrinsic!(Truncf),
            Intrinsic::TruncF64 => codegen_simple_intrinsic!(Trunc),
            Intrinsic::TypeId => codegen_intrinsic_const!(),
            Intrinsic::TypeName => codegen_intrinsic_const!(),
            Intrinsic::TypedSwap => self.codegen_swap(fargs, farg_types, loc),
            Intrinsic::UnalignedVolatileLoad => {
                unstable_codegen!(self.codegen_expr_to_place_stable(
                    place,
                    fargs.remove(0).dereference(),
                    loc
                ))
            }
            Intrinsic::UncheckedDiv => codegen_op_with_div_overflow_check!(div),
            Intrinsic::UncheckedRem => codegen_op_with_div_overflow_check!(rem),
            Intrinsic::Unlikely => self.codegen_expr_to_place_stable(place, fargs.remove(0), loc),
            Intrinsic::VolatileCopyMemory => unstable_codegen!(codegen_intrinsic_copy!(Memmove)),
            Intrinsic::VolatileCopyNonOverlappingMemory => {
                unstable_codegen!(codegen_intrinsic_copy!(Memcpy))
            }
            Intrinsic::VolatileLoad => self.codegen_volatile_load(fargs, farg_types, place, loc),
            Intrinsic::VolatileStore => {
                assert!(self.place_ty_stable(place).kind().is_unit());
                self.codegen_volatile_store(fargs, farg_types, loc)
            }
            Intrinsic::VtableSize => self.vtable_info(VTableInfo::Size, fargs, place, loc),
            Intrinsic::VtableAlign => self.vtable_info(VTableInfo::Align, fargs, place, loc),
            Intrinsic::WrappingAdd => codegen_wrapping_op!(plus),
            Intrinsic::WrappingMul => codegen_wrapping_op!(mul),
            Intrinsic::WrappingSub => codegen_wrapping_op!(sub),
            Intrinsic::WriteBytes => {
                assert!(self.place_ty_stable(place).kind().is_unit());
                self.codegen_write_bytes(fargs, farg_types, loc)
            }
            // Unimplemented
            Intrinsic::Unimplemented { name, issue_link } => {
                self.codegen_unimplemented_stmt(&name, loc, &issue_link)
            }
        }
    }

    /// Perform type checking and code generation for the `ctpop` rust intrinsic.
    fn codegen_ctpop(
        &mut self,
        target_place: &Place,
        span: Span,
        arg: Expr,
        arg_rust_ty: Ty,
    ) -> Stmt {
        if !arg.typ().is_integer() {
            self.intrinsics_typecheck_fail(span, "ctpop", "integer type", arg_rust_ty)
        } else {
            let loc = self.codegen_span_stable(span);
            self.codegen_expr_to_place_stable(&target_place, arg.popcount(), loc)
        }
    }

    /// Report that a delayed type check on an intrinsic failed.
    ///
    /// The idea is to blame one of the arguments on the failed type check and
    /// report the type that was found for that argument in `actual`. The
    /// `expected` type for that argument can be very permissive (e.g. "some
    /// integer type") and as a result it allows a permissive string as
    /// description.
    ///
    /// Calling this function will abort the compilation though that is not
    /// obvious by the type.
    fn intrinsics_typecheck_fail(&self, span: Span, name: &str, expected: &str, actual: Ty) -> ! {
        utils::span_err(
            self.tcx,
            span,
            format!(
                "Type check failed for intrinsic `{name}`: Expected {expected}, found {}",
                self.pretty_ty(actual)
            ),
        );
        self.tcx.dcx().abort_if_errors();
        unreachable!("Rustc should have aborted already")
    }

    // Fast math intrinsics for floating point operations like `fadd_fast`
    // assume that their inputs are finite:
    // https://doc.rust-lang.org/std/intrinsics/fn.fadd_fast.html
    // This function adds assertions to the statement which performs the
    // operation and checks for overflow failures.
    fn add_finite_args_checks(
        &mut self,
        intrinsic: &str,
        mut fargs: Vec<Expr>,
        stmt: Stmt,
        span: Span,
    ) -> Stmt {
        let arg1 = fargs.remove(0);
        let arg2 = fargs.remove(0);
        let msg1 = format!("first argument for {intrinsic} is finite");
        let msg2 = format!("second argument for {intrinsic} is finite");
        let loc = self.codegen_span_stable(span);
        let finite_check1 = self.codegen_assert_assume(
            arg1.is_finite(),
            PropertyClass::FiniteCheck,
            msg1.as_str(),
            loc,
        );
        let finite_check2 = self.codegen_assert_assume(
            arg2.is_finite(),
            PropertyClass::FiniteCheck,
            msg2.as_str(),
            loc,
        );
        Stmt::block(vec![finite_check1, finite_check2, stmt], loc)
    }

    fn div_does_not_overflow(&self, a: Expr, b: Expr) -> Expr {
        let mm = self.symbol_table.machine_model();
        let atyp = a.typ();
        let btyp = b.typ();
        let dividend_is_int_min = if atyp.is_signed(mm) {
            a.clone().eq(atyp.min_int_expr(mm))
        } else {
            Expr::bool_false()
        };
        let divisor_is_minus_one =
            if btyp.is_signed(mm) { b.clone().eq(btyp.one().neg()) } else { Expr::bool_false() };
        dividend_is_int_min.and(divisor_is_minus_one).not()
    }

    /// Intrinsics of the form *_with_overflow
    fn codegen_op_with_overflow(
        &mut self,
        binop: BinaryOperator,
        mut fargs: Vec<Expr>,
        place: &Place,
        loc: Location,
    ) -> Stmt {
        let place_ty = self.place_ty_stable(place);
        let result_type = self.codegen_ty_stable(place_ty);
        let left = fargs.remove(0);
        let right = fargs.remove(0);
        let res = self.codegen_binop_with_overflow(binop, left, right, result_type.clone(), loc);
        self.codegen_expr_to_place_stable(
            place,
            Expr::statement_expression(vec![res.as_stmt(loc)], result_type, loc),
            loc,
        )
    }

    fn codegen_exact_div(&mut self, mut fargs: Vec<Expr>, p: &Place, loc: Location) -> Stmt {
        // Check for undefined behavior conditions defined in
        // https://doc.rust-lang.org/std/intrinsics/fn.exact_div.html
        let a = fargs.remove(0);
        let b = fargs.remove(0);
        let atyp = a.typ();
        let btyp = b.typ();
        let division_is_exact = a.clone().rem(b.clone()).eq(atyp.zero());
        let divisor_is_nonzero = b.clone().neq(btyp.zero());
        let division_does_not_overflow = self.div_does_not_overflow(a.clone(), b.clone());
        Stmt::block(
            vec![
                self.codegen_assert_assume(
                    division_is_exact,
                    PropertyClass::ExactDiv,
                    "exact_div arguments divide exactly",
                    loc,
                ),
                self.codegen_assert_assume(
                    divisor_is_nonzero,
                    PropertyClass::ExactDiv,
                    "exact_div divisor is nonzero",
                    loc,
                ),
                self.codegen_assert_assume(
                    division_does_not_overflow,
                    PropertyClass::ExactDiv,
                    "exact_div division does not overflow",
                    loc,
                ),
                self.codegen_expr_to_place_stable(p, a.div(b), loc),
            ],
            loc,
        )
    }

    /// Generates either a panic or no-op for `assert_*` intrinsics.
    /// These are intrinsics that statically compile to panics if the type
    /// layout is invalid so we get a message that mentions the offending type.
    ///
    /// <https://doc.rust-lang.org/std/intrinsics/fn.assert_inhabited.html>
    /// <https://doc.rust-lang.org/std/intrinsics/fn.assert_mem_uninitialized_valid.html>
    /// <https://doc.rust-lang.org/std/intrinsics/fn.assert_zero_valid.html>
    fn codegen_assert_intrinsic(
        &mut self,
        instance: Instance,
        intrinsic: &str,
        span: Span,
    ) -> Stmt {
        // Get the type `T` from the `assert_fn<T>` definition.
        let args = instance_args(&instance);
        let target_ty = args.0[0].expect_ty();
        let layout = self.layout_of_stable(*target_ty);
        // Note: We follow the pattern seen in `codegen_panic_intrinsic` from `rustc_codegen_ssa`
        // https://github.com/rust-lang/rust/blob/master/compiler/rustc_codegen_ssa/src/mir/block.rs

        // For all intrinsics we first check `is_uninhabited` to give a more
        // precise error message
        if layout.is_uninhabited() {
            return self.codegen_fatal_error(
                PropertyClass::SafetyCheck,
                &format!(
                    "attempted to instantiate uninhabited type `{}`",
                    self.pretty_ty(*target_ty)
                ),
                span,
            );
        }

        let param_env_and_type =
            ParamEnv::reveal_all().and(rustc_internal::internal(self.tcx, target_ty));

        // Then we check if the type allows "raw" initialization for the cases
        // where memory is zero-initialized or entirely uninitialized
        if intrinsic == "assert_zero_valid"
            && !self
                .tcx
                .check_validity_requirement((ValidityRequirement::Zero, param_env_and_type))
                .unwrap()
        {
            return self.codegen_fatal_error(
                PropertyClass::SafetyCheck,
                &format!(
                    "attempted to zero-initialize type `{}`, which is invalid",
                    self.pretty_ty(*target_ty)
                ),
                span,
            );
        }

        if intrinsic == "assert_mem_uninitialized_valid"
            && !self
                .tcx
                .check_validity_requirement((
                    ValidityRequirement::UninitMitigated0x01Fill,
                    param_env_and_type,
                ))
                .unwrap()
        {
            return self.codegen_fatal_error(
                PropertyClass::SafetyCheck,
                &format!(
                    "attempted to leave type `{}` uninitialized, which is invalid",
                    self.pretty_ty(*target_ty)
                ),
                span,
            );
        }

        // Otherwise we generate a no-op statement
        let loc = self.codegen_span_stable(span);
        Stmt::skip(loc)
    }

    /// An atomic load simply returns the value referenced
    /// in its argument (as in other atomic operations)
    /// -------------------------
    /// var = atomic_load(var1)
    /// -------------------------
    /// var = *var1;
    /// -------------------------
    fn codegen_atomic_load(
        &mut self,
        intrinsic: &str,
        mut fargs: Vec<Expr>,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        self.store_concurrent_construct(intrinsic, loc);
        let var1_ref = fargs.remove(0);
        let var1 = var1_ref.dereference().with_location(loc);
        let res_stmt = self.codegen_expr_to_place_stable(p, var1, loc);
        Stmt::atomic_block(vec![res_stmt], loc)
    }

    /// An atomic compare-and-exchange updates the value referenced in
    /// its primary argument and returns a tuple that contains:
    ///  * the previous value
    ///  * a boolean value indicating whether the operation was successful or not
    ///
    /// In a sequential context, the update is always sucessful so we assume the
    /// second value to be true.
    /// -------------------------
    /// var = atomic_cxchg(var1, var2, var3)
    /// -------------------------
    /// unsigned char tmp;
    /// tmp = *var1;
    /// if (*var1 == var2) *var1 = var3;
    /// var = (tmp, true);
    /// -------------------------
    fn codegen_atomic_cxchg(
        &mut self,
        intrinsic: &str,
        mut fargs: Vec<Expr>,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        self.store_concurrent_construct(intrinsic, loc);
        let var1_ref = fargs.remove(0);
        let var1 = var1_ref.dereference().with_location(loc);
        let (tmp, decl_stmt) =
            self.decl_temp_variable(var1.typ().clone(), Some(var1.to_owned()), loc);
        let var2 = fargs.remove(0).with_location(loc);
        let var3 = fargs.remove(0).with_location(loc);
        let eq_expr = (var1.clone()).eq(var2);
        let assign_stmt = var1.assign(var3, loc);
        let cond_update_stmt = Stmt::if_then_else(eq_expr, assign_stmt, None, loc);
        let place_type = self.place_ty_stable(p);
        let res_type = self.codegen_ty_stable(place_type);
        let tuple_expr =
            Expr::struct_expr_from_values(res_type, vec![tmp, Expr::c_true()], &self.symbol_table)
                .with_location(loc);
        let res_stmt = self.codegen_expr_to_place_stable(p, tuple_expr, loc);
        Stmt::atomic_block(vec![decl_stmt, cond_update_stmt, res_stmt], loc)
    }

    /// An atomic store updates the value referenced in
    /// its primary argument and returns its previous value
    /// -------------------------
    /// var = atomic_store(var1, var2)
    /// -------------------------
    /// unsigned char tmp;
    /// tmp = *var1;
    /// *var1 = var2;
    /// var = tmp;
    /// -------------------------
    fn codegen_atomic_store(
        &mut self,
        intrinsic: &str,
        mut fargs: Vec<Expr>,
        place: &Place,
        loc: Location,
    ) -> Stmt {
        self.store_concurrent_construct(intrinsic, loc);
        let var1_ref = fargs.remove(0);
        let var1 = var1_ref.dereference().with_location(loc);
        let (tmp, decl_stmt) =
            self.decl_temp_variable(var1.typ().clone(), Some(var1.to_owned()), loc);
        let var2 = fargs.remove(0).with_location(loc);
        let assign_stmt = var1.assign(var2, loc);
        let res_stmt = self.codegen_expr_to_place_stable(place, tmp, loc);
        Stmt::atomic_block(vec![decl_stmt, assign_stmt, res_stmt], loc)
    }

    /// Atomic no-ops (e.g., atomic_fence) are transformed into SKIP statements
    fn codegen_atomic_noop(&mut self, intrinsic: &str, loc: Location) -> Stmt {
        self.store_concurrent_construct(intrinsic, loc);
        let skip_stmt = Stmt::skip(loc);
        Stmt::atomic_block(vec![skip_stmt], loc)
    }

    /// Copies `count * size_of::<T>()` bytes from `src` to `dst`.
    ///
    /// Note that this function handles code generation for:
    ///  1. The `copy` intrinsic.
    ///     <https://doc.rust-lang.org/core/intrinsics/fn.copy.html>
    ///  2. The `CopyNonOverlapping` statement.
    ///     <https://doc.rust-lang.org/core/intrinsics/fn.copy_nonoverlapping.html>
    ///
    /// Undefined behavior if any of these conditions are violated:
    ///  * Both `src`/`dst` must be properly aligned (done by alignment checks)
    ///  * Both `src`/`dst` must be valid for reads/writes of `count *
    ///      size_of::<T>()` bytes (done by calls to `memmove`)
    ///  * (Exclusive to nonoverlapping copy) The region of memory beginning
    ///    at `src` with a size of `count * size_of::<T>()` bytes must *not*
    ///    overlap with the region of memory beginning at `dst` with the same
    ///    size.
    ///
    /// In addition, we check that computing `count` in bytes (i.e., the third
    /// argument of the copy built-in call) would not overflow.
    pub fn codegen_copy(
        &mut self,
        intrinsic: &str,
        is_non_overlapping: bool,
        mut fargs: Vec<Expr>,
        farg_types: &[Ty],
        p: Option<&Place>,
        loc: Location,
    ) -> Stmt {
        // The two first arguments are pointers. It's safe to cast them to void
        // pointers or directly unwrap the `pointee_type` result as seen later.
        let src = fargs.remove(0).cast_to(Type::void_pointer());
        let dst = fargs.remove(0).cast_to(Type::void_pointer());

        // Generate alignment checks for both pointers
        let src_align = self.is_ptr_aligned(farg_types[0], src.clone());
        let src_align_check = self.codegen_assert_assume(
            src_align,
            PropertyClass::SafetyCheck,
            "`src` must be properly aligned",
            loc,
        );
        let dst_align = self.is_ptr_aligned(farg_types[1], dst.clone());
        let dst_align_check = self.codegen_assert_assume(
            dst_align,
            PropertyClass::SafetyCheck,
            "`dst` must be properly aligned",
            loc,
        );

        // Compute the number of bytes to be copied
        let count = fargs.remove(0);
        let pointee_type = pointee_type_stable(farg_types[0]).unwrap();
        let (count_bytes, overflow_check) =
            self.count_in_bytes(count, pointee_type, Type::size_t(), intrinsic, loc);

        // Build the call to the copy built-in (`memmove` or `memcpy`)
        let copy_builtin = if is_non_overlapping { BuiltinFn::Memcpy } else { BuiltinFn::Memmove };
        let copy_call = copy_builtin.call(vec![dst.clone(), src, count_bytes.clone()], loc);

        // The C implementations of `memmove` and `memcpy` do not allow an
        // invalid pointer for `src` nor `dst`, but the LLVM implementations
        // specify that a zero-length copy is a no-op:
        // https://llvm.org/docs/LangRef.html#llvm-memmove-intrinsic
        // https://llvm.org/docs/LangRef.html#llvm-memcpy-intrinsic
        // This comes up specifically when handling the empty string; CBMC will
        // fail on passing a reference to it unless we codegen this zero check.
        let copy_if_nontrivial = count_bytes.is_zero().ternary(dst, copy_call);
        let copy_expr = if let Some(p) = p {
            self.codegen_expr_to_place_stable(p, copy_if_nontrivial, loc)
        } else {
            copy_if_nontrivial.as_stmt(loc)
        };
        Stmt::block(vec![src_align_check, dst_align_check, overflow_check, copy_expr], loc)
    }

    /// This is an intrinsic that was added in
    /// <https://github.com/rust-lang/rust/pull/114382> that is essentially the
    /// same as memcmp: it compares two slices up to the specified length.
    /// The implementation is the same as the hook for `memcmp`.
    pub fn codegen_compare_bytes(
        &mut self,
        mut fargs: Vec<Expr>,
        place: &Place,
        loc: Location,
    ) -> Stmt {
        let lhs = fargs.remove(0).cast_to(Type::void_pointer());
        let rhs = fargs.remove(0).cast_to(Type::void_pointer());
        let len = fargs.remove(0);
        let (len_var, len_decl) = self.decl_temp_variable(len.typ().clone(), Some(len), loc);
        let (lhs_var, lhs_decl) = self.decl_temp_variable(lhs.typ().clone(), Some(lhs), loc);
        let (rhs_var, rhs_decl) = self.decl_temp_variable(rhs.typ().clone(), Some(rhs), loc);
        let is_len_zero = len_var.clone().is_zero();
        // We have to ensure that the pointers are valid even if we're comparing zero bytes.
        // According to Rust's current definition (see https://github.com/model-checking/kani/issues/1489),
        // this means they have to be non-null and aligned.
        // But alignment is automatically satisfied because `memcmp` takes `*const u8` pointers.
        let is_lhs_ok = lhs_var.clone().is_nonnull();
        let is_rhs_ok = rhs_var.clone().is_nonnull();
        let should_skip_pointer_checks = is_len_zero.and(is_lhs_ok).and(is_rhs_ok);
        let place_expr = unwrap_or_return_codegen_unimplemented_stmt!(
            self,
            self.codegen_place_stable(place, loc)
        )
        .goto_expr;
        let res = should_skip_pointer_checks.ternary(
            Expr::int_constant(0, place_expr.typ().clone()), // zero bytes are always equal (as long as pointers are nonnull and aligned)
            BuiltinFn::Memcmp
                .call(vec![lhs_var, rhs_var, len_var], loc)
                .cast_to(place_expr.typ().clone()),
        );
        let code = place_expr.assign(res, loc).with_location(loc);
        Stmt::block(vec![len_decl, lhs_decl, rhs_decl, code], loc)
    }

    // In some contexts (e.g., compilation-time evaluation),
    // `ptr_guaranteed_cmp` compares two pointers and returns:
    //  * 2 if the result is unknown.
    //  * 1 if they are guaranteed to be equal.
    //  * 0 if they are guaranteed to be not equal.
    // But at runtime, this intrinsic behaves as a regular pointer comparison.
    // Therefore, we return 1 if the pointers are equal and 0 otherwise.
    //
    // This intrinsic replaces `ptr_guaranteed_eq` and `ptr_guaranteed_ne`:
    // https://doc.rust-lang.org/beta/std/primitive.pointer.html#method.guaranteed_eq
    fn codegen_ptr_guaranteed_cmp(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let a = fargs.remove(0);
        let b = fargs.remove(0);
        let place_type = self.place_ty_stable(p);
        let res_type = self.codegen_ty_stable(place_type);
        let eq_expr = a.eq(b);
        let cmp_expr = eq_expr.ternary(res_type.one(), res_type.zero());
        self.codegen_expr_to_place_stable(p, cmp_expr, loc)
    }

    /// Computes the offset from a pointer.
    ///
    /// This function handles code generation for the `arith_offset` intrinsic.
    ///     <https://doc.rust-lang.org/std/intrinsics/fn.arith_offset.html>
    /// According to the documenation, the operation is always safe.
    fn codegen_arith_offset(&mut self, mut fargs: Vec<Expr>, p: &Place, loc: Location) -> Stmt {
        let src_ptr = fargs.remove(0);
        let offset = fargs.remove(0);

        // Compute `dst_ptr` with standard addition to avoid conversion
        let dst_ptr = src_ptr.plus(offset);
        self.codegen_expr_to_place_stable(p, dst_ptr, loc)
    }

    /// ptr_offset_from returns the offset between two pointers
    /// <https://doc.rust-lang.org/std/intrinsics/fn.ptr_offset_from.html>
    fn codegen_ptr_offset_from(&mut self, fargs: Vec<Expr>, p: &Place, loc: Location) -> Stmt {
        let (offset_expr, offset_overflow) = self.codegen_ptr_offset_from_expr(fargs);

        // Check that computing `offset` in bytes would not overflow an `isize`
        // These checks may allow a wrapping-around behavior in CBMC:
        // https://github.com/model-checking/kani/issues/1150
        let overflow_check = self.codegen_assert_assume(
            offset_overflow.overflowed.not(),
            PropertyClass::ArithmeticOverflow,
            "attempt to compute offset in bytes which would overflow an `isize`",
            loc,
        );

        let offset_expr = self.codegen_expr_to_place_stable(p, offset_expr, loc);
        Stmt::block(vec![overflow_check, offset_expr], loc)
    }

    /// `ptr_offset_from_unsigned` returns the offset between two pointers where the order is known.
    /// The logic is similar to `ptr_offset_from` but the return value is a `usize`.
    /// See <https://github.com/rust-lang/rust/issues/95892> for more details
    fn codegen_ptr_offset_from_unsigned(
        &mut self,
        fargs: Vec<Expr>,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let (offset_expr, offset_overflow) = self.codegen_ptr_offset_from_expr(fargs);

        // Check that computing `offset` in bytes would not overflow an `isize`
        // These checks may allow a wrapping-around behavior in CBMC:
        // https://github.com/model-checking/kani/issues/1150
        let overflow_check = self.codegen_assert_assume(
            offset_overflow.overflowed.not(),
            PropertyClass::ArithmeticOverflow,
            "attempt to compute offset in bytes which would overflow an `isize`",
            loc,
        );

        let non_negative_check = self.codegen_assert_assume(
            offset_overflow.result.is_non_negative(),
            PropertyClass::SafetyCheck,
            "attempt to compute unsigned offset with negative distance",
            loc,
        );

        let offset_expr =
            self.codegen_expr_to_place_stable(p, offset_expr.cast_to(Type::size_t()), loc);
        Stmt::block(vec![overflow_check, non_negative_check, offset_expr], loc)
    }

    /// Both `ptr_offset_from` and `ptr_offset_from_unsigned` return the offset between two pointers.
    /// This function implements the common logic between them.
    fn codegen_ptr_offset_from_expr(
        &mut self,
        mut fargs: Vec<Expr>,
    ) -> (Expr, ArithmeticOverflowResult) {
        let dst_ptr = fargs.remove(0);
        let src_ptr = fargs.remove(0);

        // Compute the offset with standard substraction using `isize`
        let cast_dst_ptr = dst_ptr.clone().cast_to(Type::ssize_t());
        let cast_src_ptr = src_ptr.clone().cast_to(Type::ssize_t());
        let offset_overflow = cast_dst_ptr.sub_overflow(cast_src_ptr);

        // Re-compute the offset with standard substraction (no casts this time)
        let ptr_offset_expr = dst_ptr.sub(src_ptr);
        (ptr_offset_expr, offset_overflow)
    }

    /// A transmute is a bitcast from the argument type to the return type.
    /// <https://doc.rust-lang.org/std/intrinsics/fn.transmute.html>
    ///
    /// let bitpattern = unsafe {
    ///     std::mem::transmute::<f32, u32>(1.0)
    /// };
    /// assert!(bitpattern == 0x3F800000);
    ///
    /// Note that this cannot be handled using a simple cast: (uint32_t)(1.0) == 1, not 0x3F800000.
    /// We handle this using the coerce_to(t) operation, which translates to `*(t*)&`.
    /// The other options to handle this type corecion would be using type punning in a union, or a memcpy.
    /// The generated code is the moral equivalent of the following C:
    ///
    /// void main(void)
    /// {
    ///     unsigned int bitpattern;
    ///     float temp_0=1.0f;
    ///     bitpattern = *((unsigned int *)&temp_0);
    ///     assert(bitpattern == 0x3F800000);
    /// }
    ///
    /// Note(std): An earlier attempt to add alignment checks for both the argument and result types
    /// had catastrophic results in the regression. Hence, we don't perform any additional checks
    /// and only encode the transmute operation here.
    fn codegen_intrinsic_transmute(
        &mut self,
        mut fargs: Vec<Expr>,
        ret_ty: Ty,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        assert!(fargs.len() == 1, "transmute had unexpected arguments {fargs:?}");
        let arg = fargs.remove(0);
        let cbmc_ret_ty = self.codegen_ty_stable(ret_ty);
        let expr = arg.transmute_to(cbmc_ret_ty, &self.symbol_table);
        self.codegen_expr_to_place_stable(p, expr, loc)
    }

    // `raw_eq` determines whether the raw bytes of two values are equal.
    // https://doc.rust-lang.org/core/intrinsics/fn.raw_eq.html
    //
    // The implementation below calls `memcmp` and returns equal if the result is zero, and
    // immediately returns zero when ZSTs are compared to mimic what compare_bytes and our memcmp
    // hook do.
    //
    // TODO: It's UB to call `raw_eq` if any of the bytes in the first or second
    // arguments are uninitialized. At present, we cannot detect if there is
    // uninitialized memory, but `raw_eq` would basically return a nondet. value
    // when one of the arguments is uninitialized.
    // https://github.com/model-checking/kani/issues/920
    fn codegen_intrinsic_raw_eq(
        &mut self,
        instance: Instance,
        mut fargs: Vec<Expr>,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let args = instance_args(&instance);
        let ty = *args.0[0].expect_ty();
        let dst = fargs.remove(0).cast_to(Type::void_pointer());
        let val = fargs.remove(0).cast_to(Type::void_pointer());
        let layout = self.layout_of_stable(ty);
        if layout.size.bytes() == 0 {
            self.codegen_expr_to_place_stable(p, Expr::int_constant(1, Type::c_bool()), loc)
        } else {
            let sz = Expr::int_constant(layout.size.bytes(), Type::size_t())
                .with_size_of_annotation(self.codegen_ty_stable(ty));
            let e = BuiltinFn::Memcmp
                .call(vec![dst, val, sz], loc)
                .eq(Type::c_int().zero())
                .cast_to(Type::c_bool());
            self.codegen_expr_to_place_stable(p, e, loc)
        }
    }

    // This is an operation that is primarily relevant for stacked borrow
    // checks.  For Kani, we simply return the pointer.
    fn codegen_retag_box_to_raw(&mut self, mut fargs: Vec<Expr>, p: &Place, loc: Location) -> Stmt {
        assert_eq!(fargs.len(), 1, "raw_box_to_box expected one argument");
        let arg = fargs.remove(0);
        self.codegen_expr_to_place_stable(p, arg, loc)
    }

    fn vtable_info(
        &mut self,
        info: VTableInfo,
        mut fargs: Vec<Expr>,
        place: &Place,
        loc: Location,
    ) -> Stmt {
        assert_eq!(fargs.len(), 1, "vtable intrinsics expects one raw pointer argument");
        let vtable_obj = fargs
            .pop()
            .unwrap()
            .cast_to(self.codegen_ty_common_vtable().to_pointer())
            .dereference();
        let expr = match info {
            VTableInfo::Size => vtable_obj.member(typ::VTABLE_SIZE_FIELD, &self.symbol_table),
            VTableInfo::Align => vtable_obj.member(typ::VTABLE_ALIGN_FIELD, &self.symbol_table),
        };
        self.codegen_expr_to_place_stable(place, expr, loc)
    }

    /// Gets the length for a `simd_shuffle*` instance, which comes in two
    /// forms:
    ///  1. `simd_shuffleN`, where `N` is a number which is part of the name
    ///     (e.g., `simd_shuffle4`).
    ///  2. `simd_shuffle`, where `N` isn't specified and must be computed from
    ///     the length of the indexes SIMD vector (the third argument).
    fn simd_shuffle_length(&mut self, stripped: &str, farg_types: &[Ty], span: Span) -> u64 {
        let n = if stripped.is_empty() {
            // Make sure that this is an SIMD vector, since only the
            // length-suffixed version of `simd_shuffle` (e.g.,
            // `simd_shuffle4`) is type-checked
            if farg_types[2].kind().is_simd()
                && matches!(
                    self.simd_size_and_type(farg_types[2]).1.kind(),
                    TyKind::RigidTy(RigidTy::Uint(UintTy::U32))
                )
            {
                self.simd_size_and_type(farg_types[2]).0
            } else {
                let err_msg = format!(
                    "simd_shuffle index must be a SIMD vector of `u32`, got `{}`",
                    self.pretty_ty(farg_types[2])
                );
                utils::span_err(self.tcx, span, err_msg);
                // Return a dummy value
                u64::MIN
            }
        } else {
            stripped.parse().unwrap_or_else(|_| {
                utils::span_err(
                    self.tcx,
                    span,
                    "bad `simd_shuffle` instruction only caught in codegen?".to_string(),
                );
                // Return a dummy value
                u64::MIN
            })
        };
        self.tcx.dcx().abort_if_errors();
        n
    }

    /// This function computes the size and alignment of a dynamically-sized type.
    /// The implementations follows closely the SSA implementation found in
    /// `rustc_codegen_ssa::glue::size_and_align_of_dst`.
    pub fn size_and_align_of_dst(&mut self, ty: Ty, arg: Expr) -> SizeAlign {
        let layout = self.layout_of_stable(ty);
        let usizet = Type::size_t();
        if !layout.is_unsized() {
            let size = Expr::int_constant(layout.size.bytes_usize(), Type::size_t())
                .with_size_of_annotation(self.codegen_ty_stable(ty));
            let align = Expr::int_constant(layout.align.abi.bytes(), usizet);
            return SizeAlign { size, align };
        }
        match ty.kind() {
            TyKind::RigidTy(RigidTy::Dynamic(..)) => {
                // For traits, we need to retrieve the size and alignment from the vtable.
                let vtable = arg.member("vtable", &self.symbol_table).dereference();
                SizeAlign {
                    size: vtable.clone().member("size", &self.symbol_table),
                    align: vtable.member("align", &self.symbol_table),
                }
            }
            TyKind::RigidTy(RigidTy::Slice(_)) | TyKind::RigidTy(RigidTy::Str) => {
                let unit_t = match ty.kind() {
                    TyKind::RigidTy(RigidTy::Slice(et)) => et,
                    TyKind::RigidTy(RigidTy::Str) => Ty::unsigned_ty(UintTy::U8),
                    _ => unreachable!(),
                };
                let unit = self.layout_of_stable(unit_t);
                // The info in this case is the length of the str, so the size is that
                // times the unit size.
                let size = Expr::int_constant(unit.size.bytes_usize(), Type::size_t())
                    .with_size_of_annotation(self.codegen_ty_stable(unit_t))
                    .mul(arg.member("len", &self.symbol_table));
                let align = Expr::int_constant(layout.align.abi.bytes(), usizet);
                SizeAlign { size, align }
            }
            _ => {
                // This arm handles the case where the dynamically-sized type is nested within the type.
                // The first arm handled the case of the dynamically-sized type itself (a trait object).
                // This case assumes that layout correctly describes the layout of the type instance.
                // In particular, if this is an object of an enum type, the layout describes the
                // layout of the current variant.  The layout includes the offset from
                // the start of the object to the start of each field of the object.
                // The only size left in question is the size of the final field.

                // FIXME: Modify the macro calling this function to ensure that it is only called
                // with a dynamically-sized type (and not, for example, a pointer type of known size).

                assert!(!ty.kind().is_simd());

                // The offset of the nth field gives the size of the first n-1 fields.
                // FIXME: We assume they are aligned according to the machine-preferred alignment given by layout abi.
                let n = layout.fields.count() - 1;
                let sized_size =
                    Expr::int_constant(layout.fields.offset(n).bytes(), Type::size_t())
                        .with_size_of_annotation(self.codegen_ty_stable(ty));
                let sized_align = Expr::int_constant(layout.align.abi.bytes(), Type::size_t());

                // Call this function recursively to compute the size and align for the last field.
                let field_ty = rustc_internal::stable(layout.field(self, n).ty);
                let SizeAlign { size: unsized_size, align: mut unsized_align } =
                    self.size_and_align_of_dst(field_ty, arg);

                // The size of the object is the sum of the sized and unsized portions.
                // FIXME: We should add padding between the sized and unsized portions,
                // but see the comment in ssa codegen saying this is not currently done
                // until issues #26403 and #27023 are resolved.
                let size = sized_size.plus(unsized_size);

                // Packed types ignore the alignment of their fields.
                if let TyKind::RigidTy(RigidTy::Adt(def, _)) = ty.kind() {
                    if rustc_internal::internal(self.tcx, def).repr().packed() {
                        unsized_align = sized_align.clone();
                    }
                }

                // The alignment should be the maximum of the alignments for the
                // sized and unsized portions.
                let align = sized_align
                    .clone()
                    .ge(unsized_align.clone())
                    .ternary(sized_align, unsized_align);

                // Pad the size of the type to make it a multiple of align.
                // We follow the SSA implementation using bit arithmetic: (size + (align-1)) & -align
                // This assumes that align is a power of two, and that all values have the same size_t.

                let one = Expr::int_constant::<isize>(1, Type::size_t());
                let addend = align.clone().sub(one);
                let add = size.plus(addend);
                let neg = align.clone().neg();
                let size = add.bitand(neg);

                SizeAlign { size, align }
            }
        }
    }

    /// `simd_extract(vector, n)` returns the `n`-th element of `vector`
    ///
    /// We check that both the vector's base type and the return type are the
    /// same. In the case of some SIMD intrinsics, the backend is responsible
    /// for performing this and similar checks, and erroring out if it proceeds.
    fn codegen_intrinsic_simd_extract(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place,
        rust_arg_types: &[Ty],
        rust_ret_type: Ty,
        span: Span,
    ) -> Stmt {
        assert!(fargs.len() == 2, "`simd_extract` had unexpected arguments {fargs:?}");
        let vec = fargs.remove(0);
        let index = fargs.remove(0);

        let (_, vector_base_type) = self.simd_size_and_type(rust_arg_types[0]);
        if rust_ret_type != vector_base_type {
            let err_msg = format!(
                "expected return type `{}` (element of input `{}`), found `{}`",
                self.pretty_ty(vector_base_type),
                self.pretty_ty(rust_arg_types[0]),
                self.pretty_ty(rust_ret_type)
            );
            utils::span_err(self.tcx, span, err_msg);
        }
        self.tcx.dcx().abort_if_errors();

        let loc = self.codegen_span_stable(span);
        self.codegen_expr_to_place_stable(p, vec.index_array(index), loc)
    }

    /// Insert is a generic update of a single value in a SIMD vector.
    /// `P = simd_insert(vector, index, newval)` is here translated to
    /// `{ T v = vector; v[index] = (cast)newval; P = v; }`
    ///
    /// CBMC does not currently seem to implement intrinsics like insert e.g.:
    /// `**** WARNING: no body for function __builtin_ia32_vec_set_v4si`
    ///
    /// We check that both the vector's base type and the new value's type are
    /// the same. In the case of some SIMD intrinsics, the backend is
    /// responsible for performing this and similar checks, and erroring out if
    /// it proceeds.
    fn codegen_intrinsic_simd_insert(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place,
        cbmc_ret_ty: Type,
        rust_arg_types: &[Ty],
        span: Span,
        loc: Location,
    ) -> Stmt {
        assert!(fargs.len() == 3, "`simd_insert` had unexpected arguments {fargs:?}");
        let vec = fargs.remove(0);
        let index = fargs.remove(0);
        let newval = fargs.remove(0);

        let (_, vector_base_type) = self.simd_size_and_type(rust_arg_types[0]);
        if vector_base_type != rust_arg_types[2] {
            let err_msg = format!(
                "expected inserted type `{}` (element of input `{}`), found `{}`",
                self.pretty_ty(vector_base_type),
                self.pretty_ty(rust_arg_types[0]),
                self.pretty_ty(rust_arg_types[2]),
            );
            utils::span_err(self.tcx, span, err_msg);
        }
        self.tcx.dcx().abort_if_errors();

        // Type checker should have ensured it's a vector type
        let elem_ty = cbmc_ret_ty.base_type().unwrap().clone();
        let (tmp, decl) = self.decl_temp_variable(cbmc_ret_ty, Some(vec), loc);
        Stmt::block(
            vec![
                decl,
                tmp.clone().index_array(index).assign(newval.cast_to(elem_ty), loc),
                self.codegen_expr_to_place_stable(p, tmp, loc),
            ],
            loc,
        )
    }

    /// Generates code for a SIMD vector comparison intrinsic.
    ///
    /// We perform some typechecks here for two reasons:
    ///  * In the case of SIMD intrinsics, these checks depend on the backend.
    ///  * We can emit a friendly error here, but not in `cprover_bindings`.
    ///
    /// We check the following:
    ///  1. The return type must be the same length as the input types. The
    ///     argument types have already been checked to ensure they have the same
    ///     length (an error would've been emitted otherwise), so we can compare
    ///     the return type against any of the argument types.
    ///
    ///     An example that triggers this error:
    ///     ```rust
    ///     let x = u64x2(0, 0);
    ///     let y = u64x2(0, 1);
    ///     unsafe { let invalid_simd: u32x4 = simd_eq(x, y); }
    ///     ```
    ///     We compare two `u64x2` vectors but try to store the result in a `u32x4`.
    ///  2. The return type must have an integer base type.
    ///
    ///     An example that triggers this error:
    ///     ```rust
    ///     let x = u64x2(0, 0);
    ///     let y = u64x2(0, 1);
    ///     unsafe { let invalid_simd: f32x2 = simd_eq(x, y); }
    ///     ```
    ///     We compare two `u64x2` vectors but try to store the result in a `f32x4`,
    ///     which is composed of `f32` values.
    fn codegen_simd_cmp<F: FnOnce(Expr, Expr, Type) -> Expr>(
        &mut self,
        f: F,
        mut fargs: Vec<Expr>,
        p: &Place,
        span: Span,
        rust_arg_types: &[Ty],
        rust_ret_type: Ty,
    ) -> Stmt {
        let arg1 = fargs.remove(0);
        let arg2 = fargs.remove(0);
        let ret_typ = self.codegen_ty_stable(rust_ret_type);

        if arg1.typ().len().unwrap() != ret_typ.len().unwrap() {
            let err_msg = format!(
                "expected return type with length {} (same as input type `{}`), \
                found `{}` with length {}",
                arg1.typ().len().unwrap(),
                self.pretty_ty(rust_arg_types[0]),
                self.pretty_ty(rust_ret_type),
                ret_typ.len().unwrap()
            );
            utils::span_err(self.tcx, span, err_msg);
        }

        if !ret_typ.base_type().unwrap().is_integer() {
            let (_, rust_base_type) = self.simd_size_and_type(rust_ret_type);
            let err_msg = format!(
                "expected return type with integer elements, found `{}` with non-integer `{}`",
                self.pretty_ty(rust_ret_type),
                self.pretty_ty(rust_base_type),
            );
            utils::span_err(self.tcx, span, err_msg);
        }
        self.tcx.dcx().abort_if_errors();

        // Create the vector comparison expression
        let e = f(arg1, arg2, ret_typ);
        let loc = self.codegen_span_stable(span);
        self.codegen_expr_to_place_stable(p, e, loc)
    }

    /// Codegen for `simd_div` and `simd_rem` intrinsics.
    /// This checks for overflow in signed integer division (i.e. when dividing the minimum integer
    /// for the type by -1). Overflow checks on floating point division are handled by CBMC, as is
    /// division by zero for both integers and floats.
    fn codegen_simd_div_with_overflow(
        &mut self,
        fargs: Vec<Expr>,
        intrinsic: &str,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let op_fun = match intrinsic {
            "simd_div" => Expr::div,
            "simd_rem" => Expr::rem,
            _ => unreachable!("expected simd_div or simd_rem"),
        };
        let base_type = fargs[0].typ().base_type().unwrap().clone();
        if base_type.is_integer() && base_type.is_signed(self.symbol_table.machine_model()) {
            let min_int_expr = base_type.min_int_expr(self.symbol_table.machine_model());
            let negative_one = Expr::int_constant(-1, base_type);
            self.codegen_simd_op_with_overflow(
                op_fun,
                |a, b| a.eq(min_int_expr.clone()).and(b.eq(negative_one.clone())),
                fargs,
                intrinsic,
                p,
                loc,
            )
        } else {
            self.binop(p, fargs, op_fun, loc)
        }
    }

    /// Intrinsics which encode a SIMD arithmetic operation with overflow check.
    /// We expand the overflow check because CBMC overflow operations don't accept array as
    /// argument.
    fn codegen_simd_op_with_overflow<F: FnOnce(Expr, Expr) -> Expr, G: Fn(Expr, Expr) -> Expr>(
        &mut self,
        op_fun: F,
        overflow_fun: G,
        mut fargs: Vec<Expr>,
        intrinsic: &str,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let a = fargs.remove(0);
        let b = fargs.remove(0);

        let a_size = a.typ().len().unwrap();
        let b_size = b.typ().len().unwrap();
        assert_eq!(a_size, b_size, "expected same length vectors");

        let mut check = Expr::bool_false();
        for i in 0..a_size {
            // create expression
            let index = Expr::int_constant(i, Type::ssize_t());
            let v_a = a.clone().index_array(index.clone());
            let v_b = b.clone().index_array(index);
            check = check.or(overflow_fun(v_a, v_b));
        }
        let check_stmt = self.codegen_assert_assume(
            check.not(),
            PropertyClass::ArithmeticOverflow,
            format!("attempt to compute {intrinsic} which would overflow").as_str(),
            loc,
        );
        let res = op_fun(a, b);
        let expr_place = self.codegen_expr_to_place_stable(p, res, loc);
        Stmt::block(vec![check_stmt, expr_place], loc)
    }

    /// Intrinsics which encode a SIMD bitshift.
    /// Also checks for valid shift distance. Shifts on an integer of type T are UB if shift
    /// distance < 0 or >= T::BITS.
    fn codegen_simd_shift_with_distance_check(
        &mut self,
        mut fargs: Vec<Expr>,
        intrinsic: &str,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let values = fargs.remove(0);
        let distances = fargs.remove(0);

        let values_len = values.typ().len().unwrap();
        let distances_len = distances.typ().len().unwrap();
        assert_eq!(values_len, distances_len, "expected same length vectors");

        let value_type = values.typ().base_type().unwrap();
        let distance_type = distances.typ().base_type().unwrap();
        let value_width = value_type.sizeof_in_bits(&self.symbol_table);
        let value_width_expr = Expr::int_constant(value_width, distance_type.clone());
        let distance_is_signed = distance_type.is_signed(self.symbol_table.machine_model());

        let mut excessive_check = Expr::bool_false();
        let mut negative_check = Expr::bool_false();
        for i in 0..distances_len {
            let index = Expr::int_constant(i, Type::ssize_t());
            let distance = distances.clone().index_array(index);
            let excessive_distance_cond = distance.clone().ge(value_width_expr.clone());
            excessive_check = excessive_check.or(excessive_distance_cond);
            if distance_is_signed {
                let negative_distance_cond = distance.is_negative();
                negative_check = negative_check.or(negative_distance_cond);
            }
        }
        let excessive_check_stmt = self.codegen_assert_assume(
            excessive_check.not(),
            PropertyClass::ArithmeticOverflow,
            format!("attempt {intrinsic} with excessive shift distance").as_str(),
            loc,
        );

        let op_fun = match intrinsic {
            "simd_shl" => Expr::shl,
            "simd_shr" => {
                if distance_is_signed {
                    Expr::ashr
                } else {
                    Expr::lshr
                }
            }
            _ => unreachable!("expected a simd shift intrinsic"),
        };
        let res = op_fun(values, distances);
        let expr_place = self.codegen_expr_to_place_stable(p, res, loc);

        if distance_is_signed {
            let negative_check_stmt = self.codegen_assert_assume(
                negative_check.not(),
                PropertyClass::ArithmeticOverflow,
                format!("attempt {intrinsic} with negative shift distance").as_str(),
                loc,
            );
            Stmt::block(vec![excessive_check_stmt, negative_check_stmt, expr_place], loc)
        } else {
            Stmt::block(vec![excessive_check_stmt, expr_place], loc)
        }
    }

    /// `simd_shuffle` constructs a new vector from the elements of two input
    /// vectors, choosing values according to an input array of indexes.
    ///
    /// We check that:
    ///  1. The return type length is equal to the expected length (`n`) of the
    ///     `simd_shuffle` operation.
    ///  2. The return type's subtype is equal to the vector's subtype (i.e.,
    ///     the 1st argument). Both input vectors are guaranteed to be of the
    ///     same type when they get here due to the `simd_shuffle` definition.
    ///
    /// In the case of some SIMD intrinsics, the backend is responsible for
    /// performing this and similar checks, and erroring out if it proceeds.
    ///
    /// TODO: Check that `indexes` contains constant values which are within the
    /// expected bounds. See
    /// <https://github.com/model-checking/kani/issues/1960> for more details.
    ///
    /// This code mimics CBMC's `shuffle_vector_exprt::lower()` here:
    /// <https://github.com/diffblue/cbmc/blob/develop/src/ansi-c/c_expr.cpp>
    ///
    /// We can't use shuffle_vector_exprt because it's not understood by the CBMC backend,
    /// it's immediately lowered by the C frontend.
    /// Issue: <https://github.com/diffblue/cbmc/issues/6297>
    fn codegen_intrinsic_simd_shuffle(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place,
        rust_arg_types: &[Ty],
        rust_ret_type: Ty,
        n: u64,
        span: Span,
    ) -> Stmt {
        // vector, size n: translated as vector types which cbmc treats as arrays
        let vec1 = fargs.remove(0);
        let vec2 = fargs.remove(0);
        // [u32; n]: translated wrapped in a struct
        let indexes = fargs.remove(0);

        let (in_type_len, vec_subtype) = self.simd_size_and_type(rust_arg_types[0]);
        let (ret_type_len, ret_type_subtype) = self.simd_size_and_type(rust_ret_type);
        if ret_type_len != n {
            let err_msg = format!(
                "expected return type of length {n}, found `{}` with length {ret_type_len}",
                self.pretty_ty(rust_ret_type),
            );
            utils::span_err(self.tcx, span, err_msg);
        }
        if vec_subtype != ret_type_subtype {
            let err_msg = format!(
                "expected return element type `{}` (element of input `{}`), \
                 found `{}` with element type `{}`",
                self.pretty_ty(vec_subtype),
                self.pretty_ty(rust_arg_types[0]),
                self.pretty_ty(rust_ret_type),
                self.pretty_ty(ret_type_subtype),
            );
            utils::span_err(self.tcx, span, err_msg);
        }

        // An unsigned type here causes an invariant violation in CBMC.
        // Issue: https://github.com/diffblue/cbmc/issues/6298
        let st_rep = Type::ssize_t();
        let n_rep = Expr::int_constant(in_type_len, st_rep.clone());

        // P = indexes.expanded_map(v -> if v < N then vec1[v] else vec2[v-N])
        let elems = (0..n)
            .map(|i| {
                let idx = Expr::int_constant(i, st_rep.clone());
                // Must not use `indexes.index(i)` directly, because codegen wraps arrays in struct
                let v = self.codegen_idx_array(indexes.clone(), idx).cast_to(st_rep.clone());
                let cond = v.clone().lt(n_rep.clone());
                let t = vec1.clone().index(v.clone());
                let e = vec2.clone().index(v.sub(n_rep.clone()));
                cond.ternary(t, e)
            })
            .collect();
        self.tcx.dcx().abort_if_errors();
        let cbmc_ret_ty = self.codegen_ty_stable(rust_ret_type);
        let loc = self.codegen_span_stable(span);
        self.codegen_expr_to_place_stable(p, Expr::vector_expr(cbmc_ret_ty, elems), loc)
    }

    /// A volatile load of a memory location:
    /// <https://doc.rust-lang.org/std/ptr/fn.read_volatile.html>
    ///
    /// Undefined behavior if any of these conditions are violated:
    ///  * `src` must be valid for writes (done by `--pointer-check`)
    ///  * `src` must be properly aligned (done by `align_check` below)
    ///
    /// TODO: Add a check for the condition:
    ///  * `src` must point to a properly initialized value of type `T`
    ///    See <https://github.com/model-checking/kani/issues/920> for more details
    fn codegen_volatile_load(
        &mut self,
        mut fargs: Vec<Expr>,
        farg_types: &[Ty],
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let src = fargs.remove(0);
        let src_typ = farg_types[0];
        let align = self.is_ptr_aligned(src_typ, src.clone());
        let align_check = self.codegen_assert_assume(
            align,
            PropertyClass::SafetyCheck,
            "`src` must be properly aligned",
            loc,
        );
        let expr = src.dereference();
        let res_stmt = self.codegen_expr_to_place_stable(p, expr, loc);
        Stmt::block(vec![align_check, res_stmt], loc)
    }

    /// A volatile write of a memory location:
    /// <https://doc.rust-lang.org/std/ptr/fn.write_volatile.html>
    ///
    /// Undefined behavior if any of these conditions are violated:
    ///  * `dst` must be valid for writes (done by `--pointer-check`)
    ///  * `dst` must be properly aligned (done by `align_check` below)
    fn codegen_volatile_store(
        &mut self,
        mut fargs: Vec<Expr>,
        farg_types: &[Ty],
        loc: Location,
    ) -> Stmt {
        let dst = fargs.remove(0);
        let src = fargs.remove(0);
        let dst_typ = farg_types[0];
        let align = self.is_ptr_aligned(dst_typ, dst.clone());
        let align_check = self.codegen_assert_assume(
            align,
            PropertyClass::SafetyCheck,
            "`dst` must be properly aligned",
            loc,
        );
        if self.is_zst_stable(pointee_type_stable(dst_typ).unwrap()) {
            // do not attempt to dereference (and assign) a ZST
            align_check
        } else {
            let expr = dst.dereference().assign(src, loc);
            Stmt::block(vec![align_check, expr], loc)
        }
    }

    /// Sets `count * size_of::<T>()` bytes of memory starting at `dst` to `val`
    /// <https://doc.rust-lang.org/std/ptr/fn.write_bytes.html>
    ///
    /// Undefined behavior if any of these conditions are violated:
    ///  * `dst` must be valid for writes (done by memset writable check)
    ///  * `dst` must be properly aligned (done by `align_check` below)
    ///
    /// In addition, we check that computing `bytes` (i.e., the third argument
    /// for the `memset` call) would not overflow
    fn codegen_write_bytes(
        &mut self,
        mut fargs: Vec<Expr>,
        farg_types: &[Ty],
        loc: Location,
    ) -> Stmt {
        let dst = fargs.remove(0).cast_to(Type::void_pointer());
        let val = fargs.remove(0).cast_to(Type::c_int());
        let count = fargs.remove(0);

        // Check that `dst` must be properly aligned
        let dst_typ = farg_types[0];
        let align = self.is_ptr_aligned(dst_typ, dst.clone());
        let align_check = self.codegen_assert_assume(
            align,
            PropertyClass::SafetyCheck,
            "`dst` must be properly aligned",
            loc,
        );

        // Check that computing `count` in bytes would not overflow
        let (count_bytes, overflow_check) = self.count_in_bytes(
            count,
            pointee_type_stable(dst_typ).unwrap(),
            Type::size_t(),
            "write_bytes",
            loc,
        );

        let memset_call = BuiltinFn::Memset.call(vec![dst, val, count_bytes], loc);
        Stmt::block(vec![align_check, overflow_check, memset_call.as_stmt(loc)], loc)
    }

    /// Computes (multiplies) the equivalent of a memory-related number (e.g., an offset) in bytes.
    /// Because this operation may result in an arithmetic overflow, it includes an overflow check.
    /// Returns a tuple with:
    ///  * The result expression of the computation.
    ///  * An assertion statement to ensure the operation has not overflowed.
    pub fn count_in_bytes(
        &mut self,
        count: Expr,
        ty: Ty,
        res_ty: Type,
        intrinsic: &str,
        loc: Location,
    ) -> (Expr, Stmt) {
        assert!(res_ty.is_integer());
        let layout = self.layout_of_stable(ty);
        let size_of_elem = Expr::int_constant(layout.size.bytes(), res_ty)
            .with_size_of_annotation(self.codegen_ty_stable(ty));
        let size_of_count_elems = count.mul_overflow(size_of_elem);
        let message =
            format!("{intrinsic}: attempt to compute number in bytes which would overflow");
        let assert_stmt = self.codegen_assert_assume(
            size_of_count_elems.overflowed.not(),
            PropertyClass::ArithmeticOverflow,
            message.as_str(),
            loc,
        );
        (size_of_count_elems.result, assert_stmt)
    }

    /// Generates an expression `(ptr as usize) % align_of(T) == 0`
    /// to determine if a pointer `ptr` with pointee type `T` is aligned.
    fn is_ptr_aligned(&mut self, ty: Ty, ptr: Expr) -> Expr {
        // Ensure `typ` is a pointer, then extract the pointee type
        assert!(ty.kind().is_raw_ptr());
        let pointee_type = pointee_type_stable(ty).unwrap();
        // Obtain the alignment for the pointee type `T`
        let layout = self.layout_of_stable(pointee_type);
        let align = Expr::int_constant(layout.align.abi.bytes(), Type::size_t());
        // Cast the pointer to `usize` and return the alignment expression
        let cast_ptr = ptr.cast_to(Type::size_t());
        let zero = Type::size_t().zero();
        cast_ptr.rem(align).eq(zero)
    }

    /// Swaps the memory contents pointed to by arguments `x` and `y`, respectively, which is
    /// required for the `typed_swap` intrinsic.
    ///
    /// The standard library API requires that `x` and `y` are readable and writable as their
    /// (common) type (which auto-generated checks for dereferencing will take care of), and the
    /// memory regions pointed to must be non-overlapping.
    pub fn codegen_swap(&mut self, mut fargs: Vec<Expr>, farg_types: &[Ty], loc: Location) -> Stmt {
        // two parameters, and both must be raw pointers with the same base type
        assert!(fargs.len() == 2);
        assert!(farg_types[0].kind().is_raw_ptr());
        assert!(farg_types[0] == farg_types[1]);

        let x = fargs.remove(0);
        let y = fargs.remove(0);

        if self.is_zst_stable(pointee_type_stable(farg_types[0]).unwrap()) {
            // do not attempt to dereference (and assign) a ZST
            Stmt::skip(loc)
        } else {
            // if(same_object(x, y)) {
            //   assert(x + 1 <= y || y + 1 <= x);
            //   assume(x + 1 <= y || y + 1 <= x);
            // }
            let one = Expr::int_constant(1, Type::c_int());
            let non_overlapping = x
                .clone()
                .plus(one.clone())
                .le(y.clone())
                .or(y.clone().plus(one.clone()).le(x.clone()));
            let non_overlapping_check = self.codegen_assert_assume(
                non_overlapping,
                PropertyClass::SafetyCheck,
                "memory regions pointed to by `x` and `y` must not overlap",
                loc,
            );
            let non_overlapping_stmt = Stmt::if_then_else(
                x.clone().same_object(y.clone()),
                non_overlapping_check,
                None,
                loc,
            );

            // T t = *y; *y = *x; *x = t;
            let deref_y = y.clone().dereference();
            let (temp_var, assign_to_t) =
                self.decl_temp_variable(deref_y.typ().clone(), Some(deref_y), loc);
            let assign_to_y = y.dereference().assign(x.clone().dereference(), loc);
            let assign_to_x = x.dereference().assign(temp_var, loc);

            Stmt::block(vec![non_overlapping_stmt, assign_to_t, assign_to_y, assign_to_x], loc)
        }
    }

    /// Checks that the floating-point value is:
    ///     1. Finite (i.e. neither infinite nor NaN)
    ///     2. Its truncated value is in range of the target integer
    /// then performs the cast to the target type
    pub fn codegen_float_to_int_unchecked(
        &mut self,
        intrinsic: &str,
        expr: Expr,
        ty: Ty,
        place: &Place,
        res_ty: Ty,
        loc: Location,
    ) -> Stmt {
        let finite_check = self.codegen_assert_assume(
            expr.clone().is_finite(),
            PropertyClass::ArithmeticOverflow,
            format!("{intrinsic}: attempt to convert a non-finite value to an integer").as_str(),
            loc,
        );

        assert!(res_ty.kind().is_integral());
        assert!(ty.kind().is_float());
        let TyKind::RigidTy(integral_ty) = res_ty.kind() else {
            panic!(
                "Expected intrinsic `{}` type to be `RigidTy`, but found: `{:?}`",
                intrinsic, res_ty
            );
        };
        let TyKind::RigidTy(RigidTy::Float(float_type)) = ty.kind() else {
            panic!("Expected intrinsic `{}` type to be `Float`, but found: `{:?}`", intrinsic, ty);
        };
        let mm = self.symbol_table.machine_model();
        let (lower, upper) = match integral_ty {
            RigidTy::Uint(uint_ty) => get_bounds_uint_expr(float_type, uint_ty, mm),
            RigidTy::Int(int_ty) => get_bounds_int_expr(float_type, int_ty, mm),
            _ => unreachable!(),
        };

        let range_check = self.codegen_assert_assume(
            expr.clone().gt(lower).and(expr.clone().lt(upper)),
            PropertyClass::ArithmeticOverflow,
            format!("{intrinsic}: attempt to convert a value out of range of the target integer")
                .as_str(),
            loc,
        );

        let int_type = self.codegen_ty_stable(res_ty);
        let cast = expr.cast_to(int_type);

        Stmt::block(
            vec![finite_check, range_check, self.codegen_expr_to_place_stable(place, cast, loc)],
            loc,
        )
    }
}

fn instance_args(instance: &Instance) -> GenericArgs {
    let TyKind::RigidTy(RigidTy::FnDef(_, args)) = instance.ty().kind() else {
        unreachable!(
            "Expected intrinsic `{}` type to be `FnDef`, but found: `{:?}`",
            instance.trimmed_name(),
            instance.ty()
        )
    };
    args
}

fn get_bounds_uint_expr(float_ty: FloatTy, uint_ty: UintTy, mm: &MachineModel) -> (Expr, Expr) {
    match float_ty {
        FloatTy::F32 => {
            let (lower, upper) = get_bounds_f32_uint(uint_ty, mm);
            (Expr::float_constant(lower), Expr::float_constant(upper))
        }
        FloatTy::F64 => {
            let (lower, upper) = get_bounds_f64_uint(uint_ty, mm);
            (Expr::double_constant(lower), Expr::double_constant(upper))
        }
        _ => unimplemented!(),
    }
}

fn get_bounds_int_expr(float_ty: FloatTy, int_ty: IntTy, mm: &MachineModel) -> (Expr, Expr) {
    match float_ty {
        FloatTy::F32 => {
            let (lower, upper) = get_bounds_f32_int(int_ty, mm);
            (Expr::float_constant(lower), Expr::float_constant(upper))
        }
        FloatTy::F64 => {
            let (lower, upper) = get_bounds_f64_int(int_ty, mm);
            (Expr::double_constant(lower), Expr::double_constant(upper))
        }
        _ => unimplemented!(),
    }
}

const F32_U_LOWER: [u8; 4] = [0x00, 0x00, 0x80, 0xBF]; // -1.0
const F32_U8_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x43]; // 256.0
const F32_U16_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x47]; // 65536.0
const F32_U32_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x4F]; // 4294967296.0
const F32_U64_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x5F]; // 18446744073709551616.0
// The largest f32 value fits in a u128, so there is no upper bound
const F32_U128_UPPER: [u8; 4] = [0x00, 0x00, 0x80, 0x7F]; // inf

const F32_I8_LOWER: [u8; 4] = [0x00, 0x00, 0x01, 0xC3]; // -129.0
const F32_I16_LOWER: [u8; 4] = [0x00, 0x01, 0x00, 0xC7]; // -32769.0
const F32_I32_LOWER: [u8; 4] = [0x01, 0x00, 0x00, 0xCF]; // -2147483904.0
const F32_I64_LOWER: [u8; 4] = [0x01, 0x00, 0x00, 0xDF]; // -9223373136366403584.0
const F32_I128_LOWER: [u8; 4] = [0x01, 0x00, 0x00, 0xFF]; // -170141203742878835383357727663135391744.0
const F32_I8_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x43]; // 128.0
const F32_I16_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x47]; // 32768.0
const F32_I32_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x4F]; // 2147483648.0
const F32_I64_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x5F]; // 9223372036854775808.0
const F32_I128_UPPER: [u8; 4] = [0x00, 0x00, 0x00, 0x7F]; // 170141183460469231731687303715884105728.0

const F64_U_LOWER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0xBF]; // -1.0
const F64_U8_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x40]; // 256.0
const F64_U16_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x40]; // 65536.0
const F64_U32_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x41]; // 4294967296.0
const F64_U64_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x43]; // 18446744073709551616.0
const F64_U128_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x47]; // 340282366920938463463374607431768211456.0

const F64_I8_LOWER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x60, 0xC0]; // -129.0
const F64_I16_LOWER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0xE0, 0xC0]; // -32769.0
const F64_I32_LOWER: [u8; 8] = [0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xE0, 0xC1]; // -2147483649.0
const F64_I64_LOWER: [u8; 8] = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0xC3]; // -9223372036854777856.0
const F64_I128_LOWER: [u8; 8] = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0xC7]; // -170141183460469269510619166673045815296.0
const F64_I8_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x40]; // 128.0
const F64_I16_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x40]; // 32768.0
const F64_I32_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x41]; // 2147483648.0
const F64_I64_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x43]; // 9223372036854775808.0
const F64_I128_UPPER: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x47]; // 170141183460469231731687303715884105728.0

/// upper is the smallest `f32` that after truncation is strictly larger than u<N>::MAX
/// lower is the largest `f32` that after truncation is strictly smaller than u<N>::MIN
///
/// For example, for N = 8, upper is 256.0 because the previous f32 (i.e.
/// `256_f32.next_down()` which is 255.9999847412109375) when truncated is 255.0,
/// which is not strictly larger than `u8::MAX`
///
/// For all bit-widths, lower is -1.0 because the next higher number, when
/// truncated is -0.0 (or 0.0) which is not strictly smaller than `u<N>::MIN`
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

/// upper is the smallest `f32` that after truncation is strictly larger than i<N>::MAX
/// lower is the largest `f32` that after truncation is strictly smaller than i<N>::MIN
///
/// For example, for N = 16, upper is 32768.0 because the previous f32 (i.e.
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
        //   1. the value when truncated is strictly smaller (larger) than {i, u}<N>::MIN ({i, u}<N>::MAX)
        //   2. the next higher (lower) value when truncated is greater (smaller) than or equal to {i, u}<N>::MIN ({i, u}<N>::MAX)

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
}
