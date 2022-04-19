// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module handles intrinsics
use super::PropertyClass;
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Type};
use rustc_middle::mir::Place;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::Instance;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use tracing::{debug, warn};

macro_rules! emit_concurrency_warning {
    ($intrinsic: expr, $loc: expr) => {{
        warn!(
            "Kani does not support concurrency for now. `{}` in {} treated as a sequential operation.",
            $intrinsic,
            $loc.short_string()
        );
    }};
}

struct SizeAlign {
    size: Expr,
    align: Expr,
}

impl<'tcx> GotocCtx<'tcx> {
    fn binop<F: FnOnce(Expr, Expr) -> Expr>(
        &mut self,
        p: &Place<'tcx>,
        mut fargs: Vec<Expr>,
        f: F,
    ) -> Stmt {
        let arg1 = fargs.remove(0);
        let arg2 = fargs.remove(0);
        let e = f(arg1, arg2);
        self.codegen_expr_to_place(p, e)
    }

    /// Handles codegen for non returning intrinsics
    /// Non returning intrinsics are not associated with a destination
    pub fn codegen_never_return_intrinsic(
        &mut self,
        instance: Instance<'tcx>,
        span: Option<Span>,
    ) -> Stmt {
        let intrinsic = self.symbol_name(instance);
        let intrinsic = intrinsic.as_str();

        debug!("codegen_never_return_intrinsic:\n\tinstance {:?}\n\tspan {:?}", instance, span);

        match intrinsic {
            "abort" => self.codegen_fatal_error(
                PropertyClass::DefaultAssertion,
                "reached intrinsic::abort",
                span,
            ),
            // Transmuting to an uninhabited type is UB.
            "transmute" => self.codegen_fatal_error(
                PropertyClass::DefaultAssertion,
                "transmuting to uninhabited type has undefined behavior",
                span,
            ),
            _ => self.codegen_fatal_error(
                PropertyClass::UnsupportedConstruct,
                &format!("Unsupported intrinsic {}", intrinsic),
                span,
            ),
        }
    }

