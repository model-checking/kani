// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! This module contains a context for translating stable MIR into Charon's
//! unstructured low-level borrow calculus (ULLBC)

use charon_lib::ast::meta::{
    AttrInfo as CharonAttrInfo, Loc as CharonLoc, SpanData as CharonRawSpan,
};
use charon_lib::ast::types::{Ty as CharonTy, TyKind as CharonTyKind};
use charon_lib::ast::{
    Abi as CharonAbi, AbortKind as CharonAbortKind, AggregateKind as CharonAggregateKind,
    Assert as CharonAssert, BinOp as CharonBinOp, Body as CharonBody,
    BorrowKind as CharonBorrowKind, BuiltinAdt as CharonBuiltinAdt,
    BuiltinAssertKind as CharonBuiltinAssertKind, BuiltinImplData as CharonBuiltinImplData,
    BuiltinPathElem as CharonBuiltinPathElem, Call as CharonCall, CastKind as CharonCastKind,
    ConstGenericParam as CharonConstGenericVar, ConstGenericVarId as CharonConstGenericVarId,
    ConstantExpr as CharonConstantExpr, ConstantExprKind as CharonRawConstantExpr,
    DeBruijnId as CharonDeBruijnId, DeBruijnVar as CharonDeBruijnVar,
    Disambiguator as CharonDisambiguator, DropKind as CharonDropKind, Field as CharonField,
    FieldId as CharonFieldId, File as CharonFile, FileId as CharonFileId,
    FileName as CharonFileName, FloatTy as CharonFloatTy, FnOperand as CharonFnOperand,
    FnPtr as CharonFnPtr, FnPtrKind as CharonFunIdOrTraitMethodRef, FunDecl as CharonFunDecl,
    FunDeclId as CharonFunDeclId, FunSig as CharonFunSig, FunSource as CharonFunSource,
    GenericArgs as CharonGenericArgs, GenericParams as CharonGenericParams,
    GlobalDeclId as CharonGlobalDeclId, GlobalDeclRef as CharonGlobalDeclRef, IntTy as CharonIntTy,
    IntegerTy as CharonIntegerTy, IntegerValue as CharonScalarValue, ItemId as CharonAnyTransId,
    ItemMeta as CharonItemMeta, ItemOpacity as CharonItemOpacity,
    LifetimeMutability as CharonLifetimeMutability, Local as CharonVar, LocalId as CharonVarId,
    Locals as CharonLocals, Name as CharonName, Operand as CharonOperand,
    OverflowMode as CharonOverflowMode, PathElem as CharonPathElem, Place as CharonPlace,
    PolyTraitDeclRef as CharonPolyTraitDeclRef, PredicateOrigin as CharonPredicateOrigin,
    ProjectionElem as CharonProjectionElem, PtrMetadata as CharonPtrMetadata,
    RefKind as CharonRefKind, Region as CharonRegion, RegionBinder as CharonRegionBinder,
    RegionId as CharonRegionId, RegionParam as CharonRegionVar, Rvalue as CharonRvalue,
    ScalarTy as CharonLiteralTy, Span as CharonSpan, SwitchData as CharonSwitchData,
    SwitchScrutinee as CharonSwitchScrutinee, TargetInfo as CharonTargetInfo,
    TraitClauseId as CharonTraitClauseId, TraitDecl as CharonTraitDecl,
    TraitDeclId as CharonTraitDeclId, TraitDeclRef as CharonTraitDeclRef,
    TraitDeclSource as CharonTraitDeclSource, TraitImplId as CharonTraitImplId,
    TraitParam as CharonTraitClause, TraitRef as CharonTraitRef,
    TraitRefKind as CharonTraitRefKind, TranslatedCrate as CharonTranslatedCrate,
    TypeDecl as CharonTypeDecl, TypeDeclId as CharonTypeDeclId, TypeDeclKind as CharonTypeDeclKind,
    TypeDeclRef as CharonTypeDeclRef, TypeParam as CharonTypeVar, TypeSource as CharonTypeSource,
    TypeVarId as CharonTypeVarId, UIntTy as CharonUIntTy, UnOp as CharonUnOp,
    Variance as CharonVariance, Variant as CharonVariant, VariantId as CharonVariantId,
    WithRetag as CharonWithRetag,
};
use charon_lib::errors::{Error as CharonError, ErrorCtx as CharonErrorCtx, Level as CharonLevel};
use charon_lib::ids::IndexVec as CharonVector;
use charon_lib::ullbc_ast::{
    BlockData as CharonBlockData, BlockId as CharonBlockId, BodyContents as CharonBodyContents,
    BranchId as CharonBranchId, ExprBody as CharonExprBody, Statement as CharonStatement,
    StatementKind as CharonRawStatement, Terminator as CharonTerminator,
    TerminatorKind as CharonRawTerminator,
};
use charon_lib::{error_assert, raise_error, register_error};
use core::panic;
use indexmap::IndexMap;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::{TyCtxt, TypingEnv};
use rustc_public::mir::mono::{Instance, InstanceDef};
use rustc_public::mir::{
    AggregateKind, AssertMessage, BasicBlock, BinOp, Body, BorrowKind, CastKind, ConstOperand,
    Local, Mutability, Operand, Place, ProjectionElem, Rvalue, Statement, StatementKind,
    SwitchTargets, Terminator, TerminatorKind, UnOp, VarDebugInfoContents,
};
use rustc_public::rustc_internal;
use rustc_public::ty::{
    AdtDef, AdtKind, Allocation, BoundRegionKind, BoundVariableKind, ConstantKind, FieldDef, FnDef,
    GenericArgKind, GenericArgs, GenericParamDefKind, IntTy, MirConst, PolyFnSig, Region,
    RegionKind, RigidTy, Span, TraitDecl, TraitDef, Ty, TyConst, TyConstKind, TyKind, UintTy,
    VariantIdx,
};
use rustc_public::{CrateDef, CrateDefType, DefId};
use rustc_public_bridge::IndexedVal;
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
    /// Block ID of the synthetic block that aborts. It is the target of every call's unwind edge
    /// (Kani does not model unwinding) and of the return edge of calls that never return.
    abort_block: CharonBlockId,
    /// How Charon numbers the generic parameters of the item whose declaration is being
    /// translated (see [`ItemGenerics`]); `None` outside of declarations.
    item_generics: Option<ItemGenerics>,
    /// The number of binders (`for<..>` of function-pointer types) entered since the item's
    /// own binder, i.e. the De Bruijn index of the item's generic parameters.
    binder_depth: usize,
}

