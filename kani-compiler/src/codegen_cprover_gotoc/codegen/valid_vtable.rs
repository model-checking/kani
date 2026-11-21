// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The validity of trait objects and of the metadata of pointers to them, for
//! `-Z valid-value-checks` (and `kani::mem` functions that check validity).
//!
//! The metadata of a pointer to a `dyn Trait` must be the address of a vtable for `Trait`. The
//! vtables a program can create are the ones the compiler generates, which Kani generates while
//! it codegens the program: a vtable for each unsizing coercion of a type to a trait object (and
//! for the supertraits its vtable points to), and the vtables in constants. A vtable for a trait
//! can also be reused as the vtable of another: a cast of `dyn A + Send` to `dyn A` or to
//! `dyn Send`, or of `dyn Sub` to a supertrait whose vtable is a prefix of `Sub`'s. Valid metadata
//! is then the address of one of these vtables, at offset 0, which CBMC can check by comparing
//! the pointer with each of their addresses.
//!
//! The set is only complete once the whole program has been codegened, so the instrumentation
//! calls a function per trait, whose body Kani generates at the end of codegen.

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::InternedString;
use cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use rustc_middle::ty::{self, ExistentialTraitRef, TyCtxt, TypingEnv};
use rustc_public::rustc_internal;
use rustc_public::ty::Ty as TyStable;
use std::collections::HashMap;

/// The principal trait of a trait object type, with its regions erased, which identifies the
/// layout of its vtables. Auto traits don't change it. `None` for a trait object without a
/// principal trait, such as `dyn Send`.
type TraitKey<'tcx> = Option<ExistentialTraitRef<'tcx>>;

/// A vtable that Kani generated.
struct Vtable {
    /// The address of the vtable.
    addr: Expr,
}

#[derive(Default)]
pub struct ValidVtableCtx<'tcx> {
    /// The vtables generated so far, by trait, in the order they were generated.
    vtables: HashMap<TraitKey<'tcx>, Vec<Vtable>>,
    /// The names of the vtables recorded, so that each is recorded once.
    recorded: std::collections::HashSet<InternedString>,
    /// For each trait, the traits whose vtables the program reuses as vtables of that trait.
    reused: HashMap<TraitKey<'tcx>, Vec<TraitKey<'tcx>>>,
    /// The functions the instrumentation calls, `fn(vtable: *const u8) -> bool`, which check that
    /// the vtable is one of the trait's, in the order they were requested.
    functions: Vec<(TraitKey<'tcx>, InternedString)>,
    /// The index in `functions` of each function.
    function_idx: HashMap<TraitKey<'tcx>, usize>,
}

/// The key of the trait object type `dyn_ty`, or of the trait object at its tail.
fn trait_key<'tcx>(tcx: TyCtxt<'tcx>, dyn_ty: ty::Ty<'tcx>) -> TraitKey<'tcx> {
    let tail = tcx.struct_tail_for_codegen(dyn_ty, TypingEnv::fully_monomorphized());
    let ty::Dynamic(predicates, ..) = tail.kind() else {
        unreachable!("expected a trait object, found `{dyn_ty}`")
    };
    principal_key(tcx, predicates.principal())
}

fn principal_key<'tcx>(
    tcx: TyCtxt<'tcx>,
    principal: Option<ty::PolyExistentialTraitRef<'tcx>>,
) -> TraitKey<'tcx> {
    principal.map(|principal| {
        tcx.erase_and_anonymize_regions(tcx.instantiate_bound_regions_with_erased(principal))
    })
}