    /// c.f. rustc_codegen_llvm::intrinsic impl IntrinsicCallMethods<'tcx> for Builder<'a, 'll, 'tcx>
    /// fn codegen_intrinsic_call
    /// c.f. https://doc.rust-lang.org/std/intrinsics/index.html
    pub fn codegen_intrinsic(
        &mut self,
        instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        p: &Place<'tcx>,
        span: Option<Span>,
    ) -> Stmt {
        let intrinsic = self.symbol_name(instance);
        let intrinsic = intrinsic.as_str();
        let loc = self.codegen_span_option(span);
        debug!(
            "codegen_intrinsic:\n\tinstance {:?}\n\tfargs {:?}\n\tp {:?}\n\tspan {:?}",
            instance, fargs, p, span
        );
        let sig = instance.ty(self.tcx, ty::ParamEnv::reveal_all()).fn_sig(self.tcx);
        let sig = self.tcx.normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), sig);
        let ret_ty = self.monomorphize(sig.output());
        let cbmc_ret_ty = self.codegen_ty(ret_ty);

        /// https://doc.rust-lang.org/core/intrinsics/fn.copy.html
        /// https://doc.rust-lang.org/core/intrinsics/fn.copy_nonoverlapping.html
        /// An intrinsic that translates directly into either memmove (for copy) or memcpy (copy_nonoverlapping)
        macro_rules! codegen_intrinsic_copy {
            ($f:ident) => {{
                let src = fargs.remove(0).cast_to(Type::void_pointer());
                let dst = fargs.remove(0).cast_to(Type::void_pointer());
                let count = fargs.remove(0);
                let sz = {
                    match self.fn_sig_of_instance(instance).unwrap().skip_binder().inputs()[0]
                        .kind()
                    {
                        ty::RawPtr(t) => {
                            let layout = self.layout_of(t.ty);
                            Expr::int_constant(layout.size.bytes(), Type::size_t())
                        }
                        _ => unreachable!(),
                    }
                };
                let n = sz.mul(count);
                let call_memcopy = BuiltinFn::$f.call(vec![dst.clone(), src, n.clone()], loc);

                // The C implementation of memcpy does not allow an invalid pointer for
                // the src/dst, but the LLVM implementation specifies that a copy with
                // length zero is a no-op. This comes up specifically when handling
                // the empty string; CBMC will fail on passing a reference to empty
                // string unless we codegen this zero check.
                // https://llvm.org/docs/LangRef.html#llvm-memcpy-intrinsic
                let copy_if_nontrivial = n.is_zero().ternary(dst, call_memcopy);
                self.codegen_expr_to_place(p, copy_if_nontrivial)
            }};
        }

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
                    Expr::cast_arguments_to_machine_equivalent_function_parameter_types(
                        &BuiltinFn::$f.as_expr(),
                        fargs,
                        mm,
                    );
                let e = BuiltinFn::$f.call(casted_fargs, loc);
                self.codegen_expr_to_place(p, e)
            }};
        }

        // Intrinsics of the form *_with_overflow
        macro_rules! codegen_op_with_overflow {
            ($f:ident) => {{
                let pt = self.place_ty(p);
                let t = self.codegen_ty(pt);
                let a = fargs.remove(0);
                let b = fargs.remove(0);
                let res = a.$f(b);
                let e = Expr::struct_expr_from_values(
                    t,
                    vec![res.result, res.overflowed.cast_to(Type::c_bool())],
                    &self.symbol_table,
                );
                self.codegen_expr_to_place(p, e)
            }};
        }

        // Intrinsics which encode a simple arithmetic operation with overflow check
        macro_rules! codegen_op_with_overflow_check {
            ($f:ident) => {{
                let a = fargs.remove(0);
                let b = fargs.remove(0);
                let res = a.$f(b);
                let check = self.codegen_assert(
                    res.overflowed.not(),
                    PropertyClass::ArithmeticOverflow,
                    format!("attempt to compute {} which would overflow", intrinsic).as_str(),
                    loc,
                );
                let expr_place = self.codegen_expr_to_place(p, res.result);
                Stmt::block(vec![expr_place, check], loc)
            }};
        }

        // Intrinsics which encode a SIMD arithmetic operation with overflow check.
        // We expand the overflow check because CBMC overflow operations don't accept array as
        // argument.
        macro_rules! codegen_simd_with_overflow_check {
            ($op:ident, $overflow:ident) => {{
                let a = fargs.remove(0);
                let b = fargs.remove(0);
                let mut check = Expr::bool_false();
                if let Type::Vector { size, .. } = a.typ() {
                    let a_size = size;
                    if let Type::Vector { size, .. } = b.typ() {
                        let b_size = size;
                        assert_eq!(a_size, b_size, "Expected same length vectors",);
                        for i in 0..*a_size {
                            // create expression
                            let index = Expr::int_constant(i, Type::ssize_t());
                            let v_a = a.clone().index_array(index.clone());
                            let v_b = b.clone().index_array(index);
                            check = check.or(v_a.$overflow(v_b));
                        }
                    }
                }
                let check_stmt = self.codegen_assert(
                    check.not(),
                    PropertyClass::ArithmeticOverflow,
                    format!("attempt to compute {} which would overflow", intrinsic).as_str(),
                    loc,
                );
                let res = a.$op(b);
                let expr_place = self.codegen_expr_to_place(p, res);
                Stmt::block(vec![expr_place, check_stmt], loc)
            }};
        }

        // Intrinsics which encode a simple wrapping arithmetic operation
        macro_rules! codegen_wrapping_op {
            ($f:ident) => {{ codegen_intrinsic_binop!($f) }};
        }

        // Intrinsics which encode a simple binary operation
        macro_rules! codegen_intrinsic_boolean_binop {
            ($f:ident) => {{ self.binop(p, fargs, |a, b| a.$f(b).cast_to(Type::c_bool())) }};
        }

        // Intrinsics which encode a simple binary operation
        macro_rules! codegen_intrinsic_binop {
            ($f:ident) => {{ self.binop(p, fargs, |a, b| a.$f(b)) }};
        }

        // Intrinsics which encode a simple binary operation which need a machine model
        macro_rules! codegen_intrinsic_binop_with_mm {
            ($f:ident) => {{
                let arg1 = fargs.remove(0);
                let arg2 = fargs.remove(0);
                let e = arg1.$f(arg2, self.symbol_table.machine_model());
                self.codegen_expr_to_place(p, e)
            }};
        }

        // Intrinsics which encode count intrinsics (ctlz, cttz)
        // The `allow_zero` flag determines if calling these builtins with 0 causes UB
        macro_rules! codegen_count_intrinsic {
            ($builtin: ident, $allow_zero: expr) => {{
                let arg = fargs.remove(0);
                self.codegen_expr_to_place(p, arg.$builtin($allow_zero))
            }};
        }

        // Intrinsics which encode a value known during compilation (e.g., `size_of`)
        macro_rules! codegen_intrinsic_const {
            () => {{
                let value = self
                    .tcx
                    .const_eval_instance(ty::ParamEnv::reveal_all(), instance, span)
                    .unwrap();
                // We assume that the intrinsic has type checked at this point, so
                // we can use the place type as the expression type.
                let e = self.codegen_const_value(value, self.place_ty(p), span.as_ref());
                self.codegen_expr_to_place(p, e)
            }};
        }

        macro_rules! codegen_size_align {
            ($which: ident) => {{
                let tp_ty = instance.substs.type_at(0);
                let arg = fargs.remove(0);
                let size_align = self.size_and_align_of_dst(tp_ty, arg);
                self.codegen_expr_to_place(p, size_align.$which)
            }};
        }

        macro_rules! codegen_unimplemented_intrinsic {
            ($url: expr) => {{
                let e = self.codegen_unimplemented(intrinsic, cbmc_ret_ty, loc, $url);
                self.codegen_expr_to_place(p, e)
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
        // Note: Atomic arithmetic operations wrap around on overflow.
        macro_rules! codegen_atomic_binop {
            ($op: ident) => {{
                let loc = self.codegen_span_option(span);
                emit_concurrency_warning!(intrinsic, loc);
                let var1_ref = fargs.remove(0);
                let var1 = var1_ref.dereference();
                let tmp = self.gen_temp_variable(var1.typ().clone(), loc.clone()).to_expr();
                let decl_stmt = Stmt::decl(tmp.clone(), Some(var1.to_owned()), loc.clone());
                let var2 = fargs.remove(0);
                let op_expr = (var1.clone()).$op(var2).with_location(loc.clone());
                let assign_stmt = (var1.clone()).assign(op_expr, loc.clone());
                let res_stmt = self.codegen_expr_to_place(p, tmp.clone());
                Stmt::atomic_block(vec![decl_stmt, assign_stmt, res_stmt], loc)
            }};
        }

        if let Some(stripped) = intrinsic.strip_prefix("simd_shuffle") {
            let n: u64 = stripped.parse().unwrap();
            return self.codegen_intrinsic_simd_shuffle(fargs, p, cbmc_ret_ty, n);
        }

        match intrinsic {
            "add_with_overflow" => codegen_op_with_overflow!(add_overflow),
            "arith_offset" => codegen_wrapping_op!(plus),
            "assert_inhabited" => self.codegen_assert_intrinsic(instance, intrinsic, span),
            "assert_uninit_valid" => self.codegen_assert_intrinsic(instance, intrinsic, span),
            "assert_zero_valid" => self.codegen_assert_intrinsic(instance, intrinsic, span),
            // https://doc.rust-lang.org/core/intrinsics/fn.assume.html
            // Informs the optimizer that a condition is always true.
            // If the condition is false, the behavior is undefined.
            "assume" => self.codegen_assert(
                fargs.remove(0).cast_to(Type::bool()),
                PropertyClass::Assume,
                "assumption failed",
                loc,
            ),
            "atomic_and" => codegen_atomic_binop!(bitand),
            "atomic_and_acq" => codegen_atomic_binop!(bitand),
            "atomic_and_acqrel" => codegen_atomic_binop!(bitand),
            "atomic_and_rel" => codegen_atomic_binop!(bitand),
            "atomic_and_relaxed" => codegen_atomic_binop!(bitand),
            name if name.starts_with("atomic_cxchg") => {
                self.codegen_atomic_cxchg(intrinsic, fargs, p, loc)
            }
            "atomic_fence" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_fence_acq" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_fence_acqrel" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_fence_rel" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_load" => self.codegen_atomic_load(intrinsic, fargs, p, loc),
            "atomic_load_acq" => self.codegen_atomic_load(intrinsic, fargs, p, loc),
            "atomic_load_relaxed" => self.codegen_atomic_load(intrinsic, fargs, p, loc),
            "atomic_load_unordered" => self.codegen_atomic_load(intrinsic, fargs, p, loc),
            "atomic_nand" => codegen_atomic_binop!(bitnand),
            "atomic_nand_acq" => codegen_atomic_binop!(bitnand),
            "atomic_nand_acqrel" => codegen_atomic_binop!(bitnand),
            "atomic_nand_rel" => codegen_atomic_binop!(bitnand),
            "atomic_nand_relaxed" => codegen_atomic_binop!(bitnand),
            "atomic_or" => codegen_atomic_binop!(bitor),
            "atomic_or_acq" => codegen_atomic_binop!(bitor),
            "atomic_or_acqrel" => codegen_atomic_binop!(bitor),
            "atomic_or_rel" => codegen_atomic_binop!(bitor),
            "atomic_or_relaxed" => codegen_atomic_binop!(bitor),
            "atomic_singlethreadfence" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_singlethreadfence_acq" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_singlethreadfence_acqrel" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_singlethreadfence_rel" => self.codegen_atomic_noop(intrinsic, loc),
            "atomic_store" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_store_rel" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_store_relaxed" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_store_unordered" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_xadd" => codegen_atomic_binop!(plus),
            "atomic_xadd_acq" => codegen_atomic_binop!(plus),
            "atomic_xadd_acqrel" => codegen_atomic_binop!(plus),
            "atomic_xadd_rel" => codegen_atomic_binop!(plus),
            "atomic_xadd_relaxed" => codegen_atomic_binop!(plus),
            "atomic_xchg" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_xchg_acq" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_xchg_acqrel" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_xchg_rel" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_xchg_relaxed" => self.codegen_atomic_store(intrinsic, fargs, p, loc),
            "atomic_xor" => codegen_atomic_binop!(bitxor),
            "atomic_xor_acq" => codegen_atomic_binop!(bitxor),
            "atomic_xor_acqrel" => codegen_atomic_binop!(bitxor),
            "atomic_xor_rel" => codegen_atomic_binop!(bitxor),
            "atomic_xor_relaxed" => codegen_atomic_binop!(bitxor),
            "atomic_xsub" => codegen_atomic_binop!(sub),
            "atomic_xsub_acq" => codegen_atomic_binop!(sub),
            "atomic_xsub_acqrel" => codegen_atomic_binop!(sub),
            "atomic_xsub_rel" => codegen_atomic_binop!(sub),
            "atomic_xsub_relaxed" => codegen_atomic_binop!(sub),
            "bitreverse" => self.codegen_expr_to_place(p, fargs.remove(0).bitreverse()),
            // black_box is an identity function that hints to the compiler
            // to be maximally pessimistic to limit optimizations
            "black_box" => self.codegen_expr_to_place(p, fargs.remove(0)),
            "breakpoint" => Stmt::skip(loc),
            "bswap" => self.codegen_expr_to_place(p, fargs.remove(0).bswap()),
            "caller_location" => {
                codegen_unimplemented_intrinsic!(
                    "https://github.com/model-checking/kani/issues/374"
                )
            }
            "ceilf32" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "ceilf64" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "copy" => codegen_intrinsic_copy!(Memmove),
            "copy_nonoverlapping" => codegen_intrinsic_copy!(Memcpy),
            "copysignf32" => codegen_simple_intrinsic!(Copysignf),
            "copysignf64" => codegen_simple_intrinsic!(Copysign),
            "cosf32" => codegen_simple_intrinsic!(Cosf),
            "cosf64" => codegen_simple_intrinsic!(Cos),
            "ctlz" => codegen_count_intrinsic!(ctlz, true),
            "ctlz_nonzero" => codegen_count_intrinsic!(ctlz, false),
            "ctpop" => self.codegen_expr_to_place(p, fargs.remove(0).popcount()),
            "cttz" => codegen_count_intrinsic!(cttz, true),
            "cttz_nonzero" => codegen_count_intrinsic!(cttz, false),
            "discriminant_value" => {
                let ty = instance.substs.type_at(0);
                let e = self.codegen_get_discriminant(fargs.remove(0).dereference(), ty, ret_ty);
                self.codegen_expr_to_place(p, e)
            }
            "exact_div" => self.codegen_exact_div(fargs, p, loc),
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
            "floorf32" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "floorf64" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "fmaf32" => codegen_simple_intrinsic!(Fmaf),
            "fmaf64" => codegen_simple_intrinsic!(Fma),
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
            "likely" => self.codegen_expr_to_place(p, fargs.remove(0)),
            "log10f32" => codegen_simple_intrinsic!(Log10f),
            "log10f64" => codegen_simple_intrinsic!(Log10),
            "log2f32" => codegen_simple_intrinsic!(Log2f),
            "log2f64" => codegen_simple_intrinsic!(Log2),
            "logf32" => codegen_simple_intrinsic!(Logf),
            "logf64" => codegen_simple_intrinsic!(Log),
            "maxnumf32" => codegen_simple_intrinsic!(Fmaxf),
            "maxnumf64" => codegen_simple_intrinsic!(Fmax),
            "min_align_of" => codegen_intrinsic_const!(),
            "min_align_of_val" => codegen_size_align!(align),
            "minnumf32" => codegen_simple_intrinsic!(Fminf),
            "minnumf64" => codegen_simple_intrinsic!(Fmin),
            "mul_with_overflow" => codegen_op_with_overflow!(mul_overflow),
            "nearbyintf32" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "nearbyintf64" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "needs_drop" => codegen_intrinsic_const!(),
            "offset" => codegen_op_with_overflow_check!(add_overflow),
            "powf32" => codegen_simple_intrinsic!(Powf),
            "powf64" => codegen_simple_intrinsic!(Pow),
            "powif32" => codegen_simple_intrinsic!(Powif),
            "powif64" => codegen_simple_intrinsic!(Powi),
            "pref_align_of" => codegen_intrinsic_const!(),
            "ptr_guaranteed_eq" => codegen_intrinsic_boolean_binop!(eq),
            "ptr_guaranteed_ne" => codegen_intrinsic_boolean_binop!(neq),
            "ptr_offset_from" => self.codegen_ptr_offset_from(fargs, p, loc),
            "raw_eq" => self.codegen_intrinsic_raw_eq(instance, fargs, p, loc),
            "rintf32" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "rintf64" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "rotate_left" => codegen_intrinsic_binop!(rol),
            "rotate_right" => codegen_intrinsic_binop!(ror),
            "roundf32" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "roundf64" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "saturating_add" => codegen_intrinsic_binop_with_mm!(saturating_add),
            "saturating_sub" => codegen_intrinsic_binop_with_mm!(saturating_sub),
            "sinf32" => codegen_simple_intrinsic!(Sinf),
            "sinf64" => codegen_simple_intrinsic!(Sin),
            "simd_add" => codegen_simd_with_overflow_check!(plus, add_overflow_p),
            "simd_and" => codegen_intrinsic_binop!(bitand),
            "simd_div" => codegen_intrinsic_binop!(div),
            "simd_eq" => codegen_intrinsic_binop!(eq),
            "simd_extract" => {
                let vec = fargs.remove(0);
                let index = fargs.remove(0);
                self.codegen_expr_to_place(p, vec.index_array(index))
            }
            "simd_ge" => codegen_intrinsic_binop!(ge),
            "simd_gt" => codegen_intrinsic_binop!(gt),
            "simd_insert" => self.codegen_intrinsic_simd_insert(fargs, p, cbmc_ret_ty, loc),
            "simd_le" => codegen_intrinsic_binop!(le),
            "simd_lt" => codegen_intrinsic_binop!(lt),
            "simd_mul" => codegen_simd_with_overflow_check!(mul, mul_overflow_p),
            "simd_ne" => codegen_intrinsic_binop!(neq),
            "simd_or" => codegen_intrinsic_binop!(bitor),
            "simd_rem" => codegen_intrinsic_binop!(rem),
            "simd_shl" => codegen_intrinsic_binop!(shl),
            "simd_shr" => {
                if fargs[0].typ().base_type().unwrap().is_signed(self.symbol_table.machine_model())
                {
                    codegen_intrinsic_binop!(ashr)
                } else {
                    codegen_intrinsic_binop!(lshr)
                }
            }
            // "simd_shuffle#" => handled in an `if` preceding this match
            "simd_sub" => codegen_simd_with_overflow_check!(sub, sub_overflow_p),
            "simd_xor" => codegen_intrinsic_binop!(bitxor),
            "size_of" => codegen_intrinsic_const!(),
            "size_of_val" => codegen_size_align!(size),
            "sqrtf32" => codegen_simple_intrinsic!(Sqrtf),
            "sqrtf64" => codegen_simple_intrinsic!(Sqrt),
            "sub_with_overflow" => codegen_op_with_overflow!(sub_overflow),
            "transmute" => self.codegen_intrinsic_transmute(fargs, ret_ty, p),
            "truncf32" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "truncf64" => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/1025"
            ),
            "try" => {
                codegen_unimplemented_intrinsic!(
                    "https://github.com/model-checking/kani/issues/267"
                )
            }
            "type_id" => codegen_intrinsic_const!(),
            "type_name" => codegen_intrinsic_const!(),
            "unaligned_volatile_load" => {
                self.codegen_expr_to_place(p, fargs.remove(0).dereference())
            }
            "unchecked_add" => codegen_op_with_overflow_check!(add_overflow),
            "unchecked_div" => codegen_intrinsic_binop!(div),
            "unchecked_mul" => codegen_op_with_overflow_check!(mul_overflow),
            "unchecked_rem" => codegen_intrinsic_binop!(rem),
            "unchecked_shl" => codegen_intrinsic_binop!(shl),
            "unchecked_shr" => {
                if fargs[0].typ().is_signed(self.symbol_table.machine_model()) {
                    codegen_intrinsic_binop!(ashr)
                } else {
                    codegen_intrinsic_binop!(lshr)
                }
            }
            "unchecked_sub" => codegen_op_with_overflow_check!(sub_overflow),
            "unlikely" => self.codegen_expr_to_place(p, fargs.remove(0)),
            "unreachable" => self.codegen_assert(
                Expr::bool_false(),
                PropertyClass::DefaultAssertion,
                "unreachable",
                loc,
            ),
            "volatile_copy_memory" => codegen_intrinsic_copy!(Memmove),
            "volatile_copy_nonoverlapping_memory" => codegen_intrinsic_copy!(Memcpy),
            "volatile_load" => self.codegen_expr_to_place(p, fargs.remove(0).dereference()),
            "volatile_store" => {
                assert!(self.place_ty(p).is_unit());
                self.codegen_volatile_store(instance, intrinsic, fargs, loc)
            }
            "wrapping_add" => codegen_wrapping_op!(plus),
            "wrapping_mul" => codegen_wrapping_op!(mul),
            "wrapping_sub" => codegen_wrapping_op!(sub),
            "write_bytes" => {
                let dst = fargs.remove(0).cast_to(Type::void_pointer());
                let val = fargs.remove(0).cast_to(Type::c_int());
                let count = fargs.remove(0);
                let ty = self.monomorphize(instance.substs.type_at(0));
                let layout = self.layout_of(ty);
                let sz = Expr::int_constant(layout.size.bytes(), Type::size_t());
                let e = BuiltinFn::Memset.call(vec![dst, val, count.mul(sz)], loc);
                self.codegen_expr_to_place(p, e)
            }

            // Unimplemented
            _ => codegen_unimplemented_intrinsic!(
                "https://github.com/model-checking/kani/issues/new/choose"
            ),
        }
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
        span: Option<Span>,
    ) -> Stmt {
        let arg1 = fargs.remove(0);
        let arg2 = fargs.remove(0);
        let msg1 = format!("first argument for {} is finite", intrinsic);
        let msg2 = format!("second argument for {} is finite", intrinsic);
        let loc = self.codegen_span_option(span);
        let finite_check1 = self.codegen_assert(
            arg1.is_finite(),
            PropertyClass::FiniteCheck,
            msg1.as_str(),
            loc.clone(),
        );
        let finite_check2 = self.codegen_assert(
            arg2.is_finite(),
            PropertyClass::FiniteCheck,
            msg2.as_str(),
            loc.clone(),
        );
        Stmt::block(vec![finite_check1, finite_check2, stmt], loc)
    }

    fn codegen_exact_div(&mut self, mut fargs: Vec<Expr>, p: &Place<'tcx>, loc: Location) -> Stmt {
        // Check for undefined behavior conditions defined in
        // https://doc.rust-lang.org/std/intrinsics/fn.exact_div.html
        let mm = self.symbol_table.machine_model();
        let a = fargs.remove(0);
        let b = fargs.remove(0);
        let atyp = a.typ();
        let btyp = b.typ();
        let division_is_exact = a.clone().rem(b.clone()).eq(atyp.zero());
        let divisor_is_nonzero = b.clone().neq(btyp.zero());
        let dividend_is_int_min = if atyp.is_signed(&mm) {
            a.clone().eq(atyp.min_int_expr(mm))
        } else {
            Expr::bool_false()
        };
        let divisor_is_minus_one =
            if btyp.is_signed(mm) { b.clone().eq(btyp.one().neg()) } else { Expr::bool_false() };
        let division_does_not_overflow = dividend_is_int_min.and(divisor_is_minus_one).not();
        Stmt::block(
            vec![
                self.codegen_assert(
                    division_is_exact,
                    PropertyClass::ExactDiv,
                    "exact_div arguments divide exactly",
                    loc,
                ),
                self.codegen_assert(
                    divisor_is_nonzero,
                    PropertyClass::ExactDiv,
                    "exact_div divisor is nonzero",
                    loc,
                ),
                self.codegen_assert(
                    division_does_not_overflow,
                    PropertyClass::ExactDiv,
                    "exact_div division does not overflow",
                    loc,
                ),
                self.codegen_expr_to_place(p, a.div(b)),
            ],
            loc,
        )
    }

    /// Generates either a panic or no-op for `assert_*` intrinsics.
    /// These are intrinsics that statically compile to panics if the type
    /// layout is invalid so we get a message that mentions the offending type.
    ///
    /// https://doc.rust-lang.org/std/intrinsics/fn.assert_inhabited.html
    /// https://doc.rust-lang.org/std/intrinsics/fn.assert_uninit_valid.html
    /// https://doc.rust-lang.org/std/intrinsics/fn.assert_zero_valid.html
    fn codegen_assert_intrinsic(
        &mut self,
        instance: Instance<'tcx>,
        intrinsic: &str,
        span: Option<Span>,
    ) -> Stmt {
        let ty = instance.substs.type_at(0);
        let layout = self.layout_of(ty);
        // Note: We follow the pattern seen in `codegen_panic_intrinsic` from `rustc_codegen_ssa`
        // https://github.com/rust-lang/rust/blob/master/compiler/rustc_codegen_ssa/src/mir/block.rs

        // For all intrinsics we first check `is_uninhabited` to give a more
        // precise error message
        if layout.abi.is_uninhabited() {
            return self.codegen_fatal_error(
                PropertyClass::DefaultAssertion,
                &format!("attempted to instantiate uninhabited type `{}`", ty),
                span,
            );
        }

        // Then we check if the type allows "raw" initialization for the cases
        // where memory is zero-initialized or entirely uninitialized
        if intrinsic == "assert_zero_valid" && !layout.might_permit_raw_init(self, true) {
            return self.codegen_fatal_error(
                PropertyClass::DefaultAssertion,
                &format!("attempted to zero-initialize type `{}`, which is invalid", ty),
                span,
            );
        }

        if intrinsic == "assert_uninit_valid" && !layout.might_permit_raw_init(self, false) {
            return self.codegen_fatal_error(
                PropertyClass::DefaultAssertion,
                &format!("attempted to leave type `{}` uninitialized, which is invalid", ty),
                span,
            );
        }

        // Otherwise we generate a no-op statement
        let loc = self.codegen_span_option(span);
        return Stmt::skip(loc);
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
        p: &Place<'tcx>,
        loc: Location,
    ) -> Stmt {
        emit_concurrency_warning!(intrinsic, loc);
        let var1_ref = fargs.remove(0);
        let var1 = var1_ref.dereference().with_location(loc.clone());
        let res_stmt = self.codegen_expr_to_place(p, var1);
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
        p: &Place<'tcx>,
        loc: Location,
    ) -> Stmt {
        emit_concurrency_warning!(intrinsic, loc);
        let var1_ref = fargs.remove(0);
        let var1 = var1_ref.dereference().with_location(loc.clone());
        let tmp = self.gen_temp_variable(var1.typ().clone(), loc.clone()).to_expr();
        let decl_stmt = Stmt::decl(tmp.clone(), Some(var1.to_owned()), loc.clone());
        let var2 = fargs.remove(0).with_location(loc.clone());
        let var3 = fargs.remove(0).with_location(loc.clone());
        let eq_expr = (var1.clone()).eq(var2.clone());
        let assign_stmt = (var1.clone()).assign(var3, loc.clone());
        let cond_update_stmt = Stmt::if_then_else(eq_expr, assign_stmt, None, loc.clone());
        let place_type = self.place_ty(p);
        let res_type = self.codegen_ty(place_type);
        let tuple_expr =
            Expr::struct_expr_from_values(res_type, vec![tmp, Expr::c_true()], &self.symbol_table)
                .with_location(loc.clone());
        let res_stmt = self.codegen_expr_to_place(p, tuple_expr);
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
        p: &Place<'tcx>,
        loc: Location,
    ) -> Stmt {
        emit_concurrency_warning!(intrinsic, loc);
        let var1_ref = fargs.remove(0);
        let var1 = var1_ref.dereference().with_location(loc.clone());
        let tmp = self.gen_temp_variable(var1.typ().clone(), loc.clone()).to_expr();
        let decl_stmt = Stmt::decl(tmp.clone(), Some(var1.to_owned()), loc.clone());
        let var2 = fargs.remove(0).with_location(loc.clone());
        let assign_stmt = (var1.clone()).assign(var2, loc.clone());
        let res_stmt = self.codegen_expr_to_place(p, tmp.clone());
        Stmt::atomic_block(vec![decl_stmt, assign_stmt, res_stmt], loc)
    }

    /// Atomic no-ops (e.g., atomic_fence) are transformed into SKIP statements
    fn codegen_atomic_noop(&mut self, intrinsic: &str, loc: Location) -> Stmt {
        emit_concurrency_warning!(intrinsic, loc);
        let skip_stmt = Stmt::skip(loc.clone());
        Stmt::atomic_block(vec![skip_stmt], loc)
    }

    /// ptr_offset_from returns the offset between two pointers
    /// https://doc.rust-lang.org/std/intrinsics/fn.ptr_offset_from.html
    fn codegen_ptr_offset_from(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place<'tcx>,
        loc: Location,
    ) -> Stmt {
        let a = fargs.remove(0);
        let b = fargs.remove(0);
        let pointers_to_same_object = a.clone().same_object(b.clone());

        Stmt::block(
            vec![
                self.codegen_assert(
                    pointers_to_same_object,
                    PropertyClass::PointerOffset,
                    "ptr_offset_from: pointers point to same object",
                    loc.clone(),
                ),
                self.codegen_expr_to_place(p, a.sub(b)),
            ],
            loc,
        )
    }

    /// A transmute is a bitcast from the argument type to the return type.
    /// https://doc.rust-lang.org/std/intrinsics/fn.transmute.html
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
    fn codegen_intrinsic_transmute(
        &mut self,
        mut fargs: Vec<Expr>,
        ret_ty: Ty<'tcx>,
        p: &Place<'tcx>,
    ) -> Stmt {
        assert!(fargs.len() == 1, "transmute had unexpected arguments {:?}", fargs);
        let arg = fargs.remove(0);
        let expr = arg.transmute_to(self.codegen_ty(ret_ty), &self.symbol_table);
        self.codegen_expr_to_place(p, expr)
    }

    // `raw_eq` determines whether the raw bytes of two values are equal.
    // https://stdrs.dev/nightly/x86_64-pc-windows-gnu/std/intrinsics/fn.raw_eq.html
    //
    // The implementation below calls `memcmp` and returns equal if the result is zero.
    //
    // TODO: Handle more cases in this intrinsic by looking into the parameters' layouts.
    // TODO: Fix soundness issues in this intrinsic. It's UB to call `raw_eq` if any of
    // the bytes in the first or second arguments are uninitialized.
    fn codegen_intrinsic_raw_eq(
        &mut self,
        instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        p: &Place<'tcx>,
        loc: Location,
    ) -> Stmt {
        let ty = self.monomorphize(instance.substs.type_at(0));
        let dst = fargs.remove(0).cast_to(Type::void_pointer());
        let val = fargs.remove(0).cast_to(Type::void_pointer());
        let layout = self.layout_of(ty);
        let sz = Expr::int_constant(layout.size.bytes(), Type::size_t());
        let e = BuiltinFn::Memcmp
            .call(vec![dst, val, sz], loc)
            .eq(Type::c_int().zero())
            .cast_to(Type::c_bool());
        self.codegen_expr_to_place(p, e)
    }

    /// This function computes the size and alignment of a dynamically-sized type.
    /// The implementations follows closely the SSA implementation found in
    /// rustc_codegen_ssa::glue::size_and_align_of_dst.
    fn size_and_align_of_dst(&self, t: Ty<'tcx>, arg: Expr) -> SizeAlign {
        let layout = self.layout_of(t);
        let usizet = Type::size_t();
        if !layout.is_unsized() {
            let size = Expr::int_constant(layout.size.bytes_usize(), Type::size_t());
            let align = Expr::int_constant(layout.align.abi.bytes(), usizet);
            return SizeAlign { size, align };
        }
        match t.kind() {
            ty::Dynamic(..) => {
                //TODO figure out if this needs to be done, or if the result we want here is for the fat pointer.
                //We need to get the actual value from the vtable like in codegen_ssa/glue.rs
                // let vs = self.layout_of(self.tcx.vtable_methods(binder.principal().unwrap().with_self_ty(self.tcx, t)));
                // https://rust-lang.github.io/unsafe-code-guidelines/layout/pointers.html
                // The size of &dyn Trait is two words.
                let size = Expr::int_constant((layout.size.bytes_usize()) * 2, Type::size_t());
                // The alignment of &dyn Trait is the word size.
                let align = Expr::int_constant(layout.align.abi.bytes(), usizet);
                SizeAlign { size, align }
            }
            ty::Slice(_) | ty::Str => {
                let unit_t = match t.kind() {
                    ty::Slice(et) => et,
                    ty::Str => &self.tcx.types.u8,
                    _ => unreachable!(),
                };
                let unit = self.layout_of(*unit_t);
                // The info in this case is the length of the str, so the size is that
                // times the unit size.
                let size = Expr::int_constant(unit.size.bytes_usize(), Type::size_t())
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

                assert!(!t.is_simd());

                // The offset of the nth field gives the size of the first n-1 fields.
                // FIXME: We assume they are aligned according to the machine-preferred alignment given by layout abi.
                let n = layout.fields.count() - 1;
                let sized_size =
                    Expr::int_constant(layout.fields.offset(n).bytes(), Type::size_t());
                let sized_align = Expr::int_constant(layout.align.abi.bytes(), Type::size_t());

                // Call this function recursively to compute the size and align for the last field.
                let field_ty = layout.field(self, n).ty;
                let SizeAlign { size: unsized_size, align: mut unsized_align } =
                    self.size_and_align_of_dst(field_ty, arg);

                // The size of the object is the sum of the sized and unsized portions.
                // FIXME: We should add padding between the sized and unsized portions,
                // but see the comment in ssa codegen saying this is not currently done
                // until issues #26403 and #27023 are resolved.
                let size = sized_size.plus(unsized_size);

                // Packed types ignore the alignment of their fields.
                if let ty::Adt(def, _) = t.kind() {
                    if def.repr().packed() {
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

    /// Insert is a generic update of a single value in a SIMD vector.
    /// `P = simd_insert(vector, index, newval)` is here translated to
    /// `{ T v = vector; v[index] = (cast)newval; P = v; }`
    ///
    /// CBMC does not currently seem to implement intrinsics like insert e.g.:
    /// `**** WARNING: no body for function __builtin_ia32_vec_set_v4si`
    pub fn codegen_intrinsic_simd_insert(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place<'tcx>,
        cbmc_ret_ty: Type,
        loc: Location,
    ) -> Stmt {
        assert!(fargs.len() == 3, "simd_insert had unexpected arguments {:?}", fargs);
        let vec = fargs.remove(0);
        let index = fargs.remove(0);
        let newval = fargs.remove(0);
        // Type checker should have ensured it's a vector type
        let elem_ty = cbmc_ret_ty.base_type().unwrap().clone();
        let tmp = self.gen_temp_variable(cbmc_ret_ty, loc.clone()).to_expr();
        Stmt::block(
            vec![
                Stmt::decl(tmp.clone(), Some(vec), loc.clone()),
                tmp.clone().index_array(index).assign(newval.cast_to(elem_ty), loc.clone()),
                self.codegen_expr_to_place(p, tmp),
            ],
            loc,
        )
    }

    /// simd_shuffle constructs a new vector from the elements of two input vectors,
    /// choosing values according to an input array of indexes.
    ///
    /// This code mimics CBMC's `shuffle_vector_exprt::lower()` here:
    /// https://github.com/diffblue/cbmc/blob/develop/src/ansi-c/c_expr.cpp
    ///
    /// We can't use shuffle_vector_exprt because it's not understood by the CBMC backend,
    /// it's immediately lowered by the C frontend.
    /// Issue: https://github.com/diffblue/cbmc/issues/6297
    pub fn codegen_intrinsic_simd_shuffle(
        &mut self,
        mut fargs: Vec<Expr>,
        p: &Place<'tcx>,
        cbmc_ret_ty: Type,
        n: u64,
    ) -> Stmt {
        assert!(fargs.len() == 3, "simd_shuffle had unexpected arguments {:?}", fargs);
        // vector, size n: translated as vector types which cbmc treats as arrays
        let vec1 = fargs.remove(0);
        let vec2 = fargs.remove(0);
        // [u32; n]: translated wrapped in a struct
        let indexes = fargs.remove(0);

        // An unsigned type here causes an invariant violation in CBMC.
        // Issue: https://github.com/diffblue/cbmc/issues/6298
        let st_rep = Type::ssize_t();
        let n_rep = Expr::int_constant(n, st_rep.clone());

        // P = indexes.expanded_map(v -> if v < N then vec1[v] else vec2[v-N])
        let elems = (0..n)
            .map(|i| {
                let i = Expr::int_constant(i, st_rep.clone());
                // Must not use `indexes.index(i)` directly, because codegen wraps arrays in struct
                let v = self.codegen_idx_array(indexes.clone(), i).cast_to(st_rep.clone());
                let cond = v.clone().lt(n_rep.clone());
                let t = vec1.clone().index(v.clone());
                let e = vec2.clone().index(v.sub(n_rep.clone()));
                cond.ternary(t, e)
            })
            .collect();
        self.codegen_expr_to_place(p, Expr::vector_expr(cbmc_ret_ty, elems))
    }

    /// A volatile write of a memory location:
    /// https://doc.rust-lang.org/std/ptr/fn.write_volatile.html
    ///
    /// Undefined behavior if any of these conditions are violated:
    ///  * `dst` must be valid for writes (done by `--pointer-check`)
    ///  * `dst` must be properly aligned (done by `align_check` below)
    fn codegen_volatile_store(
        &mut self,
        instance: Instance<'tcx>,
        intrinsic: &str,
        mut fargs: Vec<Expr>,
        loc: Location,
    ) -> Stmt {
        emit_concurrency_warning!(intrinsic, loc);
        let dst = fargs.remove(0);
        let src = fargs.remove(0);
        let typ = instance.substs.type_at(0);
        let align = self.is_aligned(typ, dst.clone());
        let align_check = self.codegen_assert(
            align,
            PropertyClass::DefaultAssertion,
            "`dst` is properly aligned",
            loc.clone(),
        );
        let expr = dst.dereference().assign(src, loc.clone());
        Stmt::block(vec![align_check, expr], loc)
    }
}