/// How Charon numbers the generic parameters of a type or function declaration: per kind
/// (regions, types, const generics), parent generics first, and a function's late-bound regions
/// after its early-bound ones. rustc instead numbers early-bound parameters across all kinds.
#[derive(Clone, Default)]
struct ItemGenerics {
    /// rustc's parameter index -> position among the parameters of the same kind.
    positions: FxHashMap<u32, usize>,
    /// The number of early-bound region parameters.
    early_regions: usize,
    /// Whether the item is a function, whose signature binds late-bound regions.
    binds_late_regions: bool,
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
        let abort_block = CharonBlockId::from_usize(0);
        Self {
            tcx,
            instance,
            translated,
            id_map,
            errors,
            local_names,
            file_to_id,
            abort_block,
            item_generics: None,
            binder_depth: 0,
        }
    }

    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    /// Charon's numbering of the generic parameters of `def_id` (see [`ItemGenerics`]).
    fn item_generics(&self, def_id: DefId, binds_late_regions: bool) -> ItemGenerics {
        let mut chain = Vec::new();
        let mut next = Some(rustc_internal::internal(self.tcx, def_id));
        while let Some(def_id) = next {
            let generics = self.tcx.generics_of(def_id);
            chain.push(generics);
            next = generics.parent;
        }
        let mut positions = FxHashMap::default();
        let (mut regions, mut types, mut consts) = (0, 0, 0);
        for param in chain.iter().rev().flat_map(|generics| generics.own_params.iter()) {
            let counter = match param.kind {
                rustc_middle::ty::GenericParamDefKind::Lifetime => &mut regions,
                rustc_middle::ty::GenericParamDefKind::Type { .. } => &mut types,
                rustc_middle::ty::GenericParamDefKind::Const { .. } => &mut consts,
            };
            positions.insert(param.index, *counter);
            *counter += 1;
        }
        ItemGenerics { positions, early_regions: regions, binds_late_regions }
    }

    /// Run `f` with the generic parameters of `def_id` in scope, as for translating its
    /// declaration. Declarations nest (a field's type may need its own declaration), so the
    /// enclosing scope is restored afterwards.
    fn with_item_generics<T>(
        &mut self,
        def_id: DefId,
        binds_late_regions: bool,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let generics = self.item_generics(def_id, binds_late_regions);
        let outer_generics = self.item_generics.replace(generics);
        let outer_depth = std::mem::replace(&mut self.binder_depth, 0);
        let result = f(self);
        self.item_generics = outer_generics;
        self.binder_depth = outer_depth;
        result
    }

    /// The signature of `fndef` as declared, with its early-bound regions as parameters.
    /// `FnDef::fn_sig` erases those, which would lose e.g. the `'a` in
    /// `fn f<'a, T: 'a>(x: &'a T)`.
    fn declared_fn_sig(&self, fndef: FnDef) -> PolyFnSig {
        let def_id = rustc_internal::internal(self.tcx, fndef.def_id());
        rustc_internal::stable(self.tcx.fn_sig(def_id).instantiate_identity().skip_normalization())
    }

    /// Charon's position of the early-bound parameter with rustc index `index`.
    fn param_position(&self, index: u32) -> usize {
        self.item_generics
            .as_ref()
            .and_then(|generics| generics.positions.get(&index).copied())
            .unwrap_or(index as usize)
    }

    fn span_err(&mut self, span: CharonSpan, msg: &str, level: CharonLevel) -> CharonError {
        self.errors.span_err(self.translated, span, msg, level)
    }

    fn translate_traitdecl(&mut self, trait_def: TraitDef) -> CharonTraitDeclId {
        let trait_def_id = trait_def.def_id();
        let trait_decl_id = self.register_trait_decl_id(trait_def_id);
        match self.translated.trait_decls.get(trait_decl_id) {
            None => {
                let trait_decl = TraitDef::declaration(&trait_def);
                // As before, Kani declares the trait with its generics only: no implied
                // clauses, associated items or methods.
                let c_traitdecl = CharonTraitDecl {
                    def_id: trait_decl_id,
                    item_meta: self.translate_item_meta_from_defid(trait_def_id),
                    src: CharonTraitDeclSource::Normal,
                    generics: self.generic_params_from_traitdecl(trait_decl),
                    implied_clauses: CharonVector::new(),
                    consts: Default::default(),
                    types: Default::default(),
                    methods: Default::default(),
                    vtable: None,
                };
                self.translated.trait_decls.set_slot(trait_decl_id, c_traitdecl);
                trait_decl_id
            }
            Some(_) => trait_decl_id,
        }
    }

    //This function extract the traitrefs and their span from a def_id
    //Those information will be added into generic args of the type or the func with the def_id
    //Note that Generic args of Charon contains trait_refs while those of rustc_public do not
    fn get_traitrefs_and_span_from_defid(
        &mut self,
        defid: DefId,
    ) -> (CharonVector<CharonTraitClauseId, CharonTraitRef>, Vec<CharonSpan>) {
        let inter_defid = rustc_internal::internal(self.tcx, defid);
        let predicates = self.tcx().clauses_of(inter_defid).clauses.to_vec();
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
                    id: c_traitdecl_id,
                    generics: Box::new(c_genarg.clone()),
                },
            };
            let debr = CharonDeBruijnVar::free(CharonTraitClauseId::from_usize(i));
            let c_traitref = CharonTraitRef::new(CharonTraitRefKind::Clause(debr), c_polytrait);
            c_trait_refs.push(c_traitref);
            c_spans.push(self.translate_span(rustc_internal::stable(span)));
        }
        (c_trait_refs, c_spans)
    }

    //Get the trait clauses of an Adt or a Func from their def_id
    //Those information will be added into GenericParams of the Type Decl or Func Decl
    fn get_traitclauses_from_defid(
        &mut self,
        defid: DefId,
    ) -> CharonVector<CharonTraitClauseId, CharonTraitClause> {
        let inter_defid = rustc_internal::internal(self.tcx, defid);
        let predicates = self.tcx().clauses_of(inter_defid).clauses.to_vec();
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
                skip_binder: CharonTraitDeclRef {
                    id: c_traitdecl_id,
                    generics: Box::new(c_genarg),
                },
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
        let (generics, signature) = self.translate_function_signature(self.instance);
        //We temporarily don't translate the body of built-in function
        //because at the current step, we want to extend the amount of syntaxes
        //and test each syntax we extended (in tests/expected/llbc).
        //Enabling translation of dependent built-in functions now may make the
        //translation of the tests fail because of not-yet-implemented syntaxes
        //Example: tests/expected/llbc/option test fails because of the function std::ptr::drop_in_place
        let body = if is_builtin {
            CharonBody::Opaque
        } else {
            match self.translate_function_body(self.instance) {
                Ok(body) => body,
                Err(_) => {
                    return Err(());
                }
            }
        };
        let fun_decl = CharonFunDecl {
            def_id: fid,
            item_meta,
            generics,
            signature: Box::new(signature),
            src: CharonFunSource::Normal,
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
                tid
            }
        };
        debug!("register_global_decl_id: {:?}", self.id_map);
        tid.try_into().unwrap()
    }

    // similar to register_type_decl_id, but not adding new def_id, used for cases where the def_id has been registered, or in functions that take immut &self
    //This function is implemented according to how Charon encodes discriminants
    fn get_discriminant(&mut self, discr_val: u128, ty: Ty) -> CharonScalarValue {
        let ty = self.translate_ty(ty);
        let int_ty = *ty.kind().as_scalar().unwrap().as_integer().unwrap();
        CharonScalarValue::from_bits(int_ty, discr_val)
    }

    //Get the GenericParams for Trait Decl, which is neccessary in Trait Decl translation
    fn generic_params_from_traitdecl(&mut self, traitdecl: TraitDecl) -> CharonGenericParams {
        self.with_item_generics(traitdecl.def_id.def_id(), false, |this| {
            this.generic_params_from_traitdecl_in_scope(traitdecl)
        })
    }

    fn generic_params_from_traitdecl_in_scope(
        &mut self,
        traitdecl: TraitDecl,
    ) -> CharonGenericParams {
        let genvec = traitdecl.generics_of().params;
        let mut c_regions: CharonVector<CharonRegionId, CharonRegionVar> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTypeVar> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstGenericVar> =
            CharonVector::new();
        for gendef in genvec.iter() {
            let genkind = gendef.kind.clone();
            let index = self.param_position(gendef.index);
            let name = gendef.name.clone();
            match genkind {
                GenericParamDefKind::Lifetime => {
                    let c_region = CharonRegionVar {
                        index: CharonRegionId::from_usize(index),
                        name: Some(name),
                        variance: CharonVariance::Unknown,
                        mutability: CharonLifetimeMutability::Unknown,
                    };
                    c_regions.push(c_region);
                }
                GenericParamDefKind::Type { has_default: _, synthetic: _ } => {
                    let c_region = CharonTypeVar {
                        index: CharonTypeVarId::from_usize(index),
                        name,
                        variance: CharonVariance::Unknown,
                    };
                    c_types.push(c_region);
                }
                GenericParamDefKind::Const { has_default: _ } => {
                    let def_id_internal = rustc_internal::internal(self.tcx, gendef.def_id.0);
                    let pc_internal = rustc_middle::ty::ParamConst {
                        index: gendef.index,
                        name: rustc_span::Symbol::intern(&name.clone()),
                    };
                    let paramenv = TypingEnv::post_analysis(self.tcx, def_id_internal).param_env;
                    let ty_internal = pc_internal.find_const_ty_from_env(paramenv);
                    let ty_stable = rustc_internal::stable(ty_internal);
                    let trans_ty = self.translate_ty(ty_stable);
                    let c_constgeneric = CharonConstGenericVar {
                        index: CharonConstGenericVarId::from_usize(index),
                        name,
                        ty: trans_ty,
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
            trait_type_constraints: CharonVector::new(),
        }
    }

    //Get the GenericParams for Func Decl, which is neccessary in Func Decl translation
    fn generic_params_from_fndef(&mut self, fndef: FnDef, sig: &PolyFnSig) -> CharonGenericParams {
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
                            index: CharonRegionId::from_usize(self.param_position(epr.index)),
                            name: Some(epr.name),
                            variance: CharonVariance::Unknown,
                            mutability: CharonLifetimeMutability::Unknown,
                        };
                        c_regions.push(c_region);
                    }
                    _ => panic!("generic_params_from_adtdef: not an early bound region"),
                },
                GenericArgKind::Type(ty) => match ty.kind() {
                    TyKind::Param(paramty) => {
                        let c_typevar = CharonTypeVar {
                            index: CharonTypeVarId::from_usize(self.param_position(paramty.index)),
                            name: paramty.name,
                            variance: CharonVariance::Unknown,
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
                        let ty_internal = pc_internal.find_const_ty_from_env(paramenv);
                        let ty_stable = rustc_internal::stable(ty_internal);
                        let trans_ty = self.translate_ty(ty_stable);
                        let c_constgeneric = CharonConstGenericVar {
                            index: CharonConstGenericVarId::from_usize(
                                self.param_position(paramtc.index),
                            ),
                            name: paramtc.name.clone(),
                            ty: trans_ty,
                        };
                        c_const_generics.push(c_constgeneric);
                    }
                    _ => panic!("generic_params_from_fndef: not a param const"),
                },
            }
        }
        // The signature's late-bound regions, numbered after the early-bound ones, as Charon
        // does. They are all in the binder, wherever in the signature they occur.
        for name in late_bound_regions(sig) {
            c_regions.push_with(|index| CharonRegionVar {
                index,
                name,
                variance: CharonVariance::Unknown,
                mutability: CharonLifetimeMutability::Unknown,
            });
        }
        let trait_clauses = self.get_traitclauses_from_defid(fndef.def_id());
        CharonGenericParams {
            regions: c_regions,
            types: c_types,
            const_generics: c_const_generics,
            trait_clauses,
            regions_outlive: Vec::new(),
            types_outlive: Vec::new(),
            trait_type_constraints: CharonVector::new(),
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
                            index: CharonRegionId::from_usize(self.param_position(epr.index)),
                            name: Some(epr.name),
                            variance: CharonVariance::Unknown,
                            mutability: CharonLifetimeMutability::Unknown,
                        };
                        c_regions.push(c_region);
                    }
                    _ => panic!("generic_params_from_adtdef: not an early bound region"),
                },
                GenericArgKind::Type(ty) => match ty.kind() {
                    TyKind::Param(paramty) => {
                        let c_typevar = CharonTypeVar {
                            index: CharonTypeVarId::from_usize(self.param_position(paramty.index)),
                            name: paramty.name,
                            variance: CharonVariance::Unknown,
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
                        let ty_internal = pc_internal.find_const_ty_from_env(paramenv);
                        let ty_stable = rustc_internal::stable(ty_internal);
                        let trans_ty = self.translate_ty(ty_stable);
                        let c_constgeneric = CharonConstGenericVar {
                            index: CharonConstGenericVarId::from_usize(
                                self.param_position(paramtc.index),
                            ),
                            name: paramtc.name.clone(),
                            ty: trans_ty,
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
            trait_type_constraints: CharonVector::new(),
        }
    }

    fn translate_adtdef(&mut self, adt_def: AdtDef) -> CharonTypeDecl {
        self.with_item_generics(adt_def.def_id(), false, |this| {
            this.translate_adtdef_in_scope(adt_def)
        })
    }

    fn translate_adtdef_in_scope(&mut self, adt_def: AdtDef) -> CharonTypeDecl {
        let def_id = adt_def.def_id();
        let c_typedeclid = self.register_type_decl_id(def_id);
        let generics = self.generic_params_from_adtdef(adt_def);
        let item_meta = self.translate_item_meta_adt(adt_def).unwrap();
        let kind = match adt_def.kind() {
            AdtKind::Enum => {
                let mut c_variants: CharonVector<CharonVariantId, CharonVariant> =
                    CharonVector::new();
                for (var_idx, var_def) in adt_def.variants_iter().enumerate() {
                    // `variants_iter` yields variants in source-declaration order,
                    // so the enumeration index is the variant's `VariantIdx`.
                    // `VariantDef::idx` is no longer publicly accessible.
                    let variant_idx = VariantIdx::to_val(var_idx);
                    let fields = self.translate_fields(adt_def, var_def.fields());
                    let span = self.translate_span(adt_def.span());

                    let adtdef_internal = rustc_internal::internal(self.tcx, adt_def);
                    let variant_index_internal = rustc_internal::internal(self.tcx, variant_idx);
                    let discr =
                        adtdef_internal.discriminant_for_variant(self.tcx, variant_index_internal);
                    let discr_val = discr.val;
                    let discr_ty = rustc_internal::stable(discr.ty);
                    let c_discr = self.get_discriminant(discr_val, discr_ty);

                    let c_varidx = c_variants.push_with(|id| CharonVariant {
                        id,
                        span,
                        attr_info: default_attr_info(),
                        name: var_def.name(),
                        fields,
                        discriminant: c_discr,
                    });
                    assert_eq!(c_varidx.index(), var_idx);
                }
                CharonTypeDeclKind::Enum(c_variants)
            }
            AdtKind::Struct => {
                let only_variant = *adt_def.variants().first().unwrap();
                CharonTypeDeclKind::Struct(self.translate_fields(adt_def, only_variant.fields()))
            }
            _ => todo!(),
        };
        let typedecl = CharonTypeDecl {
            def_id: c_typedeclid,
            item_meta,
            generics,
            src: CharonTypeSource::Normal,
            kind,
            layout: Default::default(),
            // Kani only translates sized ADTs.
            ptr_metadata: CharonPtrMetadata::None,
        };
        self.translated.type_decls.set_slot(c_typedeclid, typedecl.clone());
        typedecl
    }

    fn translate_fields(
        &mut self,
        adt_def: AdtDef,
        fields: Vec<FieldDef>,
    ) -> CharonVector<CharonFieldId, CharonField> {
        let mut c_fields: CharonVector<CharonFieldId, CharonField> = CharonVector::new();
        for field_def in fields {
            let ty = self.translate_ty(field_def.ty());
            let span = self.translate_span(adt_def.span());
            // Tuple-struct and tuple-variant fields are named by their position, which Charon
            // spells `_0`, `_1`, ...
            let is_positional = field_def.name.chars().all(|c| c.is_ascii_digit());
            let name = if is_positional { format!("_{}", field_def.name) } else { field_def.name };
            c_fields.push(CharonField {
                span,
                attr_info: default_attr_info(),
                name,
                is_positional,
                ty,
            });
        }
        c_fields
    }

    /// The type declaration of `str`. Charon declares it as the builtin struct `str { _0: [u8] }`
    /// (upstream synthesizes the same item), rather than as a builtin type without a declaration.
    fn str_type_decl_ref(&mut self) -> CharonTypeDeclRef {
        // One declaration per crate: look for it, since a `Context` only lives for one function.
        let existing = self
            .translated
            .type_decls
            .iter()
            .find(|decl| matches!(decl.src, CharonTypeSource::Builtin(CharonBuiltinAdt::Str)))
            .map(|decl| decl.def_id);
        let id = match existing {
            Some(id) => id,
            None => {
                let id = self.translated.type_decls.reserve_slot();
                let span = CharonSpan::dummy();
                let name = CharonName {
                    name: vec![CharonPathElem::Builtin(
                        CharonBuiltinPathElem::Str,
                        CharonDisambiguator::ZERO,
                    )],
                };
                let u8_ty = CharonTy::new(CharonTyKind::Scalar(CharonLiteralTy::Integer(
                    CharonIntegerTy::Unsigned(CharonUIntTy::U8),
                )));
                let mut fields = CharonVector::new();
                fields.push(CharonField {
                    span,
                    attr_info: default_attr_info(),
                    name: "_0".to_owned(),
                    is_positional: true,
                    // No `Sized` proof, as in `translate_rigid_ty`.
                    ty: CharonTy::mk_slice(u8_ty, None),
                });
                let decl = CharonTypeDecl {
                    def_id: id,
                    item_meta: item_meta(span, name),
                    generics: CharonGenericParams::empty(),
                    src: CharonTypeSource::Builtin(CharonBuiltinAdt::Str),
                    kind: CharonTypeDeclKind::Struct(fields),
                    layout: Default::default(),
                    ptr_metadata: CharonPtrMetadata::Length,
                };
                self.translated.type_decls.set_slot(id, decl);
                id
            }
        };
        CharonTypeDeclRef {
            id,
            generics: Box::new(CharonGenericArgs::empty()),
            builtin: Some(CharonBuiltinAdt::Str),
        }
    }

    /// Compute the meta information for a Rust item identified by its id.
    fn translate_item_meta_from_rid(
        &mut self,
        instance: Instance,
    ) -> Result<CharonItemMeta, CharonError> {
        let span = self.translate_instance_span(instance);
        let name = self.def_to_name(instance.def)?;
        Ok(item_meta(span, name))
    }

    fn translate_item_meta_from_defid(&mut self, defid: DefId) -> CharonItemMeta {
        let def_id = rustc_internal::internal(self.tcx(), defid);
        let span = self.translate_span(rustc_internal::stable(self.tcx.def_span(def_id)));
        let name = self.defid_to_name(defid).unwrap();
        item_meta(span, name)
    }

    fn translate_item_meta_adt(&mut self, adt: AdtDef) -> Result<CharonItemMeta, CharonError> {
        let span = self.translate_span(adt.span());
        let name = self.adtdef_to_name(adt)?;
        Ok(item_meta(span, name))
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

    /// Retrieve an item name from a [DefId].
    /// This function is adapted from Charon:
    /// https://github.com/AeneasVerif/charon/blob/53530427db2941ce784201e64086766504bc5642/charon/src/bin/charon-driver/translate/translate_ctx.rs#L344
    fn defid_to_name(&mut self, defid: DefId) -> Result<CharonName, CharonError> {
        let tcx = self.tcx();
        let def_id = rustc_internal::internal(self.tcx(), defid);
        let span: CharonSpan = self.translate_span(rustc_internal::stable(tcx.def_span(def_id)));

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
                    raise_error!(self, span, "Unexpected DefPathData: {:?}", data);
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

    /// The name of a function instance: its path, with the method name suffixed by the
    /// implementing type when it is defined in an `impl` block.
    fn def_to_name(&mut self, def: InstanceDef) -> Result<CharonName, CharonError> {
        let mut name = self.defid_to_name(def.def_id())?;
        let def_id = rustc_internal::internal(self.tcx(), def.def_id());
        if let Some(impl_defid_internal) = self.tcx.impl_of_assoc(def_id) {
            // `{impl}` path elements are skipped, so methods of different impls share a path
            // (`core::num::wrapping_add` for every integer type); tell them apart by the
            // implementing type. That is the impl's self type for inherent and trait impls
            // alike -- only trait impls have a trait ref, and asking an inherent impl for one
            // aborted the compiler on any inherent method call.
            let self_ty = self.tcx.type_of(impl_defid_internal).skip_binder().to_string();
            if self.tcx.impl_is_of_trait(impl_defid_internal) {
                let impl_defid = DefId::to_val(impl_defid_internal.index.as_usize());
                let _impl_id = self.register_trait_impl_id(impl_defid);
            }
            let funcname = match name.name.pop().unwrap() {
                CharonPathElem::Ident(name, _) => name + self_ty.as_str(),
                _ => panic!("Expected ident"),
            };
            name.name.push(CharonPathElem::Ident(funcname, CharonDisambiguator::new(0)));
        };
        trace!("{:?}", name);
        Ok(name)
    }

    fn adtdef_to_name(&mut self, def: AdtDef) -> Result<CharonName, CharonError> {
        self.defid_to_name(def.def_id())
    }

    fn translate_instance_span(&mut self, instance: Instance) -> CharonSpan {
        self.translate_span(instance.def.span())
    }

    /// Compute the span information for MIR span
    fn translate_span(&mut self, span: Span) -> CharonSpan {
        let filename = CharonFileName::Local(PathBuf::from(span.get_filename()));
        let file_id = match self.file_to_id.get(&filename) {
            Some(file_id) => *file_id,
            None => {
                let crate_name = self.translated.crate_name.clone();
                let file_id = self.translated.files.push_with(|id| CharonFile {
                    id,
                    name: filename.clone(),
                    crate_name,
                    contents: None,
                });
                self.file_to_id.insert(filename, file_id);
                file_id
            }
        };
        let lineinfo = span.get_lines();
        let rspan = CharonRawSpan {
            file_id,
            beg: CharonLoc { line: loc(lineinfo.start_line), col: loc(lineinfo.start_col) },
            end: CharonLoc { line: loc(lineinfo.end_line), col: loc(lineinfo.end_col) },
        };

        // TODO: populate `generated_from_span` info
        CharonSpan::new(rspan, None)
    }

    /// The generics and the signature of `instance`: Charon keeps the generics on the `FunDecl`.
    fn translate_function_signature(
        &mut self,
        instance: Instance,
    ) -> (CharonGenericParams, CharonFunSig) {
        let fndef = match instance.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(fndef, _)) => fndef,
            _ => panic!("Expected a function type"),
        };
        let sig = self.declared_fn_sig(fndef);
        let value = sig.value.clone();
        let (c_genparam, c_inputs, c_output) =
            self.with_item_generics(fndef.def_id(), true, |this| {
                let c_genparam = this.generic_params_from_fndef(fndef, &sig);
                let c_inputs: Vec<CharonTy> =
                    value.inputs().iter().map(|ty| this.translate_ty(*ty)).collect();
                let c_output = this.translate_ty(value.output());
                (c_genparam, c_inputs, c_output)
            });
        // TODO: populate the rest of the information (`is_unsafe`, `abi`, etc.)
        let sig = CharonFunSig {
            is_unsafe: false,
            abi: CharonAbi::Rust,
            is_variadic: false,
            inputs: c_inputs,
            output: c_output,
        };
        (c_genparam, sig)
    }

    fn translate_function_body(&mut self, instance: Instance) -> Result<CharonBody, ()> {
        let fndef = match instance.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(fndef, _)) => fndef,
            _ => panic!("Expected a function type"),
        };
        // This is the generic body, so its types refer to the function's generic parameters.
        let mir_body = fndef.body().unwrap();
        let body =
            self.with_item_generics(fndef.def_id(), true, |this| this.translate_body(mir_body));
        Ok(body)
    }

    fn translate_body(&mut self, mir_body: Body) -> CharonBody {
        let span = self.translate_span(mir_body.span);
        let arg_count = self.instance.fn_abi().unwrap().args.len();
        let vars = self.translate_body_locals(&mir_body);
        let locals = CharonLocals { locals: vars, arg_count };
        // The synthetic abort block (see below) is appended after the translated
        // blocks, so its ID is the number of MIR blocks. It must be assigned
        // *before* translating the blocks: `Call` terminators reference it as
        // their `on_unwind` target, and as their return target when the callee
        // never returns.
        self.abort_block = CharonBlockId::from_usize(mir_body.blocks.len());
        let mut body: CharonBodyContents =
            mir_body.blocks.iter().map(|bb| self.translate_block(bb)).collect();

        // Add the synthetic block that aborts (Kani does not model unwinding).
        let abort_block = CharonBlockData {
            statements: Vec::new(),
            terminator: CharonTerminator::new(
                span,
                CharonRawTerminator::Abort(CharonAbortKind::UndefinedBehavior),
            ),
        };
        body.push(abort_block);
        assert_eq!(self.abort_block.index(), body.len() - 1);

        // TODO: Kani does not bind any region in bodies.
        let body_expr =
            CharonExprBody { span, bound_body_regions: 0, locals, body, comments: Vec::new() };
        CharonBody::Unstructured(body_expr)
    }

    fn translate_generic_args(&mut self, ga: GenericArgs, defid: DefId) -> CharonGenericArgs {
        let genvec = ga.0;
        let mut c_regions: CharonVector<CharonRegionId, CharonRegion> = CharonVector::new();
        let mut c_types: CharonVector<CharonTypeVarId, CharonTy> = CharonVector::new();
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstantExpr> =
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
                    let c_const_generic = self.tyconst_to_constgeneric(tc, None);
                    c_const_generics.push(c_const_generic);
                }
            }
        }
        let (gen_trait_refs, spans) = self.get_traitrefs_and_span_from_defid(defid);
        let mut trait_refs: CharonVector<CharonTraitClauseId, CharonTraitRef> = CharonVector::new();
        let trait_ref_span_zip = zip(spans.clone(), gen_trait_refs.clone());
        for (_, trait_ref) in trait_ref_span_zip {
            let traitgenarg = trait_ref.trait_decl_ref.skip_binder.generics.clone();
            let t_regions: CharonVector<CharonRegionId, CharonRegion> = CharonVector::new();
            let mut t_types: CharonVector<CharonTypeVarId, CharonTy> = CharonVector::new();
            let t_const_generics: CharonVector<CharonConstGenericVarId, CharonConstantExpr> =
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
            let traitdecl_id = trait_ref.trait_decl_ref.skip_binder.id;
            let subs_traitdeclref = CharonPolyTraitDeclRef {
                regions: trait_ref.trait_decl_ref.regions.clone(),
                skip_binder: CharonTraitDeclRef {
                    id: traitdecl_id,
                    generics: Box::new(generics.clone()),
                },
            };
            // TODO: this proof is a placeholder, as it was before Charon changed the
            // representation: Kani does not resolve which impl proves the clause.
            let subs_traitref = CharonTraitRef::new(
                CharonTraitRefKind::BuiltinOrAuto {
                    builtin_data: CharonBuiltinImplData::Auto,
                    parent_trait_refs: CharonVector::new(),
                    types: Default::default(),
                    vtable: None,
                },
                subs_traitdeclref,
            );
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
        let mut c_const_generics: CharonVector<CharonConstGenericVarId, CharonConstantExpr> =
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
                    let c_const_generic = self.tyconst_to_constgeneric(tc, None);
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
                    CharonDeBruijnId::new(self.binder_depth),
                    CharonTypeVarId::from_usize(self.param_position(paramty.index)),
                );
                CharonTy::new(CharonTyKind::TypeVar(debr))
            }
            x => todo!("Not yet implemented translation for TyKind: {:?}", x),
        }
    }

    /// A type-level constant (an array length or a const generic argument). Charon merged its
    /// separate const-generic representation into `ConstantExpr`.
    fn tyconst_to_constgeneric(
        &mut self,
        tyconst: TyConst,
        param_ty: Option<CharonTy>,
    ) -> CharonConstantExpr {
        match tyconst.kind() {
            TyConstKind::Value(ty, alloc) => {
                let kind = self.translate_allocation(alloc, *ty);
                CharonConstantExpr::new(kind, self.translate_ty(*ty))
            }
            TyConstKind::Param(paramc) => {
                let debr = CharonDeBruijnVar::Bound(
                    CharonDeBruijnId::new(self.binder_depth),
                    CharonConstGenericVarId::from_usize(self.param_position(paramc.index)),
                );
                // Neither `TyConst` nor `ParamConst` carries the parameter's type.
                let ty = param_ty.unwrap_or_else(|| todo!("const generic parameter {paramc:?}"));
                CharonConstantExpr::new(CharonRawConstantExpr::Var(debr), ty)
            }
            _ => todo!(),
        }
    }

    /// Translate a type, following Charon's own translation (`translate_ty`): arrays and slices
    /// carry no `Sized` proof (the `aeneas` preset hides marker traits), tuples are the builtin
    /// tuple ADT (the preset does not generate tuple structs), and `Box` is tagged as builtin.
    fn translate_rigid_ty(&mut self, rigid_ty: RigidTy) -> CharonTy {
        debug!("translate_rigid_ty: {rigid_ty:?}");
        match rigid_ty {
            RigidTy::Bool => CharonTy::new(CharonTyKind::Scalar(CharonLiteralTy::Bool)),
            RigidTy::Char => CharonTy::new(CharonTyKind::Scalar(CharonLiteralTy::Char)),
            RigidTy::Int(it) => {
                CharonTy::new(CharonTyKind::Scalar(CharonLiteralTy::Integer(translate_int_ty(it))))
            }
            RigidTy::Uint(uit) => CharonTy::new(CharonTyKind::Scalar(CharonLiteralTy::Integer(
                translate_uint_ty(uit),
            ))),
            RigidTy::Never => CharonTy::new(CharonTyKind::Never),
            RigidTy::Str => CharonTy::new(CharonTyKind::Adt(self.str_type_decl_ref())),
            RigidTy::Array(ty, tyconst) => {
                let c_ty = self.translate_ty(ty);
                // An array length is always a `usize`.
                let len = self.tyconst_to_constgeneric(tyconst, Some(CharonTy::mk_usize()));
                CharonTy::mk_array(c_ty, len, None)
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
                CharonTy::new(CharonTyKind::Adt(CharonTypeDeclRef {
                    id: CharonTypeDeclId::UNIT,
                    generics: Box::new(CharonGenericArgs::new_types(types)),
                    builtin: Some(CharonBuiltinAdt::Tuple),
                }))
            }
            RigidTy::FnDef(def_id, args) => {
                let fn_ptr = CharonFnPtr {
                    kind: Box::new(CharonFunIdOrTraitMethodRef::Fun(
                        self.register_fun_decl_id(def_id.def_id()),
                    )),
                    generics: Box::new(self.translate_generic_args(args, def_id.def_id())),
                };
                // TODO: populate regions?
                CharonTy::new(CharonTyKind::FnDef(CharonRegionBinder {
                    regions: CharonVector::new(),
                    skip_binder: fn_ptr,
                }))
            }
            RigidTy::Adt(adt_def, genarg) => {
                let def_id = adt_def.def_id();
                let c_typedeclid = self.register_type_decl_id(def_id);
                if self.translated.type_decls.get(c_typedeclid).is_none() {
                    self.translate_adtdef(adt_def);
                }
                let c_generic_args = self.translate_generic_args(genarg, adt_def.def_id());
                let internal = rustc_internal::internal(self.tcx, def_id);
                let builtin = self
                    .tcx
                    .is_lang_item(internal, rustc_hir::attrs::lang_items::LangItem::OwnedBox)
                    .then_some(CharonBuiltinAdt::Box);
                CharonTy::new(CharonTyKind::Adt(CharonTypeDeclRef {
                    id: c_typedeclid,
                    generics: Box::new(c_generic_args),
                    builtin,
                }))
            }
            RigidTy::Slice(ty) => CharonTy::mk_slice(self.translate_ty(ty), None),
            RigidTy::RawPtr(ty, mutability) => {
                let c_ty = self.translate_ty(ty);
                CharonTy::new(CharonTyKind::RawPtr(
                    c_ty,
                    match mutability {
                        Mutability::Mut => CharonRefKind::Mut,
                        Mutability::Not => CharonRefKind::Shared,
                    },
                ))
            }
            RigidTy::FnPtr(polyfunsig) => {
                let mut regions = CharonVector::new();
                for name in late_bound_regions(&polyfunsig) {
                    regions.push_with(|index| CharonRegionVar {
                        index,
                        name,
                        variance: CharonVariance::Unknown,
                        mutability: CharonLifetimeMutability::Unknown,
                    });
                }
                let value = polyfunsig.value;
                self.binder_depth += 1;
                let inputs = value.inputs().iter().map(|ty| self.translate_ty(*ty)).collect();
                let output = self.translate_ty(value.output());
                self.binder_depth -= 1;
                let sig = CharonFunSig {
                    is_unsafe: value.safety == rustc_public::mir::Safety::Unsafe,
                    abi: CharonAbi::Rust,
                    is_variadic: value.c_variadic,
                    inputs,
                    output,
                };
                CharonTy::new(CharonTyKind::FnPtr(CharonRegionBinder { regions, skip_binder: sig }))
            }
            // Kani never translated trait objects: this used to be a placeholder predicate, and
            // Charon now requires the real one.
            RigidTy::Dynamic(_, _) => {
                CharonTy::new(CharonTyKind::Error("trait objects are not supported".to_owned()))
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
            let name = self.local_names.get(&local).cloned();
            let span = self.translate_span(local_decl.span);
            locals.push_with(|index| CharonVar { index, name, span, ty, drop_flag_for: None });
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
        content.map(|content| {
            let span = self.translate_span(stmt.source_info.span);
            CharonStatement::new(span, content)
        })
    }

    fn translate_terminator(
        &mut self,
        terminator: &Terminator,
    ) -> (Option<CharonStatement>, CharonTerminator) {
        let span = self.translate_span(terminator.source_info.span);
        let (statement, terminator) = match &terminator.kind {
            TerminatorKind::Return => (None, CharonRawTerminator::Return),
            TerminatorKind::Goto { target } => {
                (None, CharonRawTerminator::Goto { target: CharonBlockId::from_usize(*target) })
            }
            TerminatorKind::Unreachable => {
                (None, CharonRawTerminator::Abort(CharonAbortKind::UndefinedBehavior))
            }
            TerminatorKind::Drop { place, target, .. } => {
                // Charon now carries the drop glue to run. Upstream reaches it through a trait
                // proof for its synthetic `Destruct::drop_glue` method, which Kani does not model.
                // `resolve_drop_in_place` gives the glue for `T` directly: an instance of the
                // `core::ptr::drop_glue` lang item (which `drop_in_place::<T>` merely wraps), and
                // Kani already collects it.
                let place_ty = place.ty(self.instance.body().unwrap().locals()).unwrap();
                let drop_glue = Instance::resolve_drop_in_place(place_ty);
                let fn_ptr = self.translate_fn_ptr(drop_glue);
                (
                    None,
                    CharonRawTerminator::Drop {
                        // Kani translates optimized MIR, where drops are precise.
                        kind: CharonDropKind::Precise,
                        place: self.translate_place(place),
                        fn_ptr,
                        target: CharonBlockId::from_usize(*target),
                        on_unwind: self.abort_block,
                    },
                )
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                let (data, branches) = self.translate_switch_targets(discr, targets);
                (None, CharonRawTerminator::Switch { data, branches })
            }
            TerminatorKind::Call { func, args, destination, target, .. } => {
                debug!("translate_call: {func:?} {args:?} {destination:?} {target:?}");
                let fn_ty = func.ty(self.instance.body().unwrap().locals()).unwrap();
                let fn_ptr = match fn_ty.kind() {
                    TyKind::RigidTy(RigidTy::FnDef(def, genarg)) => {
                        let instance = Instance::resolve(def, &genarg).unwrap();
                        self.translate_fn_ptr(instance)
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
                (
                    None,
                    CharonRawTerminator::Call {
                        call,
                        // A call to a diverging function has no return target in MIR. ULLBC
                        // requires one, so point it at the synthetic abort block: control cannot
                        // reach it, and aborting if it somehow did matches how the CBMC backend
                        // models the same case ("Unexpected return from Never function").
                        // Unwrapping here turned any program containing such a call -- a call to
                        // `panic!`, `process::exit`, or any `-> !` function -- into a compiler
                        // crash.
                        target: target.map_or(self.abort_block, CharonBlockId::from_usize),
                        on_unwind: self.abort_block,
                    },
                )
            }
            // As in Charon's own translation, an `Assert` terminator whose `check_kind` records
            // which check it is: `reconstruct_fallible_operations` needs it to fold an overflow,
            // bounds or division check into the operation it guards.
            TerminatorKind::Assert { cond, expected, msg, target, .. } => (
                None,
                CharonRawTerminator::Assert {
                    assert: CharonAssert {
                        cond: self.translate_operand(cond),
                        expected: *expected,
                        check_kind: Some(self.translate_assert_kind(msg)),
                    },
                    target: CharonBlockId::from_usize(*target),
                    on_unwind: self.abort_block,
                },
            ),
            _ => todo!(),
        };
        (
            statement.map(|statement| CharonStatement::new(span, statement)),
            CharonTerminator::new(span, terminator),
        )
    }

    /// A pointer to the function `instance`.
    fn translate_fn_ptr(&mut self, instance: Instance) -> CharonFnPtr {
        let def_id = instance.def.def_id();
        let fid = self.register_fun_decl_id(def_id);
        let genarg_resolve = match instance.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(_, ga)) => ga,
            _ => panic!("Expected a function type"),
        };
        let mut generics = self.translate_generic_args(genarg_resolve, def_id);
        // The callee's declaration also binds its signature's late-bound regions
        // (`generic_params_from_fndef`), which the instance's arguments do not carry. Pass them as
        // erased, as Charon's own translation does; Charon's type check rejects the call otherwise.
        let sig = match instance.ty().kind() {
            TyKind::RigidTy(RigidTy::FnDef(fndef, _)) => fndef.fn_sig(),
            _ => panic!("Expected a function type"),
        };
        for _ in late_bound_regions(&sig) {
            generics.regions.push(CharonRegion::Erased);
        }
        CharonFnPtr::new(CharonFunIdOrTraitMethodRef::Fun(fid), generics)
    }

    /// The check an `Assert` performs, as Charon's own translation (`translate_assert_kind`)
    /// records it.
    fn translate_assert_kind(&mut self, msg: &AssertMessage) -> CharonBuiltinAssertKind {
        use CharonBuiltinAssertKind as K;
        match msg {
            AssertMessage::BoundsCheck { len, index } => K::BoundsCheck {
                len: self.translate_operand(len),
                index: self.translate_operand(index),
            },
            AssertMessage::Overflow(bin_op, lhs, rhs) => K::Overflow(
                translate_bin_op(*bin_op),
                self.translate_operand(lhs),
                self.translate_operand(rhs),
            ),
            AssertMessage::OverflowNeg(op) => K::OverflowNeg(self.translate_operand(op)),
            AssertMessage::DivisionByZero(op) => K::DivisionByZero(self.translate_operand(op)),
            AssertMessage::RemainderByZero(op) => K::RemainderByZero(self.translate_operand(op)),
            AssertMessage::MisalignedPointerDereference { required, found } => {
                K::MisalignedPointerDereference {
                    required: self.translate_operand(required),
                    found: self.translate_operand(found),
                }
            }
            AssertMessage::NullPointerDereference => K::NullPointerDereference,
            AssertMessage::NullReferenceConstructed => K::NullReferenceCreated,
            AssertMessage::InvalidEnumConstruction(op) => {
                K::InvalidEnumConstruction(self.translate_operand(op))
            }
            AssertMessage::ResumedAfterReturn(_) => K::ResumedAfterReturn,
            AssertMessage::ResumedAfterPanic(_) => K::ResumedAfterPanic,
            AssertMessage::ResumedAfterDrop(_) => K::ResumedAfterDrop,
        }
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

    //Get the Ty of the Place
    fn place_ty(&self, place: &Place) -> Ty {
        let body = self.instance.body().unwrap();
        let ty = body.local_decl(place.local).unwrap().ty;
        ty
    }

    fn translate_rvalue(&mut self, rvalue: &Rvalue) -> CharonRvalue {
        trace!("translate_rvalue: {rvalue:?}");
        match rvalue {
            Rvalue::Use(operand, retag) => CharonRvalue::Use(
                self.translate_operand(operand),
                match retag {
                    rustc_public::mir::WithRetag::Yes => CharonWithRetag::Yes,
                    rustc_public::mir::WithRetag::No => CharonWithRetag::No,
                },
            ),
            Rvalue::Repeat(_operand, _) => todo!(),
            Rvalue::Ref(_region, kind, place) => CharonRvalue::Ref {
                place: self.translate_place(place),
                kind: translate_borrow_kind(kind),
                // Filled in by Charon's `insert_ptr_metadata` pass, as for Charon's own
                // translation.
                ptr_metadata: missing_ptr_metadata(),
            },
            Rvalue::AddressOf(_, _) => todo!(),
            Rvalue::Len(place) => CharonRvalue::Len(
                self.translate_place(place),
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
                translate_checked_bin_op(*bin_op),
                self.translate_operand(lhs),
                self.translate_operand(rhs),
            ),
            Rvalue::UnaryOp(op, operand) => {
                CharonRvalue::UnaryOp(translate_un_op(*op), self.translate_operand(operand))
            }
            Rvalue::Discriminant(place) => {
                let c_ty = self.translate_ty(self.place_ty(place));
                match c_ty.kind() {
                    CharonTyKind::Adt(_) => CharonRvalue::Discriminant(self.translate_place(place)),
                    _ => todo!("Not yet implemented:{:?}", c_ty.kind()),
                }
            }

            Rvalue::Aggregate(agg_kind, operands) => {
                let c_operands =
                    (*operands).iter().map(|operand| self.translate_operand(operand)).collect();
                // The type the aggregate builds, as `translate_ty` translates it: for ADTs and
                // tuples this is the `TypeDeclRef` the aggregate names.
                let agg_ty =
                    self.translate_ty(rvalue.ty(self.instance.body().unwrap().locals()).unwrap());
                match agg_kind.clone() {
                    AggregateKind::Adt(adt_def, variant_id, _genarg, _user_anot, field_id) => {
                        let (c_variant_id, c_field_id) = match adt_def.kind() {
                            AdtKind::Enum => (
                                Some(CharonVariantId::from_usize(variant_id.to_index())),
                                field_id.map(CharonFieldId::from_usize),
                            ),
                            AdtKind::Struct => (None, None),
                            _ => todo!(),
                        };
                        let tref = agg_ty.as_adt().unwrap().clone();
                        CharonRvalue::Aggregate(
                            CharonAggregateKind::Adt(tref, c_variant_id, c_field_id),
                            c_operands,
                        )
                    }
                    AggregateKind::Tuple => {
                        let tref = agg_ty.as_adt().unwrap().clone();
                        CharonRvalue::Aggregate(
                            CharonAggregateKind::Adt(tref, None, None),
                            c_operands,
                        )
                    }
                    AggregateKind::Array(ty) => {
                        let c_ty = self.translate_ty(ty);
                        let len = CharonConstantExpr::mk_usize(c_operands.len() as u128);
                        // No `Sized` proof, as in `translate_rigid_ty`.
                        CharonRvalue::Aggregate(
                            CharonAggregateKind::Array(c_ty, len, None),
                            c_operands,
                        )
                    }
                    _ => todo!(),
                }
            }

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
            // `Operand::RuntimeChecks` (rust-lang/rust#148766) is not yet modeled by the
            // experimental LLBC backend.
            Operand::RuntimeChecks(_) => todo!(),
        }
    }

    fn translate_constant(&mut self, constant: &ConstOperand) -> CharonConstantExpr {
        trace!("translate_constant: {constant:?}");
        let value = self.translate_constant_value(&constant.const_);
        CharonConstantExpr::new(value, self.translate_ty(constant.ty()))
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
                    generics: Box::new(c_genarg),
                })
            }
            ConstantKind::Param(_) => todo!(),
        }
    }

    fn translate_allocation(&self, alloc: &Allocation, ty: Ty) -> CharonRawConstantExpr {
        match ty.kind() {
            TyKind::RigidTy(RigidTy::Int(it)) => {
                // `as u128` keeps the two's-complement bits, which `from_bits` sign-extends.
                let bits = alloc.read_int().unwrap() as u128;
                CharonRawConstantExpr::Integer(CharonScalarValue::from_bits(
                    translate_int_ty(it),
                    bits,
                ))
            }
            TyKind::RigidTy(RigidTy::Uint(uit)) => {
                let bits = alloc.read_uint().unwrap();
                CharonRawConstantExpr::Integer(CharonScalarValue::from_bits(
                    translate_uint_ty(uit),
                    bits,
                ))
            }
            TyKind::RigidTy(RigidTy::Bool) => {
                CharonRawConstantExpr::Bool(alloc.read_bool().unwrap())
            }
            TyKind::RigidTy(RigidTy::Char) => {
                let value = char::from_u32(alloc.read_uint().unwrap() as u32);
                CharonRawConstantExpr::Char(value.unwrap())
            }
            _ => todo!("Not yet implement {:?}, {:?}", ty, alloc),
        }
    }

    fn translate_cast(&self, _kind: CastKind, _operand: &Operand, _ty: Ty) -> CharonCastKind {
        todo!()
    }

    /// Translate a `SwitchInt`, following Charon's own `translate_switch_targets`: one branch per
    /// distinct target block, each case value built with `ConstantExprKind::from_bits`, and the
    /// `otherwise` target as the fallback.
    fn translate_switch_targets(
        &mut self,
        discr: &Operand,
        targets: &SwitchTargets,
    ) -> (CharonSwitchData, CharonVector<CharonBranchId, CharonBlockId>) {
        trace!("translate_switch_targets: {discr:?} {targets:?}");
        let ty = discr.ty(self.instance.body().unwrap().locals()).unwrap();
        let discr = self.translate_operand(discr);
        let switch_ty = self.translate_ty(ty);
        let switch_scalar_ty = *switch_ty.kind().as_scalar().unwrap();
        let mut branch_targets: CharonVector<CharonBranchId, CharonBlockId> = CharonVector::new();
        let mut target_to_branch: IndexMap<CharonBlockId, CharonBranchId> = IndexMap::new();
        let mut branch_of = |target: usize| {
            let target = CharonBlockId::from_usize(target);
            *target_to_branch.entry(target).or_insert_with(|| branch_targets.push(target))
        };

        // Keep Charon's true-then-false traversal order for boolean switches.
        let bool_fallback =
            (switch_scalar_ty == CharonLiteralTy::Bool).then(|| branch_of(targets.otherwise()));
        let branches = targets
            .branches()
            .map(|(bits, target)| {
                let kind = CharonRawConstantExpr::from_bits(&switch_scalar_ty, bits)
                    .unwrap_or_else(|| panic!("Can't match on type {switch_ty:?}"));
                (CharonConstantExpr::new(kind, switch_ty.clone()), branch_of(target))
            })
            .collect();
        let fallback = bool_fallback.unwrap_or_else(|| branch_of(targets.otherwise()));
        let data = CharonSwitchData {
            scrutinee: CharonSwitchScrutinee::Value(discr),
            branches,
            fallback: Some(fallback),
        };
        (data, branch_targets)
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
                    // As in Charon's own translation: struct and tuple fields are projected
                    // without a variant, enum fields with the variant `Downcast` selected.
                    let variant = match current_ty.kind() {
                        CharonTyKind::Adt(tref) if tref.builtin.is_some() => Some(None),
                        CharonTyKind::Adt(tref) => {
                            match self.translated.type_decls.get(tref.id).map(|d| &d.kind) {
                                Some(CharonTypeDeclKind::Struct(_)) => Some(None),
                                Some(CharonTypeDeclKind::Enum(_)) => Some(Some(c_variantid)),
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    if let Some(variant) = variant {
                        current_ty = self.translate_ty(*ty);
                        c_provec.push((
                            CharonProjectionElem::Field(variant, c_fieldid),
                            current_ty.clone(),
                        ));
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
                    CharonDeBruijnId { index: self.binder_depth },
                    CharonRegionId::from_usize(self.param_position(epr.index)),
                );
                CharonRegion::Var(debr)
            }
            RegionKind::ReBound(var, boundregion) => {
                // A region bound by the function's own signature is one of the function's
                // generics, numbered after its early-bound regions; any other binder is a
                // function-pointer type's, whose regions are numbered from zero.
                let offset = match &self.item_generics {
                    Some(generics)
                        if generics.binds_late_regions && var as usize == self.binder_depth =>
                    {
                        generics.early_regions
                    }
                    _ => 0,
                };
                let debr = CharonDeBruijnVar::bound(
                    CharonDeBruijnId { index: var as usize },
                    CharonRegionId::from_usize(offset + boundregion.var as usize),
                );
                CharonRegion::Var(debr)
            }
            RegionKind::RePlaceholder(_) => {
                todo!("Not yet implemented RegionKind: {:?}", region.kind)
            }
        }
    }
}