impl<'tcx> GotocCtx<'tcx, '_> {
    /// Record the vtable at `addr`, generated for the trait object type `dyn_ty`.
    pub fn record_vtable(&mut self, dyn_ty: ty::Ty<'tcx>, name: InternedString, addr: Expr) {
        let key = trait_key(self.tcx, dyn_ty);
        self.record_vtable_for_key(key, name, addr);
    }

    /// Record a vtable from a constant (`GlobalAlloc::VTable`).
    pub fn record_const_vtable(
        &mut self,
        principal: Option<ty::PolyExistentialTraitRef<'tcx>>,
        name: InternedString,
        addr: Expr,
    ) {
        let key = principal_key(self.tcx, principal);
        self.record_vtable_for_key(key, name, addr);
    }

    fn record_vtable_for_key(&mut self, key: TraitKey<'tcx>, name: InternedString, addr: Expr) {
        let ctx = &mut self.valid_vtables;
        if ctx.recorded.insert(name) {
            ctx.vtables.entry(key).or_default().push(Vtable { addr });
        }
    }

    /// Record that the program uses the vtables of the trait object type `src_dyn` as vtables
    /// of `dst_dyn`, as a cast between pointers to them that keeps the vtable does.
    pub fn record_vtable_reuse(&mut self, src_dyn: TyStable, dst_dyn: TyStable) {
        let src = trait_key(self.tcx, rustc_internal::internal(self.tcx, src_dyn));
        let dst = trait_key(self.tcx, rustc_internal::internal(self.tcx, dst_dyn));
        if src != dst {
            let reused = self.valid_vtables.reused.entry(dst).or_default();
            if !reused.contains(&src) {
                reused.push(src);
            }
        }
    }

    /// The function that checks that a vtable is valid for the trait object at the tail of
    /// `dyn_ty`, which takes the vtable.
    pub fn valid_vtable_fn(&mut self, dyn_ty: TyStable) -> Expr {
        let key = trait_key(self.tcx, rustc_internal::internal(self.tcx, dyn_ty));
        let name = if let Some(idx) = self.valid_vtables.function_idx.get(&key) {
            self.valid_vtables.functions[*idx].1
        } else {
            let idx = self.valid_vtables.functions.len();
            let name: InternedString =
                format!("{}::kani_valid_vtable::{idx}", self.full_crate_name()).into();
            let pretty = match key {
                Some(principal) => format!("kani_valid_vtable::<dyn {principal}>"),
                None => "kani_valid_vtable::<dyn _>".to_string(),
            };
            let param_name = format!("{name}::vtable");
            let sym = Symbol::variable(&param_name, "vtable", byte_ptr(), Location::none())
                .with_is_parameter(true);
            self.symbol_table.insert(sym);
            let param = byte_ptr().as_parameter(Some(param_name.into()), Some("vtable".into()));
            let typ = Type::code(vec![param], Type::c_bool());
            self.symbol_table.insert(Symbol::function(name, typ, None, pretty, Location::none()));
            self.valid_vtables.functions.push((key, name));
            self.valid_vtables.function_idx.insert(key, idx);
            name
        };
        let sym = self.symbol_table.lookup(name).unwrap();
        Expr::symbol_expression(sym.name, sym.typ.clone())
    }

    /// The addresses of the vtables valid for the trait `key`: its own, and the ones of the
    /// traits whose vtables the program reuses for it, transitively.
    fn valid_vtables_for(&self, key: TraitKey<'tcx>) -> Vec<Expr> {
        let ctx = &self.valid_vtables;
        let mut keys = vec![key];
        let mut idx = 0;
        while idx < keys.len() {
            for src in ctx.reused.get(&keys[idx]).into_iter().flatten() {
                if !keys.contains(src) {
                    keys.push(*src);
                }
            }
            idx += 1;
        }
        keys.iter()
            .flat_map(|key| ctx.vtables.get(key).into_iter().flatten())
            .map(|vtable| vtable.addr.clone())
            .collect()
    }

    /// Generate the bodies of the functions the instrumentation calls, once all vtables are
    /// known: `vtable == &vtable_1 || ... || vtable == &vtable_n`.
    pub fn codegen_valid_vtable_functions(&mut self) {
        for idx in 0..self.valid_vtables.functions.len() {
            let (key, name) = self.valid_vtables.functions[idx];
            let vtable = Expr::symbol_expression(format!("{name}::vtable"), byte_ptr())
                .cast_to(Type::void_pointer());
            let valid = self
                .valid_vtables_for(key)
                .into_iter()
                .map(|addr| vtable.clone().eq(addr.cast_to(Type::void_pointer())))
                .reduce(Expr::or)
                .unwrap_or_else(Expr::bool_false);
            let body = Stmt::ret(Some(valid.cast_to(Type::c_bool())), Location::none());
            self.symbol_table.update_fn_declaration_with_definition(name, body);
        }
    }
}

fn byte_ptr() -> Type {
    Type::unsigned_int(8).to_pointer()
}
