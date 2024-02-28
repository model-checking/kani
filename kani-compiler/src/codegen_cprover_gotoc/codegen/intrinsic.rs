// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module handles intrinsics
use super::typ;
use super::{bb_label, PropertyClass};
use crate::codegen_cprover_gotoc::codegen::ty_stable::pointee_type_stable;
use crate::codegen_cprover_gotoc::{utils, GotocCtx};
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{
    ArithmeticOverflowResult, BinaryOperator, BuiltinFn, Expr, Location, Stmt, Type,
};
use rustc_middle::ty::layout::ValidityRequirement;
use rustc_middle::ty::ParamEnv;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{BasicBlockIdx, Operand, Place};
use stable_mir::ty::{GenericArgs, RigidTy, Span, Ty, TyKind, UintTy};
use tracing::debug;

struct SizeAlign {
    size: Expr,
    align: Expr,
}

enum VTableInfo {
    Size,
    Align,
}

impl<'tcx> GotocCtx<'tcx> {
    fn binop<F: FnOnce(Expr, Expr) -> Expr>(
        &mut self,
        place: &Place,
        mut fargs: Vec<Expr>,
        f: F,
    ) -> Stmt {
        let arg1 = fargs.remove(0);
        let arg2 = fargs.remove(0);
        let expr = f(arg1, arg2);
        self.codegen_expr_to_place_stable(place, expr)
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
        let intrinsic = instance.mangled_name();

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
        let intrinsic_sym = instance.mangled_name();
        let intrinsic = intrinsic_sym.as_str();
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
                self.codegen_expr_to_place_stable(place, expr)
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
                    format!("attempt to compute {} which would overflow", intrinsic).as_str(),
                    loc,
                );
                let res = a.$f(b);
                let expr_place = self.codegen_expr_to_place_stable(place, res);
                Stmt::block(vec![div_overflow_check, expr_place], loc)
            }};
        }

        // Intrinsics which encode a simple wrapping arithmetic operation
        macro_rules! codegen_wrapping_op {
            ($f:ident) => {{ codegen_intrinsic_binop!($f) }};
        }

        // Intrinsics which encode a simple binary operation
        macro_rules! codegen_intrinsic_binop {
            ($f:ident) => {{ self.binop(place, fargs, |a, b| a.$f(b)) }};
        }

        // Intrinsics which encode a simple binary operation which need a machine model
        macro_rules! codegen_intrinsic_binop_with_mm {
            ($f:ident) => {{
                let arg1 = fargs.remove(0);
                let arg2 = fargs.remove(0);
                let expr = arg1.$f(arg2, self.symbol_table.machine_model());
                self.codegen_expr_to_place_stable(place, expr)
            }};
        }

        // Intrinsics which encode count intrinsics (ctlz, cttz)
        // The `allow_zero` flag determines if calling these builtins with 0 causes UB
        macro_rules! codegen_count_intrinsic {
            ($builtin: ident, $allow_zero: expr) => {{
                let arg = fargs.remove(0);
                self.codegen_expr_to_place_stable(place, arg.$builtin($allow_zero))
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
                let expr = self.codegen_allocation(&alloc, place_ty, Some(span));
                self.codegen_expr_to_place_stable(&place, expr)
            }};
        }

        macro_rules! codegen_size_align {
            ($which: ident) => {{
                let args = instance_args(&instance);
                // The type `T` that we'll compute the size or alignment.
                let target_ty = args.0[0].expect_ty();
                let arg = fargs.remove(0);
                let size_align = self.size_and_align_of_dst(*target_ty, arg);
                self.codegen_expr_to_place_stable(place, size_align.$which)
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
                self.store_concurrent_construct(intrinsic, loc);
                let var1_ref = fargs.remove(0);
                let var1 = var1_ref.dereference();
                let (tmp, decl_stmt) =
                    self.decl_temp_variable(var1.typ().clone(), Some(var1.to_owned()), loc);
                let var2 = fargs.remove(0);
                let op_expr: Expr;
                if var2.typ().is_pointer() {
                    op_expr = (var1.clone().cast_to(Type::c_size_t()))
                        .$op(var2.cast_to(Type::c_size_t()))
                        .with_location(loc)
                        .cast_to(var1.typ().clone());
                } else {
                    op_expr = (var1.clone()).$op(var2).with_location(loc);
                }
                let assign_stmt = (var1.clone()).assign(op_expr, loc);
                let res_stmt = self.codegen_expr_to_place_stable(place, tmp.clone());
                Stmt::atomic_block(vec![decl_stmt, assign_stmt, res_stmt], loc)
            }};
        }

        macro_rules! unstable_codegen {
            ($($tt:tt)*) => {{
                let expr = self.codegen_unimplemented_expr(
                    &format!("'{}' intrinsic", intrinsic),
                    cbmc_ret_ty,
                    loc,
                    "https://github.com/model-checking/kani/issues/new/choose",
                );
                self.codegen_expr_to_place_stable(place, expr)
            }};
        }

        if let Some(stripped) = intrinsic.strip_prefix("simd_shuffle") {
            assert!(fargs.len() == 3, "`simd_shuffle` had unexpected arguments {fargs:?}");
            let n: u64 = self.simd_shuffle_length(stripped, farg_types, span);
            return self.codegen_intrinsic_simd_shuffle(fargs, place, farg_types, ret_ty, n, span);
        }

        match intrinsic {
            "add_with_overflow" => {
                self.codegen_op_with_overflow(BinaryOperator::OverflowResultPlus, fargs, place, loc)
            }
            "arith_offset" => self.codegen_offset(intrinsic, instance, fargs, place, loc),
            "assert_inhabited" => self.codegen_assert_intrinsic(instance, intrinsic, span),
            "assert_mem_uninitialized_valid" => {
                self.codegen_assert_intrinsic(instance, intrinsic, span)
            }
            "assert_zero_valid" => self.codegen_assert_intrinsic(instance, intrinsic, span),
            // https://doc.rust-lang.org/core/intrinsics/fn.assume.html
            // Informs the optimizer that a condition is always true.
            // If the condition is false, the behavior is undefined.
            "assume" => self.codegen_assert_assume(
                fargs.remove(0).cast_to(Type::bool()),
                PropertyClass::Assume,
                "assumption failed",
                loc,
            ),
            "atomic_and_seqcst" => codegen_atomic_binop!(bitand),
            "atomic_and_acquire" => codegen_atomic_binop!(bitand),
            "atomic_and_acqrel" => codegen_atomic_binop!(bitand),
            "atomic_and_release" => codegen_atomic_binop!(bitand),
            "atomic_and_relaxed" => codegen_atomic_binop!(bitand),
            name if name.starts_with("atomic_cxchg") => {
                self.codegen_atomic_cxchg(intrinsic, fargs, place, loc)
            }
            "atomic_fence_seqcst" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_fence_acquire" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_fence_acqrel" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_fence_release" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_load_seqcst" => self.codegen_atomic_load(intrinsic, fargs, place, loc),
            "atomic_load_acquire" => self.codegen_atomic_load(intrinsic, fargs, place, loc),
            "atomic_load_relaxed" => self.codegen_atomic_load(intrinsic, fargs, place, loc),
            "atomic_load_unordered" => self.codegen_atomic_load(intrinsic, fargs, place, loc),
            "atomic_max_seqcst" => codegen_atomic_binop!(max),
            "atomic_max_acquire" => codegen_atomic_binop!(max),
            "atomic_max_acqrel" => codegen_atomic_binop!(max),
            "atomic_max_release" => codegen_atomic_binop!(max),
            "atomic_max_relaxed" => codegen_atomic_binop!(max),
            "atomic_min_seqcst" => codegen_atomic_binop!(min),
            "atomic_min_acquire" => codegen_atomic_binop!(min),
            "atomic_min_acqrel" => codegen_atomic_binop!(min),
            "atomic_min_release" => codegen_atomic_binop!(min),
            "atomic_min_relaxed" => codegen_atomic_binop!(min),
            "atomic_nand_seqcst" => codegen_atomic_binop!(bitnand),
            "atomic_nand_acquire" => codegen_atomic_binop!(bitnand),
            "atomic_nand_acqrel" => codegen_atomic_binop!(bitnand),
            "atomic_nand_release" => codegen_atomic_binop!(bitnand),
            "atomic_nand_relaxed" => codegen_atomic_binop!(bitnand),
            "atomic_or_seqcst" => codegen_atomic_binop!(bitor),
            "atomic_or_acquire" => codegen_atomic_binop!(bitor),
            "atomic_or_acqrel" => codegen_atomic_binop!(bitor),
            "atomic_or_release" => codegen_atomic_binop!(bitor),
            "atomic_or_relaxed" => codegen_atomic_binop!(bitor),
            "atomic_singlethreadfence_seqcst" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_singlethreadfence_acquire" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_singlethreadfence_acqrel" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_singlethreadfence_release" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_store_seqcst" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_store_release" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_store_relaxed" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_store_unordered" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_umax_seqcst" => codegen_atomic_binop!(max),
            "atomic_umax_acquire" => codegen_atomic_binop!(max),
            "atomic_umax_acqrel" => codegen_atomic_binop!(max),
            "atomic_umax_release" => codegen_atomic_binop!(max),
            "atomic_umax_relaxed" => codegen_atomic_binop!(max),
            "atomic_umin_seqcst" => codegen_atomic_binop!(min),
            "atomic_umin_acquire" => codegen_atomic_binop!(min),
            "atomic_umin_acqrel" => codegen_atomic_binop!(min),
            "atomic_umin_release" => codegen_atomic_binop!(min),
            "atomic_umin_relaxed" => codegen_atomic_binop!(min),
            "atomic_xadd_seqcst" => codegen_atomic_binop!(plus),
            "atomic_xadd_acquire" => codegen_atomic_binop!(plus),
            "atomic_xadd_acqrel" => codegen_atomic_binop!(plus),
            "atomic_xadd_release" => codegen_atomic_binop!(plus),
            "atomic_xadd_relaxed" => codegen_atomic_binop!(plus),
            "atomic_xchg_seqcst" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_xchg_acquire" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_xchg_acqrel" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_xchg_release" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_xchg_relaxed" => self.codegen_atomic_store(intrinsic, fargs, place, loc),
            "atomic_xor_seqcst" => codegen_atomic_binop!(bitxor),
            "atomic_xor_acquire" => codegen_atomic_binop!(bitxor),
            "atomic_xor_acqrel" => codegen_atomic_binop!(bitxor),
            "atomic_xor_release" => codegen_atomic_binop!(bitxor),
            "atomic_xor_relaxed" => codegen_atomic_binop!(bitxor),
            "atomic_xsub_seqcst" => codegen_atomic_binop!(sub),
            "atomic_xsub_acquire" => codegen_atomic_binop!(sub),
            "atomic_xsub_acqrel" => codegen_atomic_binop!(sub),
            "atomic_xsub_release" => codegen_atomic_binop!(sub),
            "atomic_xsub_relaxed" => codegen_atomic_binop!(sub),
            "bitreverse" => self.codegen_expr_to_place_stable(place, fargs.remove(0).bitreverse()),
            // black_box is an identity function that hints to the compiler
            // to be maximally pessimistic to limit optimizations
            "black_box" => self.codegen_expr_to_place_stable(place, fargs.remove(0)),
            "breakpoint" => Stmt::skip(loc),
            "bswap" => self.codegen_expr_to_place_stable(place, fargs.remove(0).bswap()),
            "caller_location" => self.codegen_unimplemented_stmt(
                intrinsic,
                loc,
                "https://github.com/model-checking/kani/issues/374",
            ),
            "ceilf32" => codegen_simple_intrinsic!(Ceilf),
            "ceilf64" => codegen_simple_intrinsic!(Ceil),
            "compare_bytes" => self.codegen_compare_bytes(fargs, place, loc),
            "copy" => self.codegen_copy(intrinsic, false, fargs, farg_types, Some(place), loc),
            "copy_nonoverlapping" => unreachable!(
                "Expected `core::intrinsics::unreachable` to be handled by `StatementKind::CopyNonOverlapping`"
            ),
            "copysignf32" => codegen_simple_intrinsic!(Copysignf),
            "copysignf64" => codegen_simple_intrinsic!(Copysign),
            "cosf32" => codegen_simple_intrinsic!(Cosf),
            "cosf64" => codegen_simple_intrinsic!(Cos),
            "ctlz" => codegen_count_intrinsic!(ctlz, true),
            "ctlz_nonzero" => codegen_count_intrinsic!(ctlz, false),
            "ctpop" => self.codegen_ctpop(place, span, fargs.remove(0), farg_types[0]),
            "cttz" => codegen_count_intrinsic!(cttz, true),
            "cttz_nonzero" => codegen_count_intrinsic!(cttz, false),
            "discriminant_value" => {
                let sig = instance.ty().kind().fn_sig().unwrap().skip_binder();
                let ty = pointee_type_stable(sig.inputs()[0]).unwrap();
                let e = self.codegen_get_discriminant(fargs.remove(0).dereference(), ty, ret_ty);
                self.codegen_expr_to_place_stable(place, e)
            }
            "exact_div" => self.codegen_exact_div(fargs, place, loc),
            "exp2f32" => codegen_simple_intrinsic!(Exp2f),
            "exp2f64" => codegen_simple_intrinsic!(Exp2),
            "expf32" => codegen_simple_intrinsic!(Expf),
            "expf64" => codegen_simple_intrinsic!(Exp),
            "fabsf32" => codegen_simple_intrinsic!(Fabsf),
            "fabsf64" => codegen_simple_intrinsic!(Fabs),
            "fadd_fast" => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(plus);
                self.add_finite_args_checks(intrinsic, fargs_clone, binop_stmt, span)
            }
            "fdiv_fast" => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(div);
                self.add_finite_args_checks(intrinsic, fargs_clone, binop_stmt, span)
            }
            "floorf32" => codegen_simple_intrinsic!(Floorf),
            "floorf64" => codegen_simple_intrinsic!(Floor),
            "fmaf32" => unstable_codegen!(codegen_simple_intrinsic!(Fmaf)),
            "fmaf64" => unstable_codegen!(codegen_simple_intrinsic!(Fma)),
            "fmul_fast" => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(mul);
                self.add_finite_args_checks(intrinsic, fargs_clone, binop_stmt, span)
            }
            "forget" => Stmt::skip(loc),
            "fsub_fast" => {
                let fargs_clone = fargs.clone();
                let binop_stmt = codegen_intrinsic_binop!(sub);
                self.add_finite_args_checks(intrinsic, fargs_clone, binop_stmt, span)
            }
            "likely" => self.codegen_expr_to_place_stable(place, fargs.remove(0)),
            "log10f32" => unstable_codegen!(codegen_simple_intrinsic!(Log10f)),
            "log10f64" => unstable_codegen!(codegen_simple_intrinsic!(Log10)),
            "log2f32" => unstable_codegen!(codegen_simple_intrinsic!(Log2f)),
            "log2f64" => unstable_codegen!(codegen_simple_intrinsic!(Log2)),
            "logf32" => codegen_simple_intrinsic!(Logf),
            "logf64" => codegen_simple_intrinsic!(Log),
            "maxnumf32" => codegen_simple_intrinsic!(Fmaxf),
            "maxnumf64" => codegen_simple_intrinsic!(Fmax),
            "min_align_of" => codegen_intrinsic_const!(),
            "min_align_of_val" => codegen_size_align!(align),
            "minnumf32" => codegen_simple_intrinsic!(Fminf),
            "minnumf64" => codegen_simple_intrinsic!(Fmin),
            "mul_with_overflow" => {
                self.codegen_op_with_overflow(BinaryOperator::OverflowResultMult, fargs, place, loc)
            }
            "nearbyintf32" => codegen_simple_intrinsic!(Nearbyintf),
            "nearbyintf64" => codegen_simple_intrinsic!(Nearbyint),
            "needs_drop" => codegen_intrinsic_const!(),
            // As of https://github.com/rust-lang/rust/pull/110822 the `offset` intrinsic is lowered to `mir::BinOp::Offset`
            "offset" => unreachable!(
                "Expected `core::intrinsics::unreachable` to be handled by `BinOp::OffSet`"
            ),
            "powf32" => codegen_simple_intrinsic!(Powf),
            "powf64" => codegen_simple_intrinsic!(Pow),
            "powif32" => unstable_codegen!(codegen_simple_intrinsic!(Powif)),
            "powif64" => unstable_codegen!(codegen_simple_intrinsic!(Powi)),
            "pref_align_of" => codegen_intrinsic_const!(),
            "ptr_guaranteed_cmp" => self.codegen_ptr_guaranteed_cmp(fargs, place),
            "ptr_offset_from" => self.codegen_ptr_offset_from(fargs, place, loc),
            "ptr_offset_from_unsigned" => self.codegen_ptr_offset_from_unsigned(fargs, place, loc),
            "raw_eq" => self.codegen_intrinsic_raw_eq(instance, fargs, place, loc),
            "rintf32" => codegen_simple_intrinsic!(Rintf),
            "rintf64" => codegen_simple_intrinsic!(Rint),
            "rotate_left" => codegen_intrinsic_binop!(rol),
            "rotate_right" => codegen_intrinsic_binop!(ror),
            "roundf32" => codegen_simple_intrinsic!(Roundf),
            "roundf64" => codegen_simple_intrinsic!(Round),
            "saturating_add" => codegen_intrinsic_binop_with_mm!(saturating_add),
            "saturating_sub" => codegen_intrinsic_binop_with_mm!(saturating_sub),
            "sinf32" => codegen_simple_intrinsic!(Sinf),
            "sinf64" => codegen_simple_intrinsic!(Sin),
            "simd_add" => self.codegen_simd_op_with_overflow(
                Expr::plus,
                Expr::add_overflow_p,
                fargs,
                intrinsic,
                place,
                loc,
            ),
            "simd_and" => codegen_intrinsic_binop!(bitand),
            // TODO: `simd_rem` doesn't check for overflow cases for floating point operands.
            // <https://github.com/model-checking/kani/pull/2645>
            "simd_div" | "simd_rem" => {
                self.codegen_simd_div_with_overflow(fargs, intrinsic, place, loc)
            }
            "simd_eq" => {
                self.codegen_simd_cmp(Expr::vector_eq, fargs, place, span, farg_types, ret_ty)
            }
            "simd_extract" => {
                self.codegen_intrinsic_simd_extract(fargs, place, farg_types, ret_ty, span)
            }
            "simd_ge" => {
                self.codegen_simd_cmp(Expr::vector_ge, fargs, place, span, farg_types, ret_ty)
            }
            "simd_gt" => {
                self.codegen_simd_cmp(Expr::vector_gt, fargs, place, span, farg_types, ret_ty)
            }
            "simd_insert" => {
                self.codegen_intrinsic_simd_insert(fargs, place, cbmc_ret_ty, farg_types, span, loc)
            }
            "simd_le" => {
                self.codegen_simd_cmp(Expr::vector_le, fargs, place, span, farg_types, ret_ty)
            }
            "simd_lt" => {
                self.codegen_simd_cmp(Expr::vector_lt, fargs, place, span, farg_types, ret_ty)
            }
            "simd_mul" => self.codegen_simd_op_with_overflow(
                Expr::mul,
                Expr::mul_overflow_p,
                fargs,
                intrinsic,
                place,
                loc,
            ),
            "simd_ne" => {
                self.codegen_simd_cmp(Expr::vector_neq, fargs, place, span, farg_types, ret_ty)
            }
            "simd_or" => codegen_intrinsic_binop!(bitor),
            "simd_shl" | "simd_shr" => {
                self.codegen_simd_shift_with_distance_check(fargs, intrinsic, place, loc)
            }
            // "simd_shuffle#" => handled in an `if` preceding this match
            "simd_sub" => self.codegen_simd_op_with_overflow(
                Expr::sub,
                Expr::sub_overflow_p,
                fargs,
                intrinsic,
                place,
                loc,
            ),
            "simd_xor" => codegen_intrinsic_binop!(bitxor),
            "size_of" => unreachable!(),
            "size_of_val" => codegen_size_align!(size),
            "sqrtf32" => unstable_codegen!(codegen_simple_intrinsic!(Sqrtf)),
            "sqrtf64" => unstable_codegen!(codegen_simple_intrinsic!(Sqrt)),
            "sub_with_overflow" => self.codegen_op_with_overflow(
                BinaryOperator::OverflowResultMinus,
                fargs,
                place,
                loc,
            ),
            "transmute" => self.codegen_intrinsic_transmute(fargs, ret_ty, place),
            "truncf32" => codegen_simple_intrinsic!(Truncf),
            "truncf64" => codegen_simple_intrinsic!(Trunc),
            "try" => self.codegen_unimplemented_stmt(
                intrinsic,
                loc,
                "https://github.com/model-checking/kani/issues/267",
            ),
            "type_id" => codegen_intrinsic_const!(),
            "type_name" => codegen_intrinsic_const!(),
            "unaligned_volatile_load" => {
                unstable_codegen!(
                    self.codegen_expr_to_place_stable(place, fargs.remove(0).dereference())
                )
            }
            "unchecked_add" | "unchecked_mul" | "unchecked_shl" | "unchecked_shr"
            | "unchecked_sub" => {
                unreachable!("Expected intrinsic `{intrinsic}` to be lowered before codegen")
            }
            "unchecked_div" => codegen_op_with_div_overflow_check!(div),
            "unchecked_rem" => codegen_op_with_div_overflow_check!(rem),
            "unlikely" => self.codegen_expr_to_place_stable(place, fargs.remove(0)),
            "unreachable" => unreachable!(
                "Expected `std::intrinsics::unreachable` to be handled by `TerminatorKind::Unreachable`"
            ),
            "volatile_copy_memory" => unstable_codegen!(codegen_intrinsic_copy!(Memmove)),
            "volatile_copy_nonoverlapping_memory" => {
                unstable_codegen!(codegen_intrinsic_copy!(Memcpy))
            }
            "volatile_load" => self.codegen_volatile_load(fargs, farg_types, place, loc),
            "volatile_store" => {
                assert!(self.place_ty_stable(place).kind().is_unit());
                self.codegen_volatile_store(fargs, farg_types, loc)
            }
            "vtable_size" => self.vtable_info(VTableInfo::Size, fargs, place, loc),
            "vtable_align" => self.vtable_info(VTableInfo::Align, fargs, place, loc),
            "wrapping_add" => codegen_wrapping_op!(plus),
            "wrapping_mul" => codegen_wrapping_op!(mul),
            "wrapping_sub" => codegen_wrapping_op!(sub),
            "write_bytes" => {
                assert!(self.place_ty_stable(place).kind().is_unit());
                self.codegen_write_bytes(fargs, farg_types, loc)
            }
            // Unimplemented
            _ => self.codegen_unimplemented_stmt(
                intrinsic,
                loc,
                "https://github.com/model-checking/kani/issues/new/choose",
            ),
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
            self.codegen_expr_to_place_stable(&target_place, arg.popcount())
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
            Expr::statement_expression(vec![res.as_stmt(loc)], result_type),
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
                self.codegen_expr_to_place_stable(p, a.div(b)),
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
        if layout.abi.is_uninhabited() {
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
        let res_stmt = self.codegen_expr_to_place_stable(p, var1);
        Stmt::atomic_block(vec![res_stmt], loc)
    }

    /// An atomic compare-and-exchange updates the value referenced in
    /// its primary argument and returns a tuple that contains:
    ///  * the previous value
    ///  * a boolean value indicating whether the operation was successful or not
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
        let res_stmt = self.codegen_expr_to_place_stable(p, tuple_expr);
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
        let res_stmt = self.codegen_expr_to_place_stable(place, tmp);
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
    ///      at `src` with a size of `count * size_of::<T>()` bytes must *not*
    ///      overlap with the region of memory beginning at `dst` with the same
    ///      size.
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
            self.codegen_expr_to_place_stable(p, copy_if_nontrivial)
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
        let place_expr =
            unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place_stable(place))
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
    fn codegen_ptr_guaranteed_cmp(&mut self, mut fargs: Vec<Expr>, p: &Place) -> Stmt {
        let a = fargs.remove(0);
        let b = fargs.remove(0);
        let place_type = self.place_ty_stable(p);
        let res_type = self.codegen_ty_stable(place_type);
        let eq_expr = a.eq(b);
        let cmp_expr = eq_expr.ternary(res_type.one(), res_type.zero());
        self.codegen_expr_to_place_stable(p, cmp_expr)
    }

    /// Computes the offset from a pointer.
    ///
    /// Note that this function handles code generation for:
    ///  1. The `offset` intrinsic.
    ///     <https://doc.rust-lang.org/std/intrinsics/fn.offset.html>
    ///  2. The `arith_offset` intrinsic.
    ///     <https://doc.rust-lang.org/std/intrinsics/fn.arith_offset.html>
    ///
    /// Note(std): We don't check that the starting or resulting pointer stay
    /// within bounds of the object they point to. Doing so causes spurious
    /// failures due to the usage of these intrinsics in the standard library.
    /// See <https://github.com/model-checking/kani/issues/1233> for more details.
    /// Also, note that this isn't a requirement for `arith_offset`, but it's
    /// one of the safety conditions specified for `offset`:
    /// <https://doc.rust-lang.org/std/primitive.pointer.html#safety-2>
    fn codegen_offset(
        &mut self,
        intrinsic: &str,
        instance: Instance,
        mut fargs: Vec<Expr>,
        p: &Place,
        loc: Location,
    ) -> Stmt {
        let src_ptr = fargs.remove(0);
        let offset = fargs.remove(0);

        // Check that computing `offset` in bytes would not overflow
        let args = instance_args(&instance);
        let ty = args.0[0].expect_ty();
        let (offset_bytes, bytes_overflow_check) =
            self.count_in_bytes(offset.clone(), *ty, Type::ssize_t(), intrinsic, loc);

        // Check that the computation would not overflow an `isize`
        // These checks may allow a wrapping-around behavior in CBMC:
        // https://github.com/model-checking/kani/issues/1150
        let dst_ptr_of = src_ptr.clone().cast_to(Type::ssize_t()).add_overflow(offset_bytes);
        let overflow_check = self.codegen_assert_assume(
            dst_ptr_of.overflowed.not(),
            PropertyClass::ArithmeticOverflow,
            "attempt to compute offset which would overflow",
            loc,
        );

        // Re-compute `dst_ptr` with standard addition to avoid conversion
        let dst_ptr = src_ptr.plus(offset);
        let expr_place = self.codegen_expr_to_place_stable(p, dst_ptr);
        Stmt::block(vec![bytes_overflow_check, overflow_check, expr_place], loc)
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

        let offset_expr = self.codegen_expr_to_place_stable(p, offset_expr);
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

        let offset_expr = self.codegen_expr_to_place_stable(p, offset_expr.cast_to(Type::size_t()));
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
    fn codegen_intrinsic_transmute(&mut self, mut fargs: Vec<Expr>, ret_ty: Ty, p: &Place) -> Stmt {
        assert!(fargs.len() == 1, "transmute had unexpected arguments {fargs:?}");
        let arg = fargs.remove(0);
        let cbmc_ret_ty = self.codegen_ty_stable(ret_ty);
        let expr = arg.transmute_to(cbmc_ret_ty, &self.symbol_table);
        self.codegen_expr_to_place_stable(p, expr)
    }

    // `raw_eq` determines whether the raw bytes of two values are equal.
    // https://doc.rust-lang.org/core/intrinsics/fn.raw_eq.html
    //
    // The implementation below calls `memcmp` and returns equal if the result is zero.
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
        let sz = Expr::int_constant(layout.size.bytes(), Type::size_t())
            .with_size_of_annotation(self.codegen_ty_stable(ty));
        let e = BuiltinFn::Memcmp
            .call(vec![dst, val, sz], loc)
            .eq(Type::c_int().zero())
            .cast_to(Type::c_bool());
        self.codegen_expr_to_place_stable(p, e)
    }

    fn vtable_info(
        &mut self,
        info: VTableInfo,
        mut fargs: Vec<Expr>,
        place: &Place,
        _loc: Location,
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
        self.codegen_expr_to_place_stable(place, expr)
    }

    /// Gets the length for a `simd_shuffle*` instance, which comes in two
    /// forms:
    ///  1. `simd_shuffleN`, where `N` is a number which is part of the name
    ///     (e.g., `simd_shuffle4`).
    ///  2. `simd_shuffle`, where `N` isn't specified and must be computed from
    ///     the length of the indexes array (the third argument).
    fn simd_shuffle_length(&mut self, stripped: &str, farg_types: &[Ty], span: Span) -> u64 {
        let n = if stripped.is_empty() {
            // Make sure that this is an array, since only the
            // length-suffixed version of `simd_shuffle` (e.g.,
            // `simd_shuffle4`) is type-checked
            match farg_types[2].kind() {
                TyKind::RigidTy(RigidTy::Array(ty, len))
                    if matches!(ty.kind(), TyKind::RigidTy(RigidTy::Uint(UintTy::U32))) =>
                {
                    len.eval_target_usize().unwrap_or_else(|err| {
                        utils::span_err(
                            self.tcx,
                            span,
                            format!("could not evaluate shuffle index array length: {err}"),
                        );
                        // Return a dummy value
                        u64::MIN
                    })
                }
                _ => {
                    let err_msg = format!(
                        "simd_shuffle index must be an array of `u32`, got `{}`",
                        self.pretty_ty(farg_types[2])
                    );
                    utils::span_err(self.tcx, span, err_msg);
                    // Return a dummy value
                    u64::MIN
                }
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
    fn size_and_align_of_dst(&mut self, ty: Ty, arg: Expr) -> SizeAlign {
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

        self.codegen_expr_to_place_stable(p, vec.index_array(index))
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
                self.codegen_expr_to_place_stable(p, tmp),
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
        self.codegen_expr_to_place_stable(p, e)
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
            self.binop(p, fargs, op_fun)
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
        let expr_place = self.codegen_expr_to_place_stable(p, res);
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
        let expr_place = self.codegen_expr_to_place_stable(p, res);

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
        self.codegen_expr_to_place_stable(p, Expr::vector_expr(cbmc_ret_ty, elems))
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
    /// See <https://github.com/model-checking/kani/issues/920> for more details
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
        let res_stmt = self.codegen_expr_to_place_stable(p, expr);
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
        let expr = dst.dereference().assign(src, loc);
        Stmt::block(vec![align_check, expr], loc)
    }

    /// Sets `count * size_of::<T>()` bytes of memory starting at `dst` to `val`
    /// <https://doc.rust-lang.org/std/ptr/fn.write_bytes.html>
    ///
    /// Undefined behavior if any of these conditions are violated:
    ///  * `dst` must be valid for writes (done by memset writable check)
    ///  * `dst` must be properly aligned (done by `align_check` below)
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