/// Set up a translated crate the way Charon's own driver does before translating any item:
/// record the target, and claim the first type declaration id for the unit type.
///
/// `TypeDeclId::UNIT` is where every tuple type points when tuple structs are not generated (the
/// `aeneas` preset), so it must be the declaration of `()`; otherwise every tuple would silently
/// name whichever ADT happened to be registered first.
pub fn prepare_translated_crate(tcx: TyCtxt, translated: &mut CharonTranslatedCrate) {
    let unit_id = translated.type_decls.reserve_slot();
    assert_eq!(unit_id, CharonTypeDeclId::UNIT, "the unit type must come first");
    let name = CharonName {
        name: vec![CharonPathElem::Builtin(
            CharonBuiltinPathElem::Tuple(0),
            CharonDisambiguator::ZERO,
        )],
    };
    translated.type_decls.set_slot(
        unit_id,
        CharonTypeDecl {
            def_id: unit_id,
            item_meta: item_meta(CharonSpan::dummy(), name),
            generics: CharonGenericParams::empty(),
            src: CharonTypeSource::Builtin(CharonBuiltinAdt::Tuple),
            // What Charon declares for `()` when it does not generate tuple structs.
            kind: CharonTypeDeclKind::Opaque,
            layout: Default::default(),
            ptr_metadata: CharonPtrMetadata::None,
        },
    );

    // As in Charon's `register_target_info`.
    let target_data = &tcx.data_layout;
    let mut primitive_alignments = charon_lib::ast::SeqHashMap::new();
    primitive_alignments.insert(CharonLiteralTy::Bool, target_data.i8_align.bytes());
    let int = |ty| CharonLiteralTy::Integer(ty);
    for (ty, alignment) in [
        (CharonIntegerTy::Signed(CharonIntTy::I8), target_data.i8_align.bytes()),
        (CharonIntegerTy::Signed(CharonIntTy::I16), target_data.i16_align.bytes()),
        (CharonIntegerTy::Signed(CharonIntTy::I32), target_data.i32_align.bytes()),
        (CharonIntegerTy::Signed(CharonIntTy::I64), target_data.i64_align.bytes()),
        (CharonIntegerTy::Signed(CharonIntTy::I128), target_data.i128_align.bytes()),
        (CharonIntegerTy::Signed(CharonIntTy::Isize), target_data.pointer_align().bytes()),
        (CharonIntegerTy::Unsigned(CharonUIntTy::U8), target_data.i8_align.bytes()),
        (CharonIntegerTy::Unsigned(CharonUIntTy::U16), target_data.i16_align.bytes()),
        (CharonIntegerTy::Unsigned(CharonUIntTy::U32), target_data.i32_align.bytes()),
        (CharonIntegerTy::Unsigned(CharonUIntTy::U64), target_data.i64_align.bytes()),
        (CharonIntegerTy::Unsigned(CharonUIntTy::U128), target_data.i128_align.bytes()),
        (CharonIntegerTy::Unsigned(CharonUIntTy::Usize), target_data.pointer_align().bytes()),
    ] {
        primitive_alignments.insert(int(ty), alignment);
    }
    for (ty, alignment) in [
        (CharonFloatTy::F16, target_data.f16_align.bytes()),
        (CharonFloatTy::F32, target_data.f32_align.bytes()),
        (CharonFloatTy::F64, target_data.f64_align.bytes()),
        (CharonFloatTy::F128, target_data.f128_align.bytes()),
    ] {
        primitive_alignments.insert(CharonLiteralTy::Float(ty), alignment);
    }
    // Not guaranteed by the reference, but by rustc's implementation (as Charon notes).
    primitive_alignments.insert(CharonLiteralTy::Char, target_data.i32_align.bytes());
    let c_enum_smallest_repr_ty = match target_data.c_enum_min_size {
        rustc_abi::Integer::I8 => CharonIntTy::I8,
        rustc_abi::Integer::I16 => CharonIntTy::I16,
        rustc_abi::Integer::I32 => CharonIntTy::I32,
        rustc_abi::Integer::I64 => CharonIntTy::I64,
        rustc_abi::Integer::I128 => CharonIntTy::I128,
    };
    let info = CharonTargetInfo {
        target_pointer_size: target_data.pointer_size().bytes(),
        is_little_endian: matches!(target_data.endian, rustc_abi::Endian::Little),
        c_enum_smallest_repr_ty,
        primitive_alignments,
    };
    translated.target_information.insert(tcx.sess.opts.target_triple.tuple().to_owned(), info);
}

