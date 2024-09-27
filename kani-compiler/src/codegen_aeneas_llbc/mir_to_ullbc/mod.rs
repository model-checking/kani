// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains a context for translating stable MIR into Charon's
//! unstructured low-level borrow calculus (ULLBC)

use core::panic;
use std::path::PathBuf;

use charon_lib::ast::CastKind as CharonCastKind;
use charon_lib::ast::Place as CharonPlace;
use charon_lib::ast::ProjectionElem as CharonProjectionElem;
use charon_lib::ast::Rvalue as CharonRvalue;
use charon_lib::ast::Span as CharonSpan;
use charon_lib::ast::meta::{AttrInfo, Loc, RawSpan};
use charon_lib::ast::types::Ty as CharonTy;
use charon_lib::ast::{AbortKind, Body as CharonBody, Var, VarId, make_locals_generator};
use charon_lib::ast::{
    AnyTransId, Assert, BodyId, BuiltinTy, Disambiguator, FileName, FunDecl, FunSig, GenericArgs,
    GenericParams, IntegerTy, ItemKind, ItemMeta, ItemOpacity, Literal, LiteralTy, Name, Opaque,
    PathElem, RawConstantExpr, RefKind, Region as CharonRegion, ScalarValue, TranslatedCrate,
    TypeId,
};
use charon_lib::ast::{
    BinOp as CharonBinOp, Call, FnOperand, FnPtr, FunDeclId, FunId, FunIdOrTraitMethodRef,
    VariantId,
};
use charon_lib::ast::{
    BorrowKind as CharonBorrowKind, ConstantExpr, Operand as CharonOperand, UnOp,
};
use charon_lib::common::Error;
use charon_lib::errors::ErrorCtx;
use charon_lib::ids::Vector;
use charon_lib::ullbc_ast::{
    BlockData, BlockId, BodyContents, ExprBody, RawStatement, RawTerminator,
    Statement as CharonStatement, SwitchTargets as CharonSwitchTargets,
    Terminator as CharonTerminator,
};
use charon_lib::{error_assert, error_or_panic};
use rustc_errors::MultiSpan;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use rustc_span::def_id::DefId as InternalDefId;
use stable_mir::abi::PassMode;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    BasicBlock, BinOp, Body, BorrowKind, CastKind, ConstOperand, Mutability, Operand, Place,
    ProjectionElem, Rvalue, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind,
};
use stable_mir::ty::{
    Allocation, ConstantKind, IndexedVal, IntTy, MirConst, Region, RegionKind, RigidTy, Span, Ty,
    TyKind, UintTy,
};

use stable_mir::{CrateDef, DefId};
use tracing::{debug, trace};

/// A context for translating a single MIR function to ULLBC.
/// The results of the translation are stored in the `translated` field.
pub struct Context<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    instance: Instance,
    translated: &'a mut TranslatedCrate,
    errors: &'a mut ErrorCtx<'tcx>,
}

impl<'a, 'tcx> Context<'a, 'tcx> {
    /// Create a new context for translating the function `instance`, populating
    /// the results of the translation in `translated`
    pub fn new(
        tcx: TyCtxt<'tcx>,
        instance: Instance,
        translated: &'a mut TranslatedCrate,
        errors: &'a mut ErrorCtx<'tcx>,
    ) -> Self {
        Self { tcx, instance, translated, errors }
    }

    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn span_err<S: Into<MultiSpan>>(&mut self, span: S, msg: &str) {
        self.errors.span_err(span, msg);
    }

    fn continue_on_failure(&self) -> bool {
        self.errors.continue_on_failure()
    }

