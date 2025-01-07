// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! This module contains a context for translating stable MIR into Charon's
//! unstructured low-level borrow calculus (ULLBC)

use charon_lib::ast::krate::TypeDeclId as CharonTypeDeclId;
use charon_lib::ast::meta::{
    AttrInfo as CharonAttrInfo, Loc as CharonLoc, RawSpan as CharonRawSpan,
};
use charon_lib::ast::types::{Ty as CharonTy, TyKind as CharonTyKind};
use charon_lib::ast::{
    AbortKind as CharonAbortKind, AggregateKind as CharonAggregateKind,
    AnyTransId as CharonAnyTransId, Assert as CharonAssert, BinOp as CharonBinOp,
    Body as CharonBody, BodyId as CharonBodyId, BorrowKind as CharonBorrowKind,
    BuiltinTy as CharonBuiltinTy, Call as CharonCall, CastKind as CharonCastKind,
    ConstGeneric as CharonConstGeneric, ConstGenericVar as CharonConstGenericVar,
    ConstGenericVarId as CharonConstGenericVarId, ConstantExpr as CharonConstantExpr,
    DeBruijnId as CharonDeBruijnId, DeBruijnVar as CharonDeBruijnVar,
    Disambiguator as CharonDisambiguator, ExistentialPredicate as CharonExistentialPredicate,
    Field as CharonField, FieldId as CharonFieldId, FieldProjKind as CharonFieldProjKind,
    File as CharonFile, FileId as CharonFileId, FileName as CharonFileName,
    FnOperand as CharonFnOperand, FnPtr as CharonFnPtr, FunDecl as CharonFunDecl,
    FunDeclId as CharonFunDeclId, FunId as CharonFunId,
    FunIdOrTraitMethodRef as CharonFunIdOrTraitMethodRef, FunSig as CharonFunSig,
    GenericArgs as CharonGenericArgs, GenericParams as CharonGenericParams,
    GlobalDeclId as CharonGlobalDeclId, GlobalDeclRef as CharonGlobalDeclRef,
    IntegerTy as CharonIntegerTy, ItemKind as CharonItemKind, ItemMeta as CharonItemMeta,
    ItemOpacity as CharonItemOpacity, Literal as CharonLiteral, LiteralTy as CharonLiteralTy,
    Locals as CharonLocals, Name as CharonName, Opaque as CharonOpaque, Operand as CharonOperand,
    PathElem as CharonPathElem, Place as CharonPlace, PolyTraitDeclRef as CharonPolyTraitDeclRef,
    PredicateOrigin as CharonPredicateOrigin, ProjectionElem as CharonProjectionElem,
    RawConstantExpr as CharonRawConstantExpr, RefKind as CharonRefKind, Region as CharonRegion,
    RegionBinder as CharonRegionBinder, RegionId as CharonRegionId, RegionVar as CharonRegionVar,
    Rvalue as CharonRvalue, ScalarValue as CharonScalarValue, Span as CharonSpan,
    TraitClause as CharonTraitClause, TraitClauseId as CharonTraitClauseId,
    TraitDecl as CharonTraitDecl, TraitDeclId as CharonTraitDeclId,
    TraitDeclRef as CharonTraitDeclRef, TraitImplId as CharonTraitImplId,
    TraitRef as CharonTraitRef, TraitRefKind as CharonTraitRefKind,
    TranslatedCrate as CharonTranslatedCrate, TypeDecl as CharonTypeDecl,
    TypeDeclKind as CharonTypeDeclKind, TypeId as CharonTypeId, TypeVar as CharonTypeVar,
    TypeVarId as CharonTypeVarId, UnOp as CharonUnOp, Var as CharonVar, VarId as CharonVarId,
    Variant as CharonVariant, VariantId as CharonVariantId,
};
use charon_lib::errors::{Error as CharonError, ErrorCtx as CharonErrorCtx};
use charon_lib::ids::Vector as CharonVector;
use charon_lib::ullbc_ast::{
    BlockData as CharonBlockData, BlockId as CharonBlockId, BodyContents as CharonBodyContents,
    ExprBody as CharonExprBody, RawStatement as CharonRawStatement,
    RawTerminator as CharonRawTerminator, Statement as CharonStatement,
    SwitchTargets as CharonSwitchTargets, Terminator as CharonTerminator,
};
use charon_lib::{error_assert, error_or_panic};
use core::panic;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::{TyCtxt, TypingEnv};
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::{Instance, InstanceDef};
use stable_mir::mir::{
    AggregateKind, BasicBlock, BinOp, Body, BorrowKind, CastKind, ConstOperand, Local, Mutability,
    Operand, Place, ProjectionElem, Rvalue, Statement, StatementKind, SwitchTargets, Terminator,
    TerminatorKind, UnOp, VarDebugInfoContents,
};
use stable_mir::ty::{
    AdtDef, AdtKind, Allocation, ConstantKind, FnDef, GenericArgKind, GenericArgs,
    GenericParamDefKind, IndexedVal, IntTy, MirConst, Region, RegionKind, RigidTy, Span, TraitDecl,
    TraitDef, Ty, TyConst, TyConstKind, TyKind, UintTy,
};
use stable_mir::{CrateDef, CrateDefType, DefId};
use std::collections::HashMap;
use std::iter::zip;
use std::path::PathBuf;
use tracing::{debug, trace};

/// A context for translating a single MIR function to ULLBC.
/// The results of the translation are stored in the `translated` field.
pub struct Context<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    instance: Instance,
    translated: &'a mut CharonTranslatedCrate,
    id_map: &'a mut FxHashMap<DefId, CharonAnyTransId>,
    errors: &'a mut CharonErrorCtx,
    local_names: FxHashMap<Local, String>,
    file_to_id: HashMap<CharonFileName, CharonFileId>,
}

impl<'a, 'tcx> Context<'a, 'tcx> {
    /// Create a new context for translating the function `instance`, populating
    /// the results of the translation in `translated`
    pub fn new(
        tcx: TyCtxt<'tcx>,
        instance: Instance,
        translated: &'a mut CharonTranslatedCrate,
        id_map: &'a mut FxHashMap<DefId, CharonAnyTransId>,
        errors: &'a mut CharonErrorCtx,
    ) -> Self {
        let mut local_names = FxHashMap::default();
        // populate names of locals
        for info in instance.body().unwrap().var_debug_info {
            if let VarDebugInfoContents::Place(p) = info.value {
                if p.projection.is_empty() {
                    local_names.insert(p.local, info.name);
                }
            }
        }
        let file_to_id: HashMap<CharonFileName, CharonFileId> = HashMap::new();
        Self { tcx, instance, translated, id_map, errors, local_names, file_to_id }
    }

    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn span_err(&mut self, span: CharonSpan, msg: &str) {
        self.errors.span_err(self.translated, span, msg);
    }

    fn continue_on_failure(&self) -> bool {
        self.errors.continue_on_failure()
    }

    fn translate_traitdecl(&mut self, trait_def: TraitDef) -> CharonTraitDeclId {
        let trait_def_id = trait_def.def_id();
        let trait_decl_id = self.register_trait_decl_id(trait_def_id);
        match self.translated.trait_decls.get(trait_decl_id) {
            None => {
                let trait_decl = TraitDef::declaration(&trait_def);
                let consts = Vec::new();
                let const_defaults = HashMap::new();
                let types = Vec::new();
                let type_clauses = Vec::new();
                let type_defaults = HashMap::new();
                let required_methods = Vec::new();
                let provided_methods = Vec::new();
                let parent_clauses = CharonVector::new();
                let c_traitdecl = CharonTraitDecl {
                    def_id: trait_decl_id,
                    item_meta: self.translate_item_meta_from_defid(trait_def_id),
                    generics: self.generic_params_from_traitdecl(trait_decl),
                    parent_clauses,
                    type_clauses,
                    consts,
                    const_defaults,
                    types,
                    type_defaults,
                    required_methods,
                    provided_methods,
                };
                self.translated.trait_decls.set_slot(trait_decl_id, c_traitdecl);
                trait_decl_id
            }
            Some(_) => trait_decl_id,
        }
    }

    //This function extract the traitrefs and their span from a def_id
    //Those information will be added into that def_id's generic args
    //Note that Generic args of Charon contains trait_refs while those of stable_mir do not 
    fn get_traitrefs_and_span_from_defid(
        &mut self,
        defid: DefId,
    ) -> (CharonVector<CharonTraitClauseId, CharonTraitRef>, Vec<CharonSpan>) {
        let inter_defid = rustc_internal::internal(self.tcx, defid);
        let predicates = self.tcx().predicates_of(inter_defid).predicates.to_vec();
        let mut c_trait_refs: CharonVector<CharonTraitClauseId, CharonTraitRef> =
            CharonVector::new();
        let mut c_spans = Vec::new();
        for (i, (clause, span)) in predicates.iter().enumerate() {
            let trait_clause = clause.as_trait_clause();
            if trait_clause.is_none() {
                continue;
            };
            let trait_id_internal = clause.as_trait_clause().unwrap();
            let trait_binder = rustc_internal::stable(trait_id_internal);
            let trait_ref = trait_binder.value.trait_ref;
            let trait_def = trait_ref.def_id;
            if self.is_marker_trait(trait_def) {
                continue;
            };
            let c_traitdecl_id = self.translate_traitdecl(trait_def);
            let c_genarg = self.translate_generic_args_without_trait(trait_ref.args().clone());
            let c_polytrait = CharonPolyTraitDeclRef {
                regions: CharonVector::new(),
                skip_binder: CharonTraitDeclRef {
                    trait_id: c_traitdecl_id,
                    generics: c_genarg.clone(),
                },
            };
            let debr = CharonDeBruijnVar::free(CharonTraitClauseId::from_usize(i));
            let c_traitref = CharonTraitRef {
                kind: CharonTraitRefKind::Clause(debr),
                trait_decl_ref: c_polytrait,
            };
            c_trait_refs.push(c_traitref);
            c_spans.push(self.translate_span(rustc_internal::stable(span)));
        }
        (c_trait_refs, c_spans)
    }