/// Record every declaration's name in `item_names`, which Charon's passes and printer read and
/// which Charon's own driver fills in as it registers items.
pub fn record_item_names(translated: &mut CharonTranslatedCrate) {
    let mut names = Vec::new();
    names.extend(
        translated
            .type_decls
            .iter()
            .map(|d| (CharonAnyTransId::Type(d.def_id), d.item_meta.name.clone())),
    );
    names.extend(
        translated
            .fun_decls
            .iter()
            .map(|d| (CharonAnyTransId::Fun(d.def_id), d.item_meta.name.clone())),
    );
    names.extend(
        translated
            .global_decls
            .iter()
            .map(|d| (CharonAnyTransId::Global(d.def_id), d.item_meta.name.clone())),
    );
    names.extend(
        translated
            .trait_decls
            .iter()
            .map(|d| (CharonAnyTransId::TraitDecl(d.def_id), d.item_meta.name.clone())),
    );
    names.extend(
        translated
            .trait_impls
            .iter()
            .map(|d| (CharonAnyTransId::TraitImpl(d.def_id), d.item_meta.name.clone())),
    );
    translated.item_names.extend(names);
}

/// The placeholder metadata of a borrow, which Charon's `insert_ptr_metadata` pass replaces. This
/// is the same placeholder as Charon's own translation emits.
fn missing_ptr_metadata() -> CharonOperand {
    CharonOperand::Const(CharonConstantExpr::new(
        CharonRawConstantExpr::Opaque("Missing metadata".to_string()),
        CharonTy::mk_unit(),
    ))
}