    /// Perform the translation
    pub fn translate(&mut self) -> Result<(), ()> {
        // Charon's `id_map` is in terms of internal `DefId`
        let def_id = rustc_internal::internal(self.tcx(), self.instance.def.def_id());

        // TODO: might want to populate `errors.dep_sources` to help with
        // debugging

        let fid = self.register_fun_decl_id(def_id);

        let item_meta = match self.translate_item_meta_from_rid(self.instance) {
            Ok(item_meta) => item_meta,
            Err(_) => {
                return Err(());
            }
        };

        let signature = self.translate_function_signature();
        let body = match self.translate_function_body() {
            Ok(body) => body,
            Err(_) => {
                return Err(());
            }
        };

        let fun_decl = FunDecl {
            def_id: fid,
            rust_id: def_id,
            item_meta,
            signature,
            kind: ItemKind::Regular,
            body: Ok(body),
        };

        self.translated.fun_decls.set_slot(fid, fun_decl);

        Ok(())
    }

    /// Get or create a `FunDeclId` for the given function
    fn register_fun_decl_id(&mut self, def_id: InternalDefId) -> FunDeclId {
        let tid = match self.translated.id_map.get(&def_id) {
            Some(tid) => *tid,
            None => {
                let tid = AnyTransId::Fun(self.translated.fun_decls.reserve_slot());
                self.translated.id_map.insert(def_id, tid);
                self.translated.reverse_id_map.insert(tid, def_id);
                self.translated.all_ids.insert(tid);
                tid
            }
        };
        *tid.as_fun()
    }

    /// Compute the meta information for a Rust item identified by its id.
    fn translate_item_meta_from_rid(&mut self, instance: Instance) -> Result<ItemMeta, Error> {
        let span = self.translate_instance_span(instance);
        let name = self.def_id_to_name(instance.def.def_id())?;
        // TODO: populate the source text
        let source_text = None;
        // TODO: populate the attribute info
        let attr_info =
            AttrInfo { attributes: Vec::new(), inline: None, rename: None, public: true };

        // Aeneas only translates items that are local to the top-level crate
        // Since we want all reachable items (including those in external
        // crates) to be translated, always set `is_local` to true
        let is_local = true;

        // For now, assume all items are transparent
        let opacity = ItemOpacity::Transparent;

        Ok(ItemMeta { span, source_text, attr_info, name, is_local, opacity })
    }