    fn get_traitclauses_from_defid(
        &mut self,
        defid: DefId,
    ) -> CharonVector<CharonTraitClauseId, CharonTraitClause> {
        let inter_defid = rustc_internal::internal(self.tcx, defid);
        let predicates = self.tcx().predicates_of(inter_defid).predicates.to_vec();
        let mut c_trait_clauses: CharonVector<CharonTraitClauseId, CharonTraitClause> =
            CharonVector::new();
        for (i, (clause, span)) in predicates.iter().enumerate() {
            let trait_clause = clause.as_trait_clause();
            if trait_clause.is_none() {
                continue;
            };
            let trait_id_internal = clause.as_trait_clause().unwrap();
            let trait_ref = rustc_internal::stable(trait_id_internal).value.trait_ref;
            let trait_def = trait_ref.def_id;
            if self.is_marker_trait(trait_def) {
                continue;
            };
            let c_traitdecl_id = self.translate_traitdecl(trait_def);
            let c_genarg = self.translate_generic_args_without_trait(trait_ref.args().clone());
            let c_polytrait = CharonPolyTraitDeclRef {
                regions: CharonVector::new(),
                skip_binder: CharonTraitDeclRef { trait_id: c_traitdecl_id, generics: c_genarg },
            };
            let c_traitclause = CharonTraitClause {
                clause_id: CharonTraitClauseId::from_usize(i),
                span: Some(self.translate_span(rustc_internal::stable(span))),
                trait_: c_polytrait,
                origin: CharonPredicateOrigin::WhereClauseOnType,
            };
            c_trait_clauses.push(c_traitclause);
        }

        c_trait_clauses
    }

    /// Perform the translation
    pub fn translate(&mut self) -> Result<(), ()> {
        // TODO: might want to populate `errors.dep_sources` to help with
        // debugging
        let instance_def = self.instance.def;

        let is_builtin = self.is_builtin_fun(instance_def);

        debug!("Func name: {:?}", self.instance.name());
        let fid = self.register_fun_decl_id(self.instance.def.def_id());

        let item_meta = match self.translate_item_meta_from_rid(self.instance) {
            Ok(item_meta) => item_meta,
            Err(_) => {
                return Err(());
            }
        };
        let funcname = item_meta.name.clone();
        let signature = self.translate_function_signature(self.instance);
        let body = if is_builtin {
            Err(CharonOpaque)
        } else {
            let bodyid = match self.translate_function_body(self.instance) {
                Ok(body) => body,
                Err(_) => {
                    return Err(());
                }
            };
            Ok(bodyid)
        };
        let fun_decl = CharonFunDecl {
            def_id: fid,
            item_meta,
            signature,
            kind: CharonItemKind::Regular,
            is_global_initializer: None,
            body,
        };
        if self.translated.fun_decls.get(fid).is_none() {
            self.translated.fun_decls.set_slot(fid, fun_decl)
        };
        debug!("Complete Func name: {:?}", funcname);
        Ok(())
    }