/// The regions bound by a function signature's binder, in order, with their names if any. They
/// are its late-bound regions, wherever in the signature they occur, each listed once.
fn late_bound_regions(sig: &PolyFnSig) -> Vec<Option<String>> {
    sig.bound_vars
        .iter()
        .map(|var| match var {
            // Charon leaves elided (`'_`) regions unnamed, too.
            BoundVariableKind::Region(BoundRegionKind::BrNamed(_, name)) if name == "'_" => None,
            BoundVariableKind::Region(BoundRegionKind::BrNamed(_, name)) => Some(name.clone()),
            BoundVariableKind::Region(BoundRegionKind::BrAnon | BoundRegionKind::BrEnv) => None,
            // Only `#![feature(non_lifetime_binders)]` binds anything else here.
            BoundVariableKind::Ty(_) | BoundVariableKind::Const => {
                todo!("non-region bound variable {var:?}")
            }
        })
        .collect()
}

fn loc(n: usize) -> u32 {
    u32::try_from(n).expect("source location out of range")
}

/// The meta information Kani gives every item it translates.
fn item_meta(span: CharonSpan, name: CharonName) -> CharonItemMeta {
    CharonItemMeta {
        name,
        span,
        // TODO: populate the source text
        source_text: None,
        attr_info: default_attr_info(),
        // Aeneas only translates items that are local to the top-level crate
        // Since we want all reachable items (including those in external
        // crates) to be translated, always set `is_local` to true
        is_local: true,
        // For now, assume all items are transparent
        opacity: CharonItemOpacity::Transparent,
        lang_item: None,
        diagnostic_item: None,
        has_errors: false,
    }
}