    /// Retrieve an item name from a [DefId].
    /// This function is adapted from Charon:
    /// https://github.com/AeneasVerif/charon/blob/53530427db2941ce784201e64086766504bc5642/charon/src/bin/charon-driver/translate/translate_ctx.rs#L344
    fn def_id_to_name(&mut self, def_id: DefId) -> Result<Name, Error> {
        trace!("{:?}", def_id);
        let def_id = rustc_internal::internal(self.tcx(), def_id);
        let tcx = self.tcx();
        let span = tcx.def_span(def_id);

        // We have to be a bit careful when retrieving names from def ids. For instance,
        // due to reexports, [`TyCtxt::def_path_str`](TyCtxt::def_path_str) might give
        // different names depending on the def id on which it is called, even though
        // those def ids might actually identify the same definition.
        // For instance: `std::boxed::Box` and `alloc::boxed::Box` are actually
        // the same (the first one is a reexport).
        // This is why we implement a custom function to retrieve the original name
        // (though this makes us lose aliases - we may want to investigate this
        // issue in the future).

        // We lookup the path associated to an id, and convert it to a name.
        // Paths very precisely identify where an item is. There are important
        // subcases, like the items in an `Impl` block:
        // ```
        // impl<T> List<T> {
        //   fn new() ...
        // }
        // ```
        //
        // One issue here is that "List" *doesn't appear* in the path, which would
        // look like the following:
        //
        //   `TypeNS("Crate") :: Impl :: ValueNs("new")`
        //                       ^^^
        //           This is where "List" should be
        //
        // For this reason, whenever we find an `Impl` path element, we actually
        // lookup the type of the sub-path, from which we can derive a name.
        //
        // Besides, as there may be several "impl" blocks for one type, each impl
        // block is identified by a unique number (rustc calls this a
        // "disambiguator"), which we grab.
        //
        // Example:
        // ========
        // For instance, if we write the following code in crate `test` and module
        // `bla`:
        // ```
        // impl<T> Foo<T> {
        //   fn foo() { ... }
        // }
        //
        // impl<T> Foo<T> {
        //   fn bar() { ... }
        // }
        // ```
        //
        // The names we will generate for `foo` and `bar` are:
        // `[Ident("test"), Ident("bla"), Ident("Foo"), Disambiguator(0), Ident("foo")]`
        // `[Ident("test"), Ident("bla"), Ident("Foo"), Disambiguator(1), Ident("bar")]`
        let mut found_crate_name = false;
        let mut name: Vec<PathElem> = Vec::new();

        let def_path = tcx.def_path(def_id);
        let crate_name = tcx.crate_name(def_path.krate).to_string();

        let parents: Vec<_> = {
            let mut parents = vec![def_id];
            let mut cur_id = def_id;
            while let Some(parent) = tcx.opt_parent(cur_id) {
                parents.push(parent);
                cur_id = parent;
            }
            parents.into_iter().rev().collect()
        };

        // Rk.: below we try to be as tight as possible with regards to sanity
        // checks, to make sure we understand what happens with def paths, and
        // fail whenever we get something which is even slightly outside what
        // we expect.
        for cur_id in parents {
            let data = tcx.def_key(cur_id).disambiguated_data;
            // Match over the key data
            let disambiguator = Disambiguator::new(data.disambiguator as usize);
            use rustc_hir::definitions::DefPathData;
            match &data.data {
                DefPathData::TypeNs(symbol) => {
                    error_assert!(self, span, data.disambiguator == 0); // Sanity check
                    name.push(PathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::ValueNs(symbol) => {
                    // I think `disambiguator != 0` only with names introduced by macros (though
                    // not sure).
                    name.push(PathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::CrateRoot => {
                    // Sanity check
                    error_assert!(self, span, data.disambiguator == 0);

                    // This should be the beginning of the path
                    error_assert!(self, span, name.is_empty());
                    found_crate_name = true;
                    name.push(PathElem::Ident(crate_name.clone(), disambiguator));
                }
                DefPathData::Impl => todo!(),
                DefPathData::OpaqueTy => {
                    // TODO: do nothing for now
                }
                DefPathData::MacroNs(symbol) => {
                    error_assert!(self, span, data.disambiguator == 0); // Sanity check

                    // There may be namespace collisions between, say, function
                    // names and macros (not sure). However, this isn't much
                    // of an issue here, because for now we don't expose macros
                    // in the AST, and only use macro names in [register], for
                    // instance to filter opaque modules.
                    name.push(PathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::Closure => {
                    // TODO: this is not very satisfactory, but on the other hand
                    // we should be able to extract closures in local let-bindings
                    // (i.e., we shouldn't have to introduce top-level let-bindings).
                    name.push(PathElem::Ident("closure".to_string(), disambiguator))
                }
                DefPathData::ForeignMod => {
                    // Do nothing, functions in `extern` blocks are in the same namespace as the
                    // block.
                }
                _ => {
                    error_or_panic!(self, span, format!("Unexpected DefPathData: {:?}", data));
                }
            }
        }

        // We always add the crate name
        if !found_crate_name {
            name.push(PathElem::Ident(crate_name, Disambiguator::new(0)));
        }

        trace!("{:?}", name);
        Ok(Name { name })
    }

    /// Compute the span information for the given instance
    fn translate_instance_span(&mut self, instance: Instance) -> CharonSpan {
        self.translate_span(instance.def.span())
    }

    /// Compute the span information for MIR span
    fn translate_span(&mut self, span: Span) -> CharonSpan {
        let filename = FileName::Local(PathBuf::from(span.get_filename()));
        let file_id = match self.translated.file_to_id.get(&filename) {
            Some(file_id) => *file_id,
            None => {
                let file_id = self.translated.id_to_file.push(filename.clone());
                self.translated.file_to_id.insert(filename, file_id);
                file_id
            }
        };
        let lineinfo = span.get_lines();
        let rspan = RawSpan {
            file_id,
            beg: Loc { line: lineinfo.start_line, col: lineinfo.start_col },
            end: Loc { line: lineinfo.end_line, col: lineinfo.end_col },
            rust_span_data: rustc_internal::internal(self.tcx(), span).data(),
        };

        // TODO: populate `generated_from_span` info
        CharonSpan { span: rspan, generated_from_span: None }
    }

    fn translate_function_signature(&mut self) -> FunSig {
        let instance = self.instance;
        let fn_abi = instance.fn_abi().unwrap();
        let requires_caller_location = self.requires_caller_location(instance);
        let num_args = fn_abi.args.len();
        let args = fn_abi
            .args
            .iter()
            .enumerate()
            .filter_map(|(idx, arg_abi)| {
                // We ignore zero-sized parameters.
                // See https://github.com/model-checking/kani/issues/274 for more details.
                // We also ingore the last parameter if the function requires
                // caller location.
                if arg_abi.mode == PassMode::Ignore
                    || (requires_caller_location && idx + 1 == num_args)
                {
                    None
                } else {
                    let ty = arg_abi.ty;
                    debug!(?idx, ?arg_abi, "fn_typ");
                    Some(self.translate_ty(ty))
                }
            })
            .collect();

        debug!(?args, ?fn_abi, "function_type");
        let ret_type = self.translate_ty(fn_abi.ret.ty);

        // TODO: populate the rest of the information (`is_unsafe`, `is_closure`, etc.)
        FunSig {
            is_unsafe: false,
            is_closure: false,
            closure_info: None,
            generics: GenericParams::default(),
            parent_params_info: None,
            inputs: args,
            output: ret_type,
        }
    }

    fn translate_function_body(&mut self) -> Result<BodyId, Opaque> {
        let instance = self.instance;
        let mir_body = instance.body().unwrap();
        let body_id = self.translated.bodies.reserve_slot();
        let body = self.translate_body(mir_body);
        self.translated.bodies.set_slot(body_id, body);
        Ok(body_id)
    }

    fn translate_body(&mut self, mir_body: Body) -> CharonBody {
        let span = self.translate_span(mir_body.span);
        let arg_count = self.instance.fn_abi().unwrap().args.len();
        let locals = self.translate_body_locals(&mir_body);
        let body: BodyContents =
            mir_body.blocks.iter().map(|bb| self.translate_block(bb)).collect();

        let body_expr = ExprBody { span, arg_count, locals, body };
        CharonBody::Unstructured(body_expr)
    }

    fn requires_caller_location(&self, instance: Instance) -> bool {
        let instance_internal = rustc_internal::internal(self.tcx(), instance);
        instance_internal.def.requires_caller_location(self.tcx())
    }

    fn translate_ty(&self, ty: Ty) -> CharonTy {
        match ty.kind() {
            TyKind::RigidTy(rigid_ty) => self.translate_rigid_ty(rigid_ty),
            _ => todo!(),
        }
    }

    fn translate_rigid_ty(&self, rigid_ty: RigidTy) -> CharonTy {
        debug!("translate_rigid_ty: {rigid_ty:?}");
        match rigid_ty {
            RigidTy::Bool => CharonTy::Literal(LiteralTy::Bool),
            RigidTy::Char => CharonTy::Literal(LiteralTy::Char),
            RigidTy::Int(it) => CharonTy::Literal(LiteralTy::Integer(translate_int_ty(it))),
            RigidTy::Uint(uit) => CharonTy::Literal(LiteralTy::Integer(translate_uint_ty(uit))),
            RigidTy::Never => CharonTy::Never,
            RigidTy::Str => CharonTy::Adt(
                TypeId::Builtin(BuiltinTy::Str),
                // TODO: find out whether any of the information below should be
                // populated for strings
                GenericArgs {
                    regions: Vector::new(),
                    types: Vector::new(),
                    const_generics: Vector::new(),
                    trait_refs: Vector::new(),
                },
            ),
            RigidTy::Ref(region, ty, mutability) => CharonTy::Ref(
                self.translate_region(region),
                Box::new(self.translate_ty(ty)),
                match mutability {
                    Mutability::Mut => RefKind::Mut,
                    Mutability::Not => RefKind::Shared,
                },
            ),
            RigidTy::Tuple(ty) => {
                let types = ty.iter().map(|ty| self.translate_ty(*ty)).collect();
                // TODO: find out if any of the information below is needed
                let generic_args = GenericArgs {
                    regions: Vector::new(),
                    types,
                    const_generics: Vector::new(),
                    trait_refs: Vector::new(),
                };
                CharonTy::Adt(TypeId::Tuple, generic_args)
            }
            RigidTy::FnDef(def_id, _args) => {
                let sig = def_id.fn_sig().value;
                let inputs = sig.inputs().iter().map(|ty| self.translate_ty(*ty)).collect();
                let output = self.translate_ty(sig.output());
                // TODO: populate regions?
                CharonTy::Arrow(Vector::new(), inputs, Box::new(output))
            }
            _ => todo!(),
        }
    }

    fn translate_body_locals(&mut self, mir_body: &Body) -> Vector<VarId, Var> {
        // Charon expects the locals in the following order:
        // - the local used for the return value (index 0)
        // - the input arguments
        // - the remaining locals, used for the intermediate computations
        let mut locals = Vector::new();
        {
            let mut add_variable = make_locals_generator(&mut locals);
            mir_body.local_decls().for_each(|(_, local)| {
                add_variable(self.translate_ty(local.ty));
            });
        }
        locals
    }

    fn translate_block(&mut self, bb: &BasicBlock) -> BlockData {
        let statements =
            bb.statements.iter().filter_map(|stmt| self.translate_statement(stmt)).collect();
        let terminator = self.translate_terminator(&bb.terminator);
        BlockData { statements, terminator }
    }

    fn translate_statement(&mut self, stmt: &Statement) -> Option<CharonStatement> {
        let content = match &stmt.kind {
            StatementKind::Assign(place, rhs) => Some(RawStatement::Assign(
                self.translate_place(&place),
                self.translate_rvalue(&rhs),
            )),
            StatementKind::SetDiscriminant { place, variant_index } => {
                Some(RawStatement::SetDiscriminant(
                    self.translate_place(&place),
                    VariantId::from_usize(variant_index.to_index()),
                ))
            }
            StatementKind::StorageLive(_) => None,
            StatementKind::StorageDead(local) => {
                Some(RawStatement::StorageDead(VarId::from_usize(*local)))
            }
            StatementKind::Nop => None,
            _ => todo!(),
        };
        if let Some(content) = content {
            let span = self.translate_span(stmt.span);
            return Some(CharonStatement { span, content });
        };
        None
    }

    fn translate_terminator(&mut self, terminator: &Terminator) -> CharonTerminator {
        let span = self.translate_span(terminator.span);
        let content = match &terminator.kind {
            TerminatorKind::Return => RawTerminator::Return,
            TerminatorKind::Goto { target } => {
                RawTerminator::Goto { target: BlockId::from_usize(*target) }
            }
            TerminatorKind::Unreachable => RawTerminator::Abort(AbortKind::UndefinedBehavior),
            TerminatorKind::Drop { place, target, .. } => RawTerminator::Drop {
                place: self.translate_place(&place),
                target: BlockId::from_usize(*target),
            },
            TerminatorKind::SwitchInt { discr, targets } => {
                let (discr, targets) = self.translate_switch_targets(discr, targets);
                RawTerminator::Switch { discr, targets }
            }
            TerminatorKind::Call { func, args, destination, target, .. } => {
                debug!("translate_call: {func:?} {args:?} {destination:?} {target:?}");
                let fn_ty = func.ty(self.instance.body().unwrap().locals()).unwrap();
                let fn_ptr = match fn_ty.kind() {
                    TyKind::RigidTy(RigidTy::FnDef(def, args)) => {
                        let instance = Instance::resolve(def, &args).unwrap();
                        let def_id = rustc_internal::internal(self.tcx(), instance.def.def_id());
                        let fid = self.register_fun_decl_id(def_id);
                        FnPtr {
                            func: FunIdOrTraitMethodRef::Fun(FunId::Regular(fid)),
                            // TODO: populate generics?
                            generics: GenericArgs {
                                regions: Vector::new(),
                                types: Vector::new(),
                                const_generics: Vector::new(),
                                trait_refs: Vector::new(),
                            },
                        }
                    }
                    TyKind::RigidTy(RigidTy::FnPtr(..)) => todo!(),
                    x => unreachable!(
                        "Function call where the function was of unexpected type: {:?}",
                        x
                    ),
                };
                let func = FnOperand::Regular(fn_ptr);
                let call = Call {
                    func,
                    args: args.iter().map(|arg| self.translate_operand(arg)).collect(),
                    dest: self.translate_place(destination),
                };
                RawTerminator::Call { call, target: target.map(BlockId::from_usize) }
            }
            TerminatorKind::Assert { cond, expected, msg: _, target, .. } => {
                RawTerminator::Assert {
                    assert: Assert { cond: self.translate_operand(cond), expected: *expected },
                    target: BlockId::from_usize(*target),
                }
            }
            _ => todo!(),
        };
        CharonTerminator { span, content }
    }

    fn translate_place(&self, place: &Place) -> CharonPlace {
        let projection = self.translate_projection(&place.projection);
        let local = place.local;
        let var_id = VarId::from_usize(local);
        CharonPlace { var_id, projection }
    }

    fn translate_rvalue(&self, rvalue: &Rvalue) -> CharonRvalue {
        trace!("translate_rvalue: {rvalue:?}");
        match rvalue {
            Rvalue::Use(operand) => CharonRvalue::Use(self.translate_operand(operand)),
            Rvalue::Repeat(_operand, _) => todo!(),
            Rvalue::Ref(_region, kind, place) => {
                CharonRvalue::Ref(self.translate_place(&place), translate_borrow_kind(kind))
            }
            Rvalue::AddressOf(_, _) => todo!(),
            Rvalue::Len(place) => CharonRvalue::Len(
                self.translate_place(&place),
                self.translate_ty(rvalue.ty(self.instance.body().unwrap().locals()).unwrap()),
                None,
            ),
            Rvalue::Cast(kind, operand, ty) => CharonRvalue::UnaryOp(
                UnOp::Cast(self.translate_cast(*kind, operand, *ty)),
                self.translate_operand(operand),
            ),
            Rvalue::BinaryOp(bin_op, lhs, rhs) => CharonRvalue::BinaryOp(
                translate_bin_op(*bin_op),
                self.translate_operand(lhs),
                self.translate_operand(rhs),
            ),
            Rvalue::CheckedBinaryOp(_, _, _) => todo!(),
            Rvalue::UnaryOp(_, _) => todo!(),
            Rvalue::Discriminant(_) => todo!(),
            Rvalue::Aggregate(_, _) => todo!(),
            Rvalue::ShallowInitBox(_, _) => todo!(),
            Rvalue::CopyForDeref(_) => todo!(),
            Rvalue::ThreadLocalRef(_) => todo!(),
            _ => todo!(),
        }
    }

    fn translate_operand(&self, operand: &Operand) -> CharonOperand {
        trace!("translate_operand: {operand:?}");
        match operand {
            Operand::Constant(constant) => CharonOperand::Const(self.translate_constant(constant)),
            Operand::Copy(place) => CharonOperand::Copy(self.translate_place(&place)),
            Operand::Move(place) => CharonOperand::Move(self.translate_place(&place)),
        }
    }

    fn translate_constant(&self, constant: &ConstOperand) -> ConstantExpr {
        trace!("translate_constant: {constant:?}");
        let value = self.translate_constant_value(&constant.const_);
        ConstantExpr { value, ty: self.translate_ty(constant.ty()) }
    }

    fn translate_constant_value(&self, constant: &MirConst) -> RawConstantExpr {
        trace!("translate_constant_value: {constant:?}");
        match constant.kind() {
            ConstantKind::Allocated(alloc) => self.translate_allocation(alloc, constant.ty()),
            ConstantKind::Ty(_) => todo!(),
            ConstantKind::ZeroSized => todo!(),
            ConstantKind::Unevaluated(_) => todo!(),
            ConstantKind::Param(_) => todo!(),
        }
    }

    fn translate_allocation(&self, alloc: &Allocation, ty: Ty) -> RawConstantExpr {
        match ty.kind() {
            TyKind::RigidTy(RigidTy::Int(it)) => {
                let value = alloc.read_int().unwrap();
                let scalar_value = match it {
                    IntTy::I8 => ScalarValue::I8(value as i8),
                    IntTy::I16 => ScalarValue::I16(value as i16),
                    IntTy::I32 => ScalarValue::I32(value as i32),
                    IntTy::I64 => ScalarValue::I64(value as i64),
                    IntTy::I128 => ScalarValue::I128(value),
                    IntTy::Isize => ScalarValue::Isize(value as i64),
                };
                RawConstantExpr::Literal(Literal::Scalar(scalar_value))
            }
            TyKind::RigidTy(RigidTy::Uint(uit)) => {
                let value = alloc.read_uint().unwrap();
                let scalar_value = match uit {
                    UintTy::U8 => ScalarValue::U8(value as u8),
                    UintTy::U16 => ScalarValue::U16(value as u16),
                    UintTy::U32 => ScalarValue::U32(value as u32),
                    UintTy::U64 => ScalarValue::U64(value as u64),
                    UintTy::U128 => ScalarValue::U128(value),
                    UintTy::Usize => ScalarValue::Usize(value as u64),
                };
                RawConstantExpr::Literal(Literal::Scalar(scalar_value))
            }
            TyKind::RigidTy(RigidTy::Bool) => {
                let value = alloc.read_bool().unwrap();
                RawConstantExpr::Literal(Literal::Bool(value))
            }
            TyKind::RigidTy(RigidTy::Char) => {
                let value = char::from_u32(alloc.read_uint().unwrap() as u32);
                RawConstantExpr::Literal(Literal::Char(value.unwrap()))
            }
            _ => todo!(),
        }
    }

    fn translate_cast(&self, _kind: CastKind, _operand: &Operand, _ty: Ty) -> CharonCastKind {
        todo!()
    }

    fn translate_switch_targets(
        &self,
        discr: &Operand,
        targets: &SwitchTargets,
    ) -> (CharonOperand, CharonSwitchTargets) {
        trace!("translate_switch_targets: {discr:?} {targets:?}");
        let ty = discr.ty(self.instance.body().unwrap().locals()).unwrap();
        let discr = self.translate_operand(discr);
        let charon_ty = self.translate_ty(ty);
        let switch_targets = if ty.kind().is_bool() {
            // Charon/Aeneas expects types with a bool discriminant to be translated to an `If`
            // `len` includes the `otherwise` branch
            assert_eq!(targets.len(), 2);
            let (value, bb) = targets.branches().last().unwrap();
            let (then_bb, else_bb) =
                if value == 0 { (targets.otherwise(), bb) } else { (bb, targets.otherwise()) };
            CharonSwitchTargets::If(BlockId::from_usize(then_bb), BlockId::from_usize(else_bb))
        } else {
            let CharonTy::Literal(LiteralTy::Integer(int_ty)) = charon_ty else {
                panic!("Expected integer type for switch discriminant");
            };
            let branches = targets
                .branches()
                .map(|(value, bb)| {
                    let scalar_val = match int_ty {
                        IntegerTy::I8 => ScalarValue::I8(value as i8),
                        IntegerTy::I16 => ScalarValue::I16(value as i16),
                        IntegerTy::I32 => ScalarValue::I32(value as i32),
                        IntegerTy::I64 => ScalarValue::I64(value as i64),
                        IntegerTy::I128 => ScalarValue::I128(value as i128),
                        IntegerTy::Isize => ScalarValue::Isize(value as i64),
                        IntegerTy::U8 => ScalarValue::U8(value as u8),
                        IntegerTy::U16 => ScalarValue::U16(value as u16),
                        IntegerTy::U32 => ScalarValue::U32(value as u32),
                        IntegerTy::U64 => ScalarValue::U64(value as u64),
                        IntegerTy::U128 => ScalarValue::U128(value),
                        IntegerTy::Usize => ScalarValue::Usize(value as u64),
                    };
                    (scalar_val, BlockId::from_usize(bb))
                })
                .collect();
            let otherwise = BlockId::from_usize(targets.otherwise());
            CharonSwitchTargets::SwitchInt(int_ty, branches, otherwise)
        };
        (discr, switch_targets)
    }

    fn translate_projection(&self, projection: &[ProjectionElem]) -> Vec<CharonProjectionElem> {
        projection.iter().map(|elem| self.translate_projection_elem(elem)).collect()
    }

    fn translate_projection_elem(&self, projection_elem: &ProjectionElem) -> CharonProjectionElem {
        match projection_elem {
            ProjectionElem::Deref => CharonProjectionElem::Deref,
            _ => todo!(),
        }
    }

    fn translate_region(&self, region: Region) -> CharonRegion {
        match region.kind {
            RegionKind::ReStatic => CharonRegion::Static,
            RegionKind::ReErased => CharonRegion::Erased,
            RegionKind::ReEarlyParam(_)
            | RegionKind::ReBound(_, _)
            | RegionKind::RePlaceholder(_) => todo!(),
        }
    }
}

fn translate_int_ty(int_ty: IntTy) -> IntegerTy {
    match int_ty {
        IntTy::I8 => IntegerTy::I8,
        IntTy::I16 => IntegerTy::I16,
        IntTy::I32 => IntegerTy::I32,
        IntTy::I64 => IntegerTy::I64,
        IntTy::I128 => IntegerTy::I128,
        // TODO: assumes 64-bit platform
        IntTy::Isize => IntegerTy::I64,
    }
}

fn translate_uint_ty(uint_ty: UintTy) -> IntegerTy {
    match uint_ty {
        UintTy::U8 => IntegerTy::U8,
        UintTy::U16 => IntegerTy::U16,
        UintTy::U32 => IntegerTy::U32,
        UintTy::U64 => IntegerTy::U64,
        UintTy::U128 => IntegerTy::U128,
        // TODO: assumes 64-bit platform
        UintTy::Usize => IntegerTy::U64,
    }
}

fn translate_bin_op(bin_op: BinOp) -> CharonBinOp {
    match bin_op {
        BinOp::Add | BinOp::AddUnchecked => CharonBinOp::Add,
        BinOp::Sub | BinOp::SubUnchecked => CharonBinOp::Sub,
        BinOp::Mul | BinOp::MulUnchecked => CharonBinOp::Mul,
        BinOp::Div => CharonBinOp::Div,
        BinOp::Rem => CharonBinOp::Rem,
        BinOp::BitXor => CharonBinOp::BitXor,
        BinOp::BitAnd => CharonBinOp::BitAnd,
        BinOp::BitOr => CharonBinOp::BitOr,
        BinOp::Shl | BinOp::ShlUnchecked => CharonBinOp::Shl,
        BinOp::Shr | BinOp::ShrUnchecked => CharonBinOp::Shr,
        BinOp::Eq => CharonBinOp::Eq,
        BinOp::Lt => CharonBinOp::Lt,
        BinOp::Le => CharonBinOp::Le,
        BinOp::Ne => CharonBinOp::Ne,
        BinOp::Ge => CharonBinOp::Ge,
        BinOp::Gt => CharonBinOp::Gt,
        BinOp::Cmp => todo!(),
        BinOp::Offset => todo!(),
    }
}

fn translate_borrow_kind(kind: &BorrowKind) -> CharonBorrowKind {
    match kind {
        BorrowKind::Shared => CharonBorrowKind::Shared,
        BorrowKind::Mut { .. } => CharonBorrowKind::Mut,
        BorrowKind::Fake(_kind) => todo!(),
    }
}