    /// Get or create a `CharonFunDeclId` for the given function
    fn register_fun_decl_id(&mut self, def_id: DefId) -> CharonFunDeclId {
        debug!("register_fun_decl_id: {:?}", def_id);
        let tid = match self.id_map.get(&def_id) {
            Some(tid) => *tid,
            None => {
                debug!("***Not found fun_decl_id!");
                let tid = CharonAnyTransId::Fun(self.translated.fun_decls.reserve_slot());
                self.id_map.insert(def_id, tid);
                self.translated.all_ids.insert(tid);
                tid
            }
        };
        debug!("register_fun_decl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    fn register_type_decl_id(&mut self, def_id: DefId) -> CharonTypeDeclId {
        debug!("register_type_decl_id: {:?}", def_id);
        let tid = match self.id_map.get(&def_id) {
            Some(tid) => *tid,
            None => {
                debug!("***Not found type_decl_id!");
                let tid = CharonAnyTransId::Type(self.translated.type_decls.reserve_slot());
                self.id_map.insert(def_id, tid);
                self.translated.all_ids.insert(tid);
                tid
            }
        };
        debug!("register_type_decl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    fn register_trait_decl_id(&mut self, def_id: DefId) -> CharonTraitDeclId {
        debug!("register_trait_decl_id: {:?}", def_id);
        let tid = match self.id_map.get(&def_id) {
            Some(tid) => *tid,
            None => {
                debug!("***Not found trait_decl_id!");
                let tid = CharonAnyTransId::TraitDecl(self.translated.trait_decls.reserve_slot());
                self.id_map.insert(def_id, tid);
                self.translated.all_ids.insert(tid);
                tid
            }
        };
        debug!("register_trait_decl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    fn register_trait_impl_id(&mut self, def_id: DefId) -> CharonTraitImplId {
        debug!("register_trait_impl_id: {:?}", def_id);
        let tid = match self.id_map.get(&def_id) {
            Some(tid) => *tid,
            None => {
                debug!("***Not found trait_impl_id!");
                let tid = CharonAnyTransId::TraitImpl(self.translated.trait_impls.reserve_slot());
                self.id_map.insert(def_id, tid);
                self.translated.all_ids.insert(tid);
                tid
            }
        };
        debug!("register_trait_impl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    fn register_global_decl_id(&mut self, def_id: DefId) -> CharonGlobalDeclId {
        debug!("register_global_decl_id: {:?}", def_id);
        let tid = match self.id_map.get(&def_id) {
            Some(tid) => *tid,
            None => {
                debug!("***Not found global_decl_id!");
                let tid = CharonAnyTransId::Global(self.translated.global_decls.reserve_slot());
                self.id_map.insert(def_id, tid);
                self.translated.all_ids.insert(tid);
                tid
            }
        };
        debug!("register_global_decl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    // similar to register_type_decl_id, but not adding new def_id, used for cases where the def_id has been registered, or in functions that take immut &self
    fn get_type_decl_id(&self, def_id: DefId) -> CharonTypeDeclId {
        debug!("register_type_decl_id: {:?}", def_id);
        let tid = *self.id_map.get(&def_id).unwrap();
        debug!("register_type_decl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    fn get_discriminant(&mut self, discr_val: u128, ty: Ty) -> CharonScalarValue {
        let ty = self.translate_ty(ty);
        let int_ty = *ty.kind().as_literal().unwrap().as_integer().unwrap();
        CharonScalarValue::from_bits(int_ty, discr_val)
    }

    //Get the GenericParams for Trait Decl, which is neccessary in Trait Decl translation
    fn generic_params_from_traitdecl(&mut self, traitdecl: TraitDecl) -> CharonGenericParams {
        let genvec = traitdecl.generics_of().params;
        let mut c_regions: CharonVector<CharonRegionId, CharonRegionVar> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTypeVar> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGenericVar> =
            CharonVector::new();
        for gendef in genvec.iter() {
            let genkind = gendef.kind.clone();
            let index = gendef.index as usize;
            let name = gendef.name.clone();
            match genkind {
                GenericParamDefKind::Lifetime => {
                    let c_region = CharonRegionVar {
                        index: CharonRegionId::from_usize(index),
                        name: Some(name),
                    };
                    c_regions.push(c_region);
                }
                GenericParamDefKind::Type { has_default: _, synthetic: _ } => {
                    let c_region =
                        CharonTypeVar { index: CharonTypeVarId::from_usize(index), name };
                    c_types.push(c_region);
                }
                GenericParamDefKind::Const { has_default: _ } => {
                    let def_id_internal = rustc_internal::internal(self.tcx, gendef.def_id.0);
                    let pc_internal = rustc_middle::ty::ParamConst {
                        index: index as u32,
                        name: rustc_span::Symbol::intern(&name.clone()),
                    };
                    let paramenv = TypingEnv::post_analysis(self.tcx, def_id_internal).param_env;
                    let ty_internal = pc_internal.find_ty_from_env(paramenv);
                    let ty_stable = rustc_internal::stable(ty_internal);
                    let trans_ty = self.translate_ty(ty_stable);
                    let lit_ty = match trans_ty.kind() {
                        CharonTyKind::Literal(lit) => *lit,
                        _ => panic!("generic_params_from_fndef: not a literal type"),
                    };
                    let c_constgeneric = CharonConstGenericVar {
                        index: CharonConstGenericVarId::from_usize(index),
                        name,
                        ty: lit_ty,
                    };
                    c_const_generics.push(c_constgeneric);
                }
            }
        }
        CharonGenericParams {
            regions: c_regions,
            types: c_types,
            const_generics: c_const_generics,
            trait_clauses: CharonVector::new(),
            regions_outlive: Vec::new(),
            types_outlive: Vec::new(),
            trait_type_constraints: Vec::new(),
        }
    }

    //Get the GenericParams for Func Decl, which is neccessary in Func Decl translation
    fn generic_params_from_fndef(&mut self, fndef: FnDef, input: Vec<Ty>) -> CharonGenericParams {
        let genvec = match fndef.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(_, genarg)) => genarg.0,
            _ => panic!("generic_params_from_fndef: not an FnDef"),
        };
        let mut c_regions: CharonVector<CharonRegionId, CharonRegionVar> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTypeVar> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGenericVar> =
            CharonVector::new();
        for genkind in genvec.iter() {
            let gk = genkind.clone();
            match gk {
                GenericArgKind::Lifetime(region) => match region.kind {
                    RegionKind::ReEarlyParam(epr) => {
                        let c_region = CharonRegionVar {
                            index: CharonRegionId::from_usize(epr.index as usize),
                            name: Some(epr.name),
                        };
                        c_regions.push(c_region);
                    }
                    _ => panic!("generic_params_from_adtdef: not an early bound region"),
                },
                GenericArgKind::Type(ty) => match ty.kind() {
                    TyKind::Param(paramty) => {
                        let c_typevar = CharonTypeVar {
                            index: CharonTypeVarId::from_usize(paramty.index as usize),
                            name: paramty.name,
                        };
                        c_types.push(c_typevar);
                    }
                    _ => panic!("generic_params_from_adtdef: not a param type"),
                },
                GenericArgKind::Const(tc) => match tc.kind() {
                    TyConstKind::Param(paramtc) => {
                        let def_id_internal = rustc_internal::internal(self.tcx, fndef.def_id());
                        let paramenv = self.tcx.param_env(def_id_internal);
                        let pc_internal = rustc_middle::ty::ParamConst {
                            index: paramtc.index,
                            name: rustc_span::Symbol::intern(&paramtc.name),
                        };
                        let ty_internal = pc_internal.find_ty_from_env(paramenv);
                        let ty_stable = rustc_internal::stable(ty_internal);
                        let trans_ty = self.translate_ty(ty_stable);
                        let lit_ty = match trans_ty.kind() {
                            CharonTyKind::Literal(lit) => *lit,
                            _ => panic!("generic_params_from_fndef: not a literal type"),
                        };
                        let c_constgeneric = CharonConstGenericVar {
                            index: CharonConstGenericVarId::from_usize(paramtc.index as usize),
                            name: paramtc.name.clone(),
                            ty: lit_ty,
                        };
                        c_const_generics.push(c_constgeneric);
                    }
                    _ => panic!("generic_params_from_fndef: not a param const"),
                },
            }
        }
        for inpty in input.iter() {
            if let TyKind::RigidTy(RigidTy::Ref(r, _, _)) = inpty.kind() {
                if let RegionKind::ReBound(_, br) = r.kind {
                    let id = br.var as usize;
                    let c_region =
                        CharonRegionVar { index: CharonRegionId::from_usize(id), name: None };
                    c_regions.push(c_region);
                }
            }
        }
        let trait_clauses = self.get_traitclauses_from_defid(fndef.def_id());
        CharonGenericParams {
            regions: c_regions,
            types: c_types,
            const_generics: c_const_generics,
            trait_clauses,
            regions_outlive: Vec::new(),
            types_outlive: Vec::new(),
            trait_type_constraints: Vec::new(),
        }
    }

    //Get the GenericParams for Adt Decl, which is neccessary in Adt Decl translation
    fn generic_params_from_adtdef(&mut self, adtdef: AdtDef) -> CharonGenericParams {
        let genvec = match adtdef.ty().kind() {
            TyKind::RigidTy(RigidTy::Adt(_, genarg)) => genarg.0,
            _ => panic!("generic_params_from_adtdef: not an adtdef"),
        };
        let mut c_regions: CharonVector<CharonRegionId, CharonRegionVar> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTypeVar> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGenericVar> =
            CharonVector::new();
        for genkind in genvec.iter() {
            let gk = genkind.clone();
            match gk {
                GenericArgKind::Lifetime(region) => match region.kind {
                    RegionKind::ReEarlyParam(epr) => {
                        let c_region = CharonRegionVar {
                            index: CharonRegionId::from_usize(epr.index as usize),
                            name: Some(epr.name),
                        };
                        c_regions.push(c_region);
                    }
                    _ => panic!("generic_params_from_adtdef: not an early bound region"),
                },
                GenericArgKind::Type(ty) => match ty.kind() {
                    TyKind::Param(paramty) => {
                        let c_typevar = CharonTypeVar {
                            index: CharonTypeVarId::from_usize(paramty.index as usize),
                            name: paramty.name,
                        };
                        c_types.push(c_typevar);
                    }
                    _ => panic!("generic_params_from_adtdef: not a param type"),
                },
                GenericArgKind::Const(tc) => match tc.kind() {
                    TyConstKind::Param(paramtc) => {
                        let def_id_internal = rustc_internal::internal(self.tcx, adtdef.def_id());
                        let paramenv =
                            TypingEnv::post_analysis(self.tcx, def_id_internal).param_env;
                        let pc_internal = rustc_middle::ty::ParamConst {
                            index: paramtc.index,
                            name: rustc_span::Symbol::intern(&paramtc.name),
                        };
                        let ty_internal = pc_internal.find_ty_from_env(paramenv);
                        let ty_stable = rustc_internal::stable(ty_internal);
                        let trans_ty = self.translate_ty(ty_stable);
                        let lit_ty = match trans_ty.kind() {
                            CharonTyKind::Literal(lit) => *lit,
                            _ => panic!("generic_params_from_adtdef: not a literal type"),
                        };
                        let c_constgeneric = CharonConstGenericVar {
                            index: CharonConstGenericVarId::from_usize(paramtc.index as usize),
                            name: paramtc.name.clone(),
                            ty: lit_ty,
                        };
                        c_const_generics.push(c_constgeneric);
                    }
                    _ => panic!("generic_params_from_adtdef: not a param const"),
                },
            }
        }
        let trait_clauses = self.get_traitclauses_from_defid(adtdef.def_id());
        CharonGenericParams {
            regions: c_regions,
            types: c_types,
            const_generics: c_const_generics,
            trait_clauses,
            regions_outlive: Vec::new(),
            types_outlive: Vec::new(),
            trait_type_constraints: Vec::new(),
        }
    }

    fn translate_adtdef(&mut self, adt_def: AdtDef) -> CharonTypeDecl {
        let c_genparam = self.generic_params_from_adtdef(adt_def);
        let item_meta = self.translate_item_meta_adt(adt_def).unwrap();
        match adt_def.kind() {
            AdtKind::Enum => {
                let def_id = adt_def.def_id();
                let c_typedeclid = self.register_type_decl_id(def_id);
                let mut c_variants: CharonVector<CharonVariantId, CharonVariant> =
                    CharonVector::new();
                for var_def in adt_def.variants_iter() {
                    let mut c_fields: CharonVector<CharonFieldId, CharonField> =
                        CharonVector::new();
                    for field_def in var_def.fields() {
                        let c_field_ty = self.translate_ty(field_def.ty());
                        let c_field_name = Some(field_def.name);
                        let c_span = self.translate_span(adt_def.span());
                        let c_field = CharonField {
                            span: c_span,
                            attr_info: CharonAttrInfo {
                                attributes: Vec::new(),
                                inline: None,
                                rename: None,
                                public: true,
                            },
                            name: c_field_name,
                            ty: c_field_ty,
                        };
                        c_fields.push(c_field);
                    }
                    let var_name = var_def.name();
                    let span = self.translate_span(adt_def.span());

                    let adtdef_internal = rustc_internal::internal(self.tcx, adt_def);
                    let variant_index_internal = rustc_internal::internal(self.tcx, var_def.idx);
                    let discr =
                        adtdef_internal.discriminant_for_variant(self.tcx, variant_index_internal);
                    let discr_val = discr.val;
                    let discr_ty = rustc_internal::stable(discr.ty);
                    let c_discr = self.get_discriminant(discr_val, discr_ty);

                    let c_variant = CharonVariant {
                        span,
                        attr_info: CharonAttrInfo {
                            attributes: Vec::new(),
                            inline: None,
                            rename: None,
                            public: true,
                        },
                        name: var_name,
                        fields: c_fields,
                        discriminant: c_discr,
                    };
                    let c_varidx = c_variants.push(c_variant);
                    assert_eq!(c_varidx.index(), var_def.idx.to_index());
                }
                let typedecl = CharonTypeDecl {
                    def_id: c_typedeclid,
                    generics: c_genparam,
                    kind: CharonTypeDeclKind::Enum(c_variants),
                    item_meta,
                };
                self.translated.type_decls.set_slot(c_typedeclid, typedecl.clone());
                typedecl
            }
            AdtKind::Struct => {
                let def_id = adt_def.def_id();
                let c_typedeclid = self.register_type_decl_id(def_id);
                let mut c_fields: CharonVector<CharonFieldId, CharonField> = CharonVector::new();
                let only_variant = *adt_def.variants().first().unwrap();
                let fields = only_variant.fields();
                for field_def in fields {
                    let c_field_ty = self.translate_ty(field_def.ty());
                    let c_field_name = Some(field_def.name);
                    let c_span = self.translate_span(adt_def.span());
                    let c_field = CharonField {
                        span: c_span,
                        attr_info: CharonAttrInfo {
                            attributes: Vec::new(),
                            inline: None,
                            rename: None,
                            public: true,
                        },
                        name: c_field_name,
                        ty: c_field_ty,
                    };
                    c_fields.push(c_field);
                }
                let typedecl = CharonTypeDecl {
                    def_id: c_typedeclid,
                    generics: c_genparam,
                    kind: CharonTypeDeclKind::Struct(c_fields),
                    item_meta,
                };
                self.translated.type_decls.set_slot(c_typedeclid, typedecl.clone());
                typedecl
            }
            _ => todo!(),
        }
    }

    /// Compute the meta information for a Rust item identified by its id.
    fn translate_item_meta_from_rid(
        &mut self,
        instance: Instance,
    ) -> Result<CharonItemMeta, CharonError> {
        let span = self.translate_instance_span(instance);
        let name = self.def_to_name(instance.def)?;
        // TODO: populate the source text
        let source_text = None;
        // TODO: populate the attribute info
        let attr_info =
            CharonAttrInfo { attributes: Vec::new(), inline: None, rename: None, public: true };

        // Aeneas only translates items that are local to the top-level crate
        // Since we want all reachable items (including those in external
        // crates) to be translated, always set `is_local` to true
        let is_local = true;

        // For now, assume all items are transparent
        let opacity = CharonItemOpacity::Transparent;

        Ok(CharonItemMeta { span, source_text, attr_info, name, is_local, opacity })
    }

    fn translate_item_meta_from_defid(&mut self, defid: DefId) -> CharonItemMeta {
        let def_id = rustc_internal::internal(self.tcx(), defid);
        let span = self.translate_span(rustc_internal::stable(self.tcx.def_span(def_id)));
        let name = self.defid_to_name(defid).unwrap();
        // TODO: populate the source text
        let source_text = None;
        // TODO: populate the attribute info
        let attr_info =
            CharonAttrInfo { attributes: Vec::new(), inline: None, rename: None, public: true };

        // Aeneas only translates items that are local to the top-level crate
        // Since we want all reachable items (including those in external
        // crates) to be translated, always set `is_local` to true
        let is_local = true;

        // For now, assume all items are transparent
        let opacity = CharonItemOpacity::Transparent;

        CharonItemMeta { span, source_text, attr_info, name, is_local, opacity }
    }

    fn translate_item_meta_adt(&mut self, adt: AdtDef) -> Result<CharonItemMeta, CharonError> {
        let span = self.translate_span(adt.span());
        let name = self.adtdef_to_name(adt)?;
        // TODO: populate the source text
        let source_text = None;
        // TODO: populate the attribute info
        let attr_info =
            CharonAttrInfo { attributes: Vec::new(), inline: None, rename: None, public: true };

        // Aeneas only translates items that are local to the top-level crate
        // Since we want all reachable items (including those in external
        // crates) to be translated, always set `is_local` to true
        let is_local = true;

        // For now, assume all items are transparent
        let opacity = CharonItemOpacity::Transparent;

        Ok(CharonItemMeta { span, source_text, attr_info, name, is_local, opacity })
    }

    fn is_builtin_fun(&mut self, func_def: InstanceDef) -> bool {
        let name = self.def_to_name(func_def).unwrap();
        let crate_name = match name.name.first().unwrap() {
            CharonPathElem::Ident(cn, _) => cn,
            _ => panic!("Expected function name"),
        };
        crate_name.starts_with("std")
            || crate_name.starts_with("core")
            || crate_name.starts_with("alloc")
    }

    fn is_marker_trait(&mut self, traitdef: TraitDef) -> bool {
        let name = self.defid_to_name(traitdef.def_id()).unwrap();
        let crate_name = match name.name.first().unwrap() {
            CharonPathElem::Ident(cn, _) => cn,
            _ => panic!("Expected crate name"),
        };
        let marker = match name.name.get(1).unwrap() {
            CharonPathElem::Ident(cn, _) => cn,
            _ => panic!("Expected trait name"),
        };
        crate_name.starts_with("core") && marker.starts_with("marker")
    }

    fn defid_to_name(&mut self, defid: DefId) -> Result<CharonName, CharonError> {
        let tcx = self.tcx();
        let def_id = rustc_internal::internal(self.tcx(), defid);
        let span: CharonSpan = self.translate_span(rustc_internal::stable(tcx.def_span(def_id)));
        let mut found_crate_name = false;
        let mut name: Vec<CharonPathElem> = Vec::new();

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

        for cur_id in parents {
            let data = tcx.def_key(cur_id).disambiguated_data;
            // Match over the key data
            let disambiguator = CharonDisambiguator::new(data.disambiguator as usize);
            use rustc_hir::definitions::DefPathData;
            match &data.data {
                DefPathData::TypeNs(symbol) => {
                    error_assert!(self, span, data.disambiguator == 0); // Sanity check
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::ValueNs(symbol) => {
                    // I think `disambiguator != 0` only with names introduced by macros (though
                    // not sure).
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::CrateRoot => {
                    // Sanity check
                    error_assert!(self, span, data.disambiguator == 0);

                    // This should be the beginning of the path
                    error_assert!(self, span, name.is_empty());
                    found_crate_name = true;
                    name.push(CharonPathElem::Ident(crate_name.clone(), disambiguator));
                }
                DefPathData::Impl => {} //will check
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
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::Closure => {
                    // TODO: this is not very satisfactory, but on the other hand
                    // we should be able to extract closures in local let-bindings
                    // (i.e., we shouldn't have to introduce top-level let-bindings).
                    name.push(CharonPathElem::Ident("closure".to_string(), disambiguator))
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
            name.push(CharonPathElem::Ident(crate_name, CharonDisambiguator::new(0)));
        }

        trace!("{:?}", name);
        Ok(CharonName { name })
    }

    /// Retrieve an item name from a [DefId].
    /// This function is adapted from Charon:
    /// https://github.com/AeneasVerif/charon/blob/53530427db2941ce784201e64086766504bc5642/charon/src/bin/charon-driver/translate/translate_ctx.rs#L344
    fn def_to_name(&mut self, def: InstanceDef) -> Result<CharonName, CharonError> {
        let def_id = def.def_id();
        trace!("{:?}", def_id);
        let tcx = self.tcx();
        let span: CharonSpan = self.translate_span(def.span());
        let def_id = rustc_internal::internal(self.tcx(), def_id);

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
        // `[Ident("test"), Ident("bla"), Ident("Foo"), CharonDisambiguator(0), Ident("foo")]`
        // `[Ident("test"), Ident("bla"), Ident("Foo"), CharonDisambiguator(1), Ident("bar")]`
        let mut found_crate_name = false;
        let mut name: Vec<CharonPathElem> = Vec::new();

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
            let disambiguator = CharonDisambiguator::new(data.disambiguator as usize);
            use rustc_hir::definitions::DefPathData;
            match &data.data {
                DefPathData::TypeNs(symbol) => {
                    error_assert!(self, span, data.disambiguator == 0); // Sanity check
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::ValueNs(symbol) => {
                    // I think `disambiguator != 0` only with names introduced by macros (though
                    // not sure).
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::CrateRoot => {
                    // Sanity check
                    error_assert!(self, span, data.disambiguator == 0);

                    // This should be the beginning of the path
                    error_assert!(self, span, name.is_empty());
                    found_crate_name = true;
                    name.push(CharonPathElem::Ident(crate_name.clone(), disambiguator));
                }
                DefPathData::Impl => {} //will check
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
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::Closure => {
                    // TODO: this is not very satisfactory, but on the other hand
                    // we should be able to extract closures in local let-bindings
                    // (i.e., we shouldn't have to introduce top-level let-bindings).
                    name.push(CharonPathElem::Ident("closure".to_string(), disambiguator))
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
            name.push(CharonPathElem::Ident(crate_name, CharonDisambiguator::new(0)));
        }

        if let Some(impl_defid_internal) = self.tcx.impl_of_method(def_id) {
            let traitref = self
                .tcx
                .impl_trait_ref(impl_defid_internal)
                .unwrap()
                .skip_binder()
                .args
                .first()
                .unwrap()
                .to_string();
            let impl_defid = DefId::to_val(impl_defid_internal.index.as_usize());
            let _impl_id = self.register_trait_impl_id(impl_defid);
            let funcname = match name.pop().unwrap() {
                CharonPathElem::Ident(name, _) => name + traitref.as_str(),
                _ => panic!("Expected ident"),
            };
            name.push(CharonPathElem::Ident(funcname, CharonDisambiguator::new(0)));
        };
        trace!("{:?}", name);
        Ok(CharonName { name })
    }

    fn adtdef_to_name(&mut self, def: AdtDef) -> Result<CharonName, CharonError> {
        let def_id = def.def_id();
        trace!("{:?}", def_id);
        let tcx = self.tcx();
        let span: CharonSpan = self.translate_span(def.span());
        let def_id = rustc_internal::internal(self.tcx(), def_id);
        let mut found_crate_name = false;
        let mut name: Vec<CharonPathElem> = Vec::new();

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
            let disambiguator = CharonDisambiguator::new(data.disambiguator as usize);
            use rustc_hir::definitions::DefPathData;
            match &data.data {
                DefPathData::TypeNs(symbol) => {
                    error_assert!(self, span, data.disambiguator == 0); // Sanity check
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::ValueNs(symbol) => {
                    // I think `disambiguator != 0` only with names introduced by macros (though
                    // not sure).
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::CrateRoot => {
                    // Sanity check
                    error_assert!(self, span, data.disambiguator == 0);

                    // This should be the beginning of the path
                    error_assert!(self, span, name.is_empty());
                    found_crate_name = true;
                    name.push(CharonPathElem::Ident(crate_name.clone(), disambiguator));
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
                    name.push(CharonPathElem::Ident(symbol.to_string(), disambiguator));
                }
                DefPathData::Closure => {
                    // TODO: this is not very satisfactory, but on the other hand
                    // we should be able to extract closures in local let-bindings
                    // (i.e., we shouldn't have to introduce top-level let-bindings).
                    name.push(CharonPathElem::Ident("closure".to_string(), disambiguator))
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
            name.push(CharonPathElem::Ident(crate_name, CharonDisambiguator::new(0)));
        }

        trace!("{:?}", name);
        Ok(CharonName { name })
    }

    /// Compute the span information for the given instance
    fn translate_instance_span(&mut self, instance: Instance) -> CharonSpan {
        self.translate_span(instance.def.span())
    }

    /// Compute the span information for MIR span
    fn translate_span(&mut self, span: Span) -> CharonSpan {
        let filename = CharonFileName::Local(PathBuf::from(span.get_filename()));
        let file_id = match self.file_to_id.get(&filename) {
            Some(file_id) => *file_id,
            None => {
                let file = CharonFile { name: filename.clone(), contents: None };
                let file_id = self.translated.files.push(file);
                self.file_to_id.insert(filename, file_id);
                file_id
            }
        };
        let lineinfo = span.get_lines();
        let rspan = CharonRawSpan {
            file_id,
            beg: CharonLoc { line: lineinfo.start_line, col: lineinfo.start_col },
            end: CharonLoc { line: lineinfo.end_line, col: lineinfo.end_col },
        };

        // TODO: populate `generated_from_span` info
        CharonSpan { span: rspan, generated_from_span: None }
    }

    fn translate_function_signature(&mut self, instance: Instance) -> CharonFunSig {
        let fndef = match instance.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(fndef, _)) => fndef,
            _ => panic!("Expected a function type"),
        };
        let value = fndef.fn_sig().value;
        let inputs = value.inputs().to_vec();
        let c_genparam = self.generic_params_from_fndef(fndef, inputs.clone());
        let c_inputs: Vec<CharonTy> = inputs.iter().map(|ty| self.translate_ty(*ty)).collect();
        let c_output = self.translate_ty(value.output());
        // TODO: populate the rest of the information (`is_unsafe`, `is_closure`, etc.)
        CharonFunSig {
            is_unsafe: false,
            is_closure: false,
            closure_info: None,
            generics: c_genparam,
            inputs: c_inputs,
            output: c_output,
        }
    }

    fn translate_function_body(
        &mut self,
        instance: Instance,
    ) -> Result<CharonBodyId, CharonOpaque> {
        let fndef = match instance.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(fndef, _)) => fndef,
            _ => panic!("Expected a function type"),
        };
        let mir_body = fndef.body().unwrap();
        let body_id = self.translated.bodies.reserve_slot();
        let body = self.translate_body(mir_body);
        self.translated.bodies.set_slot(body_id, body);
        Ok(body_id)
    }

    fn translate_body(&mut self, mir_body: Body) -> CharonBody {
        let span = self.translate_span(mir_body.span);
        let arg_count = self.instance.fn_abi().unwrap().args.len();
        let vars = self.translate_body_locals(&mir_body);
        let locals = CharonLocals { vars, arg_count };
        let body: CharonBodyContents =
            mir_body.blocks.iter().map(|bb| self.translate_block(bb)).collect();

        let body_expr = CharonExprBody { span, locals, body, comments: Vec::new() };
        CharonBody::Unstructured(body_expr)
    }

    fn translate_generic_args(&mut self, ga: GenericArgs, defid: DefId) -> CharonGenericArgs {
        let genvec = ga.0;
        let mut c_regions: CharonVector<CharonRegionId, CharonRegion> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTy> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGeneric> =
            CharonVector::new();
        for genkind in genvec.iter() {
            let gk = genkind.clone();
            match gk {
                GenericArgKind::Lifetime(region) => {
                    let c_region = self.translate_region(region);
                    c_regions.push(c_region);
                }

                GenericArgKind::Type(ty) => {
                    let c_ty = self.translate_ty(ty);
                    c_types.push(c_ty);
                }
                GenericArgKind::Const(tc) => {
                    let c_const_generic = self.tyconst_to_constgeneric(tc);
                    c_const_generics.push(c_const_generic);
                } //_ => (),
            }
        }
        let (gen_trait_refs, spans) = self.get_traitrefs_and_span_from_defid(defid);
        let mut trait_refs: CharonVector<CharonTraitClauseId, CharonTraitRef> = CharonVector::new();
        let trait_ref_span_zip = zip(spans.clone(), gen_trait_refs.clone());
        for (_, trait_ref) in trait_ref_span_zip {
            let traitgenarg = trait_ref.trait_decl_ref.skip_binder.generics.clone();
            let t_regions: CharonVector<CharonRegionId, CharonRegion> = CharonVector::new();
            let mut t_types: CharonVector<CharonTypeVarId, CharonTy> = CharonVector::new();
            let t_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGeneric> =
                CharonVector::new();
            for tyvar in traitgenarg.types.iter() {
                match tyvar.kind() {
                    CharonTyKind::TypeVar(dbtyvarid) => {
                        let tyvarid = match dbtyvarid {
                            CharonDeBruijnVar::Free(tyvarid) => *tyvarid,
                            _ => panic!("Expect free type var id"),
                        };
                        let subs_ty = c_types.get(tyvarid).unwrap().clone();
                        t_types.push(subs_ty);
                    }
                    _ => todo!("TyKind of gen must be TyVar: {:?}", tyvar.kind()),
                }
            }
            let generics = CharonGenericArgs {
                regions: t_regions,
                types: t_types,
                const_generics: t_const_generics,
                trait_refs: trait_ref.trait_decl_ref.skip_binder.generics.trait_refs.clone(),
            };
            let traitdecl_id = trait_ref.trait_decl_ref.skip_binder.trait_id;
            let subs_traitdeclref = CharonPolyTraitDeclRef {
                regions: trait_ref.trait_decl_ref.regions.clone(),
                skip_binder: CharonTraitDeclRef {
                    trait_id: traitdecl_id,
                    generics: generics.clone(),
                },
            };
            let subs_traitref = CharonTraitRef {
                kind: CharonTraitRefKind::BuiltinOrAuto(subs_traitdeclref.clone()),
                trait_decl_ref: subs_traitdeclref,
            };
            trait_refs.push(subs_traitref);
        }
        CharonGenericArgs {
            regions: c_regions,
            types: c_types,
            const_generics: c_const_generics,
            trait_refs,
        }
    }

    fn translate_generic_args_without_trait(&mut self, ga: GenericArgs) -> CharonGenericArgs {
        let genvec = ga.0;
        let mut c_regions: CharonVector<CharonRegionId, CharonRegion> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTy> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGeneric> =
            CharonVector::new();
        for genkind in genvec.iter() {
            let gk = genkind.clone();
            match gk {
                GenericArgKind::Lifetime(region) => {
                    let c_region = self.translate_region(region);
                    c_regions.push(c_region);
                }
                GenericArgKind::Type(ty) => {
                    let c_ty = self.translate_ty(ty);
                    c_types.push(c_ty);
                }
                GenericArgKind::Const(tc) => {
                    let c_const_generic = self.tyconst_to_constgeneric(tc);
                    c_const_generics.push(c_const_generic);
                }
            }
        }
        CharonGenericArgs {
            regions: c_regions,
            types: c_types,
            const_generics: c_const_generics,
            trait_refs: CharonVector::new(),
        }
    }

    fn translate_ty(&mut self, ty: Ty) -> CharonTy {
        match ty.kind() {
            TyKind::RigidTy(rigid_ty) => self.translate_rigid_ty(rigid_ty),
            TyKind::Param(paramty) => {
                let debr = CharonDeBruijnVar::Bound(
                    CharonDeBruijnId::new(0),
                    CharonTypeVarId::from_usize(paramty.index as usize),
                );
                CharonTy::new(CharonTyKind::TypeVar(debr))
            }
            x => todo!("Not yet implemented translation for TyKind: {:?}", x),
        }
    }

    fn tyconst_to_constgeneric(&self, tyconst: TyConst) -> CharonConstGeneric {
        match tyconst.kind() {
            TyConstKind::Value(ty, alloc) => {
                let c_raw_constexpr = self.translate_allocation(alloc, *ty);
                translate_constant_expr_to_const_generic(c_raw_constexpr).unwrap()
            }
            TyConstKind::Param(paramc) => {
                let debr = CharonDeBruijnVar::Bound(
                    CharonDeBruijnId::new(0),
                    CharonConstGenericVarId::from_usize(paramc.index as usize),
                );
                CharonConstGeneric::Var(debr)
            }
            _ => todo!(),
        }
    }

    fn translate_rigid_ty(&mut self, rigid_ty: RigidTy) -> CharonTy {
        debug!("translate_rigid_ty: {rigid_ty:?}");
        match rigid_ty {
            RigidTy::Bool => CharonTy::new(CharonTyKind::Literal(CharonLiteralTy::Bool)),
            RigidTy::Char => CharonTy::new(CharonTyKind::Literal(CharonLiteralTy::Char)),
            RigidTy::Int(it) => {
                CharonTy::new(CharonTyKind::Literal(CharonLiteralTy::Integer(translate_int_ty(it))))
            }
            RigidTy::Uint(uit) => CharonTy::new(CharonTyKind::Literal(CharonLiteralTy::Integer(
                translate_uint_ty(uit),
            ))),
            RigidTy::Never => CharonTy::new(CharonTyKind::Never),
            RigidTy::Str => CharonTy::new(CharonTyKind::Adt(
                CharonTypeId::Builtin(CharonBuiltinTy::Str),
                // TODO: find out whether any of the information below should be
                // populated for strings
                CharonGenericArgs {
                    regions: CharonVector::new(),
                    types: CharonVector::new(),
                    const_generics: CharonVector::new(),
                    trait_refs: CharonVector::new(),
                },
            )),
            RigidTy::Array(ty, tyconst) => {
                let c_ty = self.translate_ty(ty);
                let c_const_generic = self.tyconst_to_constgeneric(tyconst);
                let mut c_types = CharonVector::new();
                let mut c_const_generics = CharonVector::new();
                c_types.push(c_ty);
                c_const_generics.push(c_const_generic);
                CharonTy::new(CharonTyKind::Adt(
                    CharonTypeId::Builtin(CharonBuiltinTy::Array),
                    CharonGenericArgs {
                        regions: CharonVector::new(),
                        types: c_types,
                        const_generics: c_const_generics,
                        trait_refs: CharonVector::new(),
                    },
                ))
            }
            RigidTy::Ref(region, ty, mutability) => CharonTy::new(CharonTyKind::Ref(
                self.translate_region(region),
                self.translate_ty(ty),
                match mutability {
                    Mutability::Mut => CharonRefKind::Mut,
                    Mutability::Not => CharonRefKind::Shared,
                },
            )),
            RigidTy::Tuple(ty) => {
                let types = ty.iter().map(|ty| self.translate_ty(*ty)).collect();
                // TODO: find out if any of the information below is needed
                let generic_args = CharonGenericArgs {
                    regions: CharonVector::new(),
                    types,
                    const_generics: CharonVector::new(),
                    trait_refs: CharonVector::new(),
                };
                CharonTy::new(CharonTyKind::Adt(CharonTypeId::Tuple, generic_args))
            }
            RigidTy::FnDef(def_id, _args) => {
                let sig = def_id.fn_sig().value;
                let inputs = sig.inputs().iter().map(|ty| self.translate_ty(*ty)).collect();
                let output = self.translate_ty(sig.output());
                // TODO: populate regions?
                let rb = CharonRegionBinder {
                    regions: CharonVector::new(),
                    skip_binder: (inputs, output),
                };
                CharonTy::new(CharonTyKind::Arrow(rb))
            }
            RigidTy::Adt(adt_def, genarg) => {
                let def_id = adt_def.def_id();
                let c_typedeclid = self.register_type_decl_id(def_id);
                if self.translated.type_decls.get(c_typedeclid).is_none() {
                    self.translate_adtdef(adt_def);
                }
                let c_generic_args = self.translate_generic_args(genarg, adt_def.def_id());
                CharonTy::new(CharonTyKind::Adt(CharonTypeId::Adt(c_typedeclid), c_generic_args))
            }
            RigidTy::Slice(ty) => {
                let c_ty = self.translate_ty(ty);
                let mut c_types = CharonVector::new();
                c_types.push(c_ty);
                CharonTy::new(CharonTyKind::Adt(
                    CharonTypeId::Builtin(CharonBuiltinTy::Slice),
                    CharonGenericArgs::new_from_types(c_types),
                ))
            }
            RigidTy::RawPtr(ty, mutability) => {
                let c_ty = self.translate_ty(ty);
                CharonTy::new(CharonTyKind::RawPtr(c_ty, match mutability {
                    Mutability::Mut => CharonRefKind::Mut,
                    Mutability::Not => CharonRefKind::Shared,
                }))
            }
            RigidTy::FnPtr(polyfunsig) => {
                let value = polyfunsig.value;
                let inputs = value.inputs().to_vec();
                let c_inputs: Vec<CharonTy> =
                    inputs.iter().map(|ty| self.translate_ty(*ty)).collect();
                let c_output = self.translate_ty(value.output());
                let rb = CharonRegionBinder {
                    regions: CharonVector::new(),
                    skip_binder: (c_inputs, c_output),
                };
                CharonTy::new(CharonTyKind::Arrow(rb))
            }
            RigidTy::Dynamic(_, _, _) => {
                CharonTy::new(CharonTyKind::DynTrait(CharonExistentialPredicate))
            }
            _ => todo!("Not yet implemented RigidTy: {:?}", rigid_ty),
        }
    }

    fn translate_body_locals(&mut self, mir_body: &Body) -> CharonVector<CharonVarId, CharonVar> {
        // Charon expects the locals in the following order:
        // - the local used for the return value (index 0)
        // - the input arguments
        // - the remaining locals, used for the intermediate computations
        let mut locals = CharonVector::new();
        mir_body.local_decls().for_each(|(local, local_decl)| {
            let ty = self.translate_ty(local_decl.ty);
            let name = self.local_names.get(&local);
            locals.push_with(|index| CharonVar { index, name: name.cloned(), ty });
        });
        locals
    }

    fn translate_block(&mut self, bb: &BasicBlock) -> CharonBlockData {
        let mut statements: Vec<CharonStatement> =
            bb.statements.iter().filter_map(|stmt| self.translate_statement(stmt)).collect();
        let (statement, terminator) = self.translate_terminator(&bb.terminator);
        if let Some(statement) = statement {
            statements.push(statement);
        }
        CharonBlockData { statements, terminator }
    }

    fn translate_statement(&mut self, stmt: &Statement) -> Option<CharonStatement> {
        let content = match &stmt.kind {
            StatementKind::Assign(place, rhs) => Some(CharonRawStatement::Assign(
                self.translate_place(&place),
                self.translate_rvalue(&rhs),
            )),
            StatementKind::SetDiscriminant { place, variant_index } => {
                Some(CharonRawStatement::SetDiscriminant(
                    self.translate_place(&place),
                    CharonVariantId::from_usize(variant_index.to_index()),
                ))
            }
            StatementKind::StorageLive(_) => None,
            StatementKind::StorageDead(local) => {
                Some(CharonRawStatement::StorageDead(CharonVarId::from_usize(*local)))
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

    fn translate_terminator(
        &mut self,
        terminator: &Terminator,
    ) -> (Option<CharonStatement>, CharonTerminator) {
        let span = self.translate_span(terminator.span);
        let (statement, terminator) = match &terminator.kind {
            TerminatorKind::Return => (None, CharonRawTerminator::Return),
            TerminatorKind::Goto { target } => {
                (None, CharonRawTerminator::Goto { target: CharonBlockId::from_usize(*target) })
            }
            TerminatorKind::Unreachable => {
                (None, CharonRawTerminator::Abort(CharonAbortKind::UndefinedBehavior))
            }
            TerminatorKind::Drop { place, target, .. } => (
                Some(CharonRawStatement::Drop(self.translate_place(&place))),
                CharonRawTerminator::Goto { target: CharonBlockId::from_usize(*target) },
            ),
            TerminatorKind::SwitchInt { discr, targets } => {
                let (discr, targets) = self.translate_switch_targets(discr, targets);
                (None, CharonRawTerminator::Switch { discr, targets })
            }
            TerminatorKind::Call { func, args, destination, target, .. } => {
                debug!("translate_call: {func:?} {args:?} {destination:?} {target:?}");
                let fn_ty = func.ty(self.instance.body().unwrap().locals()).unwrap();
                let fn_ptr = match fn_ty.kind() {
                    TyKind::RigidTy(RigidTy::FnDef(def, genarg)) => {
                        let instance = Instance::resolve(def, &genarg).unwrap();
                        let def_id = instance.def.def_id();
                        let fid = self.register_fun_decl_id(def_id);
                        let genarg_resolve = match instance.ty().kind() {
                            TyKind::RigidTy(RigidTy::FnDef(_, ga)) => ga,
                            _ => panic!("Expected a function type"),
                        };
                        let funcid = CharonFunIdOrTraitMethodRef::Fun(CharonFunId::Regular(fid));
                        let generics = self.translate_generic_args(genarg_resolve, def_id);
                        CharonFnPtr { func: funcid, generics }
                    }
                    TyKind::RigidTy(RigidTy::FnPtr(..)) => todo!(),
                    x => unreachable!(
                        "Function call where the function was of unexpected type: {:?}",
                        x
                    ),
                };
                let c_func_op = CharonFnOperand::Regular(fn_ptr);
                let call = CharonCall {
                    func: c_func_op,
                    args: args.iter().map(|arg| self.translate_operand(arg)).collect(),
                    dest: self.translate_place(destination),
                };
                (Some(CharonRawStatement::Call(call)), CharonRawTerminator::Goto {
                    target: CharonBlockId::from_usize(target.unwrap()),
                })
            }
            TerminatorKind::Assert { cond, expected, msg: _, target, .. } => (
                Some(CharonRawStatement::Assert(CharonAssert {
                    cond: self.translate_operand(cond),
                    expected: *expected,
                })),
                CharonRawTerminator::Goto { target: CharonBlockId::from_usize(*target) },
            ),
            _ => todo!(),
        };
        (
            statement.map(|statement| CharonStatement { span, content: statement }),
            CharonTerminator { span, content: terminator },
        )
    }

    fn translate_place(&mut self, place: &Place) -> CharonPlace {
        let projection = self.translate_projection(place, &place.projection);
        let local = place.local;
        let var_id = CharonVarId::from_usize(local);
        let basetype = self.translate_ty(self.place_ty(&place));
        let mut prjplace = CharonPlace::new(var_id, basetype);
        for (projelem, ty) in projection.iter() {
            prjplace = prjplace.project(projelem.clone(), ty.clone());
        }
        prjplace
    }

    fn place_ty(&self, place: &Place) -> Ty {
        let body = self.instance.body().unwrap();
        let ty = body.local_decl(place.local).unwrap().ty;
        ty
    }

    fn translate_rvalue(&mut self, rvalue: &Rvalue) -> CharonRvalue {
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
                CharonUnOp::Cast(self.translate_cast(*kind, operand, *ty)),
                self.translate_operand(operand),
            ),
            Rvalue::BinaryOp(bin_op, lhs, rhs) => CharonRvalue::BinaryOp(
                translate_bin_op(*bin_op),
                self.translate_operand(lhs),
                self.translate_operand(rhs),
            ),
            Rvalue::CheckedBinaryOp(bin_op, lhs, rhs) => CharonRvalue::BinaryOp(
                translate_bin_op(*bin_op),
                self.translate_operand(lhs),
                self.translate_operand(rhs),
            ),
            Rvalue::UnaryOp(op, operand) => {
                CharonRvalue::UnaryOp(translate_un_op(*op), self.translate_operand(operand))
            }
            Rvalue::Discriminant(place) => {
                let c_place = self.translate_place(place);
                let ty = self.place_ty(place);
                let c_ty = self.translate_ty(ty);
                match c_ty.kind() {
                    CharonTyKind::Adt(CharonTypeId::Adt(c_typedeclid), _) => {
                        CharonRvalue::Discriminant(c_place, *c_typedeclid)
                    }
                    _ => todo!("Not yet implemented:{:?}", c_ty.kind()),
                }
            }

            Rvalue::Aggregate(agg_kind, operands) => {
                let c_operands =
                    (*operands).iter().map(|operand| self.translate_operand(operand)).collect();
                let akind = agg_kind.clone();
                match akind {
                    AggregateKind::Adt(adt_def, variant_id, genarg, _user_anot, field_id) => {
                        let adt_kind = adt_def.kind();
                        match adt_kind {
                            AdtKind::Enum => {
                                let def_id = adt_def.def_id();
                                let c_typedeclid: CharonTypeDeclId = self.get_type_decl_id(def_id);
                                let c_type_id = CharonTypeId::Adt(c_typedeclid);
                                let c_variant_id =
                                    Some(CharonVariantId::from_usize(variant_id.to_index()));
                                let c_field_id = field_id.map(CharonFieldId::from_usize);
                                let c_generic_args =
                                    self.translate_generic_args(genarg, adt_def.def_id());
                                let c_agg_kind = CharonAggregateKind::Adt(
                                    c_type_id,
                                    c_variant_id,
                                    c_field_id,
                                    c_generic_args,
                                );
                                CharonRvalue::Aggregate(c_agg_kind, c_operands)
                            }
                            AdtKind::Struct => {
                                let def_id = adt_def.def_id();
                                let c_typedeclid: CharonTypeDeclId = self.get_type_decl_id(def_id);
                                let c_type_id = CharonTypeId::Adt(c_typedeclid);
                                let c_variant_id = None;
                                let c_field_id = None;
                                let c_generic_args =
                                    self.translate_generic_args(genarg, adt_def.def_id());
                                let c_agg_kind = CharonAggregateKind::Adt(
                                    c_type_id,
                                    c_variant_id,
                                    c_field_id,
                                    c_generic_args,
                                );
                                CharonRvalue::Aggregate(c_agg_kind, c_operands)
                            }
                            _ => todo!(),
                        }
                    }
                    AggregateKind::Tuple => CharonRvalue::Aggregate(
                        CharonAggregateKind::Adt(
                            CharonTypeId::Tuple,
                            None,
                            None,
                            CharonGenericArgs::empty(),
                        ),
                        c_operands,
                    ),
                    AggregateKind::Array(ty) => {
                        let c_ty = self.translate_ty(ty);
                        let cg = CharonConstGeneric::Value(CharonLiteral::Scalar(
                            CharonScalarValue::Usize(c_operands.len() as u64),
                        ));
                        CharonRvalue::Aggregate(CharonAggregateKind::Array(c_ty, cg), c_operands)
                    }
                    _ => todo!(),
                }
            }

            Rvalue::ShallowInitBox(_, _) => todo!(),
            Rvalue::CopyForDeref(_) => todo!(),
            Rvalue::ThreadLocalRef(_) => todo!(),
            _ => todo!(),
        }
    }

    fn translate_operand(&mut self, operand: &Operand) -> CharonOperand {
        trace!("translate_operand: {operand:?}");
        match operand {
            Operand::Constant(constant) => CharonOperand::Const(self.translate_constant(constant)),
            Operand::Copy(place) => CharonOperand::Copy(self.translate_place(&place)),
            Operand::Move(place) => CharonOperand::Move(self.translate_place(&place)),
        }
    }

    fn translate_constant(&mut self, constant: &ConstOperand) -> CharonConstantExpr {
        trace!("translate_constant: {constant:?}");
        let value = self.translate_constant_value(&constant.const_);
        CharonConstantExpr { value, ty: self.translate_ty(constant.ty()) }
    }

    fn translate_constant_value(&mut self, constant: &MirConst) -> CharonRawConstantExpr {
        trace!("translate_constant_value: {constant:?}");
        match constant.kind() {
            ConstantKind::Allocated(alloc) => self.translate_allocation(alloc, constant.ty()),
            ConstantKind::Ty(_) => todo!(),
            ConstantKind::ZeroSized => todo!(),
            ConstantKind::Unevaluated(uc) => {
                let defid = uc.def.def_id();
                let c_defid = self.register_global_decl_id(defid);
                let c_genarg = self.translate_generic_args(uc.args.clone(), defid);
                CharonRawConstantExpr::Global(CharonGlobalDeclRef {
                    id: c_defid,
                    generics: c_genarg,
                })
            }
            ConstantKind::Param(_) => todo!(),
        }
    }

    fn translate_allocation(&self, alloc: &Allocation, ty: Ty) -> CharonRawConstantExpr {
        match ty.kind() {
            TyKind::RigidTy(RigidTy::Int(it)) => {
                let value = alloc.read_int().unwrap();
                let scalar_value = match it {
                    IntTy::I8 => CharonScalarValue::I8(value as i8),
                    IntTy::I16 => CharonScalarValue::I16(value as i16),
                    IntTy::I32 => CharonScalarValue::I32(value as i32),
                    IntTy::I64 => CharonScalarValue::I64(value as i64),
                    IntTy::I128 => CharonScalarValue::I128(value),
                    IntTy::Isize => CharonScalarValue::Isize(value as i64),
                };
                CharonRawConstantExpr::Literal(CharonLiteral::Scalar(scalar_value))
            }
            TyKind::RigidTy(RigidTy::Uint(uit)) => {
                let value = alloc.read_uint().unwrap();
                let scalar_value = match uit {
                    UintTy::U8 => CharonScalarValue::U8(value as u8),
                    UintTy::U16 => CharonScalarValue::U16(value as u16),
                    UintTy::U32 => CharonScalarValue::U32(value as u32),
                    UintTy::U64 => CharonScalarValue::U64(value as u64),
                    UintTy::U128 => CharonScalarValue::U128(value),
                    UintTy::Usize => CharonScalarValue::Usize(value as u64),
                };
                CharonRawConstantExpr::Literal(CharonLiteral::Scalar(scalar_value))
            }
            TyKind::RigidTy(RigidTy::Bool) => {
                let value = alloc.read_bool().unwrap();
                CharonRawConstantExpr::Literal(CharonLiteral::Bool(value))
            }
            TyKind::RigidTy(RigidTy::Char) => {
                let value = char::from_u32(alloc.read_uint().unwrap() as u32);
                CharonRawConstantExpr::Literal(CharonLiteral::Char(value.unwrap()))
            }
            _ => todo!("Not yet implement {:?}, {:?}", ty, alloc),
        }
    }

    fn translate_cast(&self, _kind: CastKind, _operand: &Operand, _ty: Ty) -> CharonCastKind {
        todo!()
    }

    fn translate_switch_targets(
        &mut self,
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
            CharonSwitchTargets::If(
                CharonBlockId::from_usize(then_bb),
                CharonBlockId::from_usize(else_bb),
            )
        } else {
            let CharonTyKind::Literal(CharonLiteralTy::Integer(int_ty)) = charon_ty.kind() else {
                panic!("Expected integer type for switch discriminant");
            };
            let branches = targets
                .branches()
                .map(|(value, bb)| {
                    let scalar_val = match int_ty {
                        CharonIntegerTy::I8 => CharonScalarValue::I8(value as i8),
                        CharonIntegerTy::I16 => CharonScalarValue::I16(value as i16),
                        CharonIntegerTy::I32 => CharonScalarValue::I32(value as i32),
                        CharonIntegerTy::I64 => CharonScalarValue::I64(value as i64),
                        CharonIntegerTy::I128 => CharonScalarValue::I128(value as i128),
                        CharonIntegerTy::Isize => CharonScalarValue::Isize(value as i64),
                        CharonIntegerTy::U8 => CharonScalarValue::U8(value as u8),
                        CharonIntegerTy::U16 => CharonScalarValue::U16(value as u16),
                        CharonIntegerTy::U32 => CharonScalarValue::U32(value as u32),
                        CharonIntegerTy::U64 => CharonScalarValue::U64(value as u64),
                        CharonIntegerTy::U128 => CharonScalarValue::U128(value),
                        CharonIntegerTy::Usize => CharonScalarValue::Usize(value as u64),
                    };
                    (scalar_val, CharonBlockId::from_usize(bb))
                })
                .collect();
            let otherwise = CharonBlockId::from_usize(targets.otherwise());
            CharonSwitchTargets::SwitchInt(*int_ty, branches, otherwise)
        };
        (discr, switch_targets)
    }

    fn translate_projection(
        &mut self,
        place: &Place,
        projection: &[ProjectionElem],
    ) -> Vec<(CharonProjectionElem, CharonTy)> {
        let c_place_ty = self.translate_ty(self.place_ty(place));
        let mut c_provec = Vec::new();
        let mut current_ty = c_place_ty.clone();
        let mut current_var: usize = 0;
        for prj in projection.iter() {
            match prj {
                ProjectionElem::Deref => {
                    if let CharonTyKind::Ref(_, ty, _) = current_ty.kind() {
                        current_ty = ty.clone()
                    };
                    c_provec.push((CharonProjectionElem::Deref, current_ty.clone()))
                }
                ProjectionElem::Field(fid, ty) => {
                    let c_fieldid = CharonFieldId::from_usize(*fid);
                    let c_variantid = CharonVariantId::from_usize(current_var);
                    match current_ty.kind() {
                        CharonTyKind::Adt(CharonTypeId::Adt(tdid), _) => {
                            let adttype = self.translated.type_decls.get(*tdid).unwrap();
                            match adttype.kind {
                                CharonTypeDeclKind::Struct(_) => {
                                    let c_fprj = CharonFieldProjKind::Adt(*tdid, None);
                                    current_ty = self.translate_ty(*ty);
                                    c_provec.push((
                                        CharonProjectionElem::Field(c_fprj, c_fieldid),
                                        current_ty.clone(),
                                    ));
                                }
                                CharonTypeDeclKind::Enum(_) => {
                                    let c_fprj = CharonFieldProjKind::Adt(*tdid, Some(c_variantid));
                                    current_ty = self.translate_ty(*ty);
                                    c_provec.push((
                                        CharonProjectionElem::Field(c_fprj, c_fieldid),
                                        current_ty.clone(),
                                    ));
                                }
                                _ => (),
                            }
                        }
                        CharonTyKind::Adt(CharonTypeId::Tuple, genargs) => {
                            let c_fprj = CharonFieldProjKind::Tuple(genargs.types.len());
                            current_ty = self.translate_ty(*ty);
                            c_provec.push((
                                CharonProjectionElem::Field(c_fprj, c_fieldid),
                                current_ty.clone(),
                            ));
                        }
                        _ => (),
                    }
                }
                ProjectionElem::Downcast(varid) => {
                    current_var = varid.to_index();
                }
                ProjectionElem::Index(local) => {
                    let c_operand = CharonOperand::Copy(CharonPlace::new(
                        CharonVarId::from_usize(*local),
                        current_ty.clone(),
                    ));
                    c_provec.push((
                        CharonProjectionElem::Index {
                            offset: Box::new(c_operand),
                            from_end: false,
                        },
                        current_ty.clone(),
                    ));
                }

                _ => continue,
            }
        }
        c_provec
    }

    fn translate_region(&self, region: Region) -> CharonRegion {
        match region.kind {
            RegionKind::ReStatic => CharonRegion::Static,
            RegionKind::ReErased => CharonRegion::Erased,
            RegionKind::ReEarlyParam(epr) => {
                let debr = CharonDeBruijnVar::bound(
                    CharonDeBruijnId { index: 0_usize },
                    CharonRegionId::from_usize(epr.index as usize),
                );
                CharonRegion::Var(debr)
            }
            RegionKind::ReBound(var, boundregion) => {
                let debr = CharonDeBruijnVar::bound(
                    CharonDeBruijnId { index: var as usize },
                    CharonRegionId::from_usize(boundregion.var as usize),
                );
                CharonRegion::Var(debr)
            }
            RegionKind::RePlaceholder(_) => {
                todo!("Not yet implemented RegionKind: {:?}", region.kind)
            }
        }
    }
}

fn translate_int_ty(int_ty: IntTy) -> CharonIntegerTy {
    match int_ty {
        IntTy::I8 => CharonIntegerTy::I8,
        IntTy::I16 => CharonIntegerTy::I16,
        IntTy::I32 => CharonIntegerTy::I32,
        IntTy::I64 => CharonIntegerTy::I64,
        IntTy::I128 => CharonIntegerTy::I128,
        // TODO: assumes 64-bit platform
        IntTy::Isize => CharonIntegerTy::Isize,
    }
}

fn translate_uint_ty(uint_ty: UintTy) -> CharonIntegerTy {
    match uint_ty {
        UintTy::U8 => CharonIntegerTy::U8,
        UintTy::U16 => CharonIntegerTy::U16,
        UintTy::U32 => CharonIntegerTy::U32,
        UintTy::U64 => CharonIntegerTy::U64,
        UintTy::U128 => CharonIntegerTy::U128,
        // TODO: assumes 64-bit platform
        UintTy::Usize => CharonIntegerTy::Usize,
    }
}

fn translate_bin_op(bin_op: BinOp) -> CharonBinOp {
    match bin_op {
        BinOp::AddUnchecked => CharonBinOp::Add,
        BinOp::Add => CharonBinOp::CheckedAdd,
        BinOp::SubUnchecked => CharonBinOp::Sub,
        BinOp::Sub => CharonBinOp::CheckedSub,
        BinOp::MulUnchecked => CharonBinOp::Mul,
        BinOp::Mul => CharonBinOp::CheckedMul,
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

fn translate_un_op(un_op: UnOp) -> CharonUnOp {
    match un_op {
        UnOp::Not => CharonUnOp::Not,
        UnOp::Neg => CharonUnOp::Neg,
        UnOp::PtrMetadata => todo!(),
    }
}

fn translate_borrow_kind(kind: &BorrowKind) -> CharonBorrowKind {
    match kind {
        BorrowKind::Shared => CharonBorrowKind::Shared,
        BorrowKind::Mut { .. } => CharonBorrowKind::Mut,
        BorrowKind::Fake(_kind) => todo!(),
    }
}

fn translate_constant_expr_to_const_generic(
    value: CharonRawConstantExpr,
) -> Result<CharonConstGeneric, CharonError> {
    match value {
        CharonRawConstantExpr::Literal(v) => Ok(CharonConstGeneric::Value(v)),
        CharonRawConstantExpr::Var(v) => Ok(CharonConstGeneric::Var(v)),
        _ => todo!(),
    }
}