// TODO: populate the attribute info
fn default_attr_info() -> CharonAttrInfo {
    CharonAttrInfo { attributes: Vec::new(), inline: None, rename: None, public: true }
}

fn translate_int_ty(int_ty: IntTy) -> CharonIntegerTy {
    CharonIntegerTy::Signed(match int_ty {
        IntTy::I8 => CharonIntTy::I8,
        IntTy::I16 => CharonIntTy::I16,
        IntTy::I32 => CharonIntTy::I32,
        IntTy::I64 => CharonIntTy::I64,
        IntTy::I128 => CharonIntTy::I128,
        IntTy::Isize => CharonIntTy::Isize,
    })
}

fn translate_uint_ty(uint_ty: UintTy) -> CharonIntegerTy {
    CharonIntegerTy::Unsigned(match uint_ty {
        UintTy::U8 => CharonUIntTy::U8,
        UintTy::U16 => CharonUIntTy::U16,
        UintTy::U32 => CharonUIntTy::U32,
        UintTy::U64 => CharonUIntTy::U64,
        UintTy::U128 => CharonUIntTy::U128,
        UintTy::Usize => CharonUIntTy::Usize,
    })
}

/// The operator of a MIR `CheckedBinaryOp`, which yields `(result, overflowed)`. Charon folds it with
/// the overflow `Assert` that follows into a panicking operator.
fn translate_checked_bin_op(bin_op: BinOp) -> CharonBinOp {
    match bin_op {
        BinOp::Add => CharonBinOp::AddChecked,
        BinOp::Sub => CharonBinOp::SubChecked,
        BinOp::Mul => CharonBinOp::MulChecked,
        _ => translate_bin_op(bin_op),
    }
}

/// The operator of a plain MIR `BinaryOp`, mapped as Charon's own translation does
/// (`translate_binaryop_kind`): MIR's `Add`/`Sub`/`Mul`/shifts wrap, their `*Unchecked` forms and
/// `Div`/`Rem` are undefined behavior on overflow (MIR guards them with an explicit `Assert`).
fn translate_bin_op(bin_op: BinOp) -> CharonBinOp {
    use CharonOverflowMode::{UB, Wrap};
    match bin_op {
        BinOp::Add => CharonBinOp::Add(Wrap),
        BinOp::AddUnchecked => CharonBinOp::Add(UB),
        BinOp::Sub => CharonBinOp::Sub(Wrap),
        BinOp::SubUnchecked => CharonBinOp::Sub(UB),
        BinOp::Mul => CharonBinOp::Mul(Wrap),
        BinOp::MulUnchecked => CharonBinOp::Mul(UB),
        BinOp::Div => CharonBinOp::Div(UB),
        BinOp::Rem => CharonBinOp::Rem(UB),
        BinOp::BitXor => CharonBinOp::BitXor,
        BinOp::BitAnd => CharonBinOp::BitAnd,
        BinOp::BitOr => CharonBinOp::BitOr,
        BinOp::Shl => CharonBinOp::Shl(Wrap),
        BinOp::ShlUnchecked => CharonBinOp::Shl(UB),
        BinOp::Shr => CharonBinOp::Shr(Wrap),
        BinOp::ShrUnchecked => CharonBinOp::Shr(UB),
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
        // As in Charon's own translation; MIR guards overflow with an explicit `Assert`.
        UnOp::Neg => CharonUnOp::Neg(CharonOverflowMode::Wrap),
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
