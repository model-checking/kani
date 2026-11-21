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
//! calls a function per trait, whose body Kani generates at the end of codegen. A check of a trait
//! object also checks that its value is valid at the type its vtable was generated for.

use crate::codegen_cprover_gotoc::GotocCtx;
use crate::codegen_cprover_gotoc::codegen::PropertyClass;
use crate::kani_middle::transform::check_values::{ValidityRange, ty_validity_per_offset};
use cbmc::InternedString;
use cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use rustc_middle::ty::{self, ExistentialTraitRef, TyCtxt, TypingEnv};
use rustc_public::rustc_internal;
use rustc_public::target::MachineInfo;
use rustc_public::ty::Ty as TyStable;
use std::collections::HashMap;

/// The principal trait of a trait object type, with its regions erased, which identifies the
/// layout of its vtables. Auto traits don't change it. `None` for a trait object without a
/// principal trait, such as `dyn Send`.
type TraitKey<'tcx> = Option<ExistentialTraitRef<'tcx>>;

/// What a generated function checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CheckKind {
    /// `fn(vtable: *const u8) -> bool`: the vtable is one of the trait's.
    Vtable,
    /// `fn(data: *const u8, vtable: *const u8, tail_offset: usize) -> bool`: the vtable is one of
    /// the trait's, and the value at `data + tail_offset` (rounded up to the alignment of the
    /// type the vtable is for) is a valid value of that type.
    DynValue,
}

/// A vtable that Kani generated.
struct Vtable {
    /// The type the vtable is for.
    concrete_ty: TyStable,
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
    /// The functions the instrumentation calls, in the order they were requested.
    functions: Vec<(TraitKey<'tcx>, CheckKind, InternedString)>,
    /// The index in `functions` of each function.
    function_idx: HashMap<(TraitKey<'tcx>, CheckKind), usize>,
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
    /// Record the vtable at `addr`, generated for `concrete_ty` as the trait object type
    /// `dyn_ty`.
    pub fn record_vtable(
        &mut self,
        dyn_ty: ty::Ty<'tcx>,
        concrete_ty: TyStable,
        name: InternedString,
        addr: Expr,
    ) {
        let key = trait_key(self.tcx, dyn_ty);
        self.record_vtable_for_key(key, concrete_ty, name, addr);
    }

    /// Record a vtable from a constant (`GlobalAlloc::VTable`).
    pub fn record_const_vtable(
        &mut self,
        principal: Option<ty::PolyExistentialTraitRef<'tcx>>,
        concrete_ty: ty::Ty<'tcx>,
        name: InternedString,
        addr: Expr,
    ) {
        let key = principal_key(self.tcx, principal);
        self.record_vtable_for_key(key, rustc_internal::stable(concrete_ty), name, addr);
    }

    fn record_vtable_for_key(
        &mut self,
        key: TraitKey<'tcx>,
        concrete_ty: TyStable,
        name: InternedString,
        addr: Expr,
    ) {
        let ctx = &mut self.valid_vtables;
        if ctx.recorded.insert(name) {
            ctx.vtables.entry(key).or_default().push(Vtable { concrete_ty, addr });
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
        self.valid_vtable_fn_for(key, CheckKind::Vtable)
    }

    /// The function that checks that the trait object at the tail of a `dyn_ty` is valid, which
    /// takes the data pointer, the vtable and the offset of the tail.
    pub fn valid_dyn_value_fn(&mut self, dyn_ty: TyStable) -> Expr {
        let key = trait_key(self.tcx, rustc_internal::internal(self.tcx, dyn_ty));
        self.valid_vtable_fn_for(key, CheckKind::DynValue)
    }

    fn valid_vtable_fn_for(&mut self, key: TraitKey<'tcx>, kind: CheckKind) -> Expr {
        let name = if let Some(idx) = self.valid_vtables.function_idx.get(&(key, kind)) {
            self.valid_vtables.functions[*idx].2
        } else {
            let idx = self.valid_vtables.functions.len();
            let suffix = match kind {
                CheckKind::Vtable => "vtable",
                CheckKind::DynValue => "dyn_value",
            };
            let name: InternedString =
                format!("{}::kani_valid_{suffix}::{idx}", self.full_crate_name()).into();
            let pretty = match key {
                Some(principal) => format!("kani_valid_{suffix}::<dyn {principal}>"),
                None => format!("kani_valid_{suffix}::<dyn _>"),
            };
            let params: Vec<(&str, Type)> = match kind {
                CheckKind::Vtable => vec![("vtable", byte_ptr())],
                CheckKind::DynValue => vec![
                    ("data", byte_ptr()),
                    ("vtable", byte_ptr()),
                    ("tail_offset", Type::size_t()),
                ],
            };
            let params = params
                .into_iter()
                .map(|(base_name, typ)| {
                    let param_name = format!("{name}::{base_name}");
                    let sym =
                        Symbol::variable(&param_name, base_name, typ.clone(), Location::none())
                            .with_is_parameter(true);
                    self.symbol_table.insert(sym);
                    typ.as_parameter(Some(param_name.into()), Some(base_name.into()))
                })
                .collect();
            let typ = Type::code(params, Type::c_bool());
            self.symbol_table.insert(Symbol::function(name, typ, None, pretty, Location::none()));
            self.valid_vtables.functions.push((key, kind, name));
            self.valid_vtables.function_idx.insert((key, kind), idx);
            name
        };
        let sym = self.symbol_table.lookup(name).unwrap();
        Expr::symbol_expression(sym.name, sym.typ.clone())
    }

    /// The vtables valid for the trait `key`: its own, and the ones of the traits whose vtables
    /// the program reuses for it, transitively.
    fn valid_vtables_for(&self, key: TraitKey<'tcx>) -> Vec<(TyStable, Expr)> {
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
            .map(|vtable| (vtable.concrete_ty, vtable.addr.clone()))
            .collect()
    }

    /// Generate the bodies of the functions the instrumentation calls, once all vtables are
    /// known. Generating them may request new functions, for the vtables in the values they
    /// check, which are generated too.
    pub fn codegen_valid_vtable_functions(&mut self) {
        let mut idx = 0;
        while idx < self.valid_vtables.functions.len() {
            let (key, kind, name) = self.valid_vtables.functions[idx];
            let param = |base_name: &str| {
                Expr::symbol_expression(
                    format!("{name}::{base_name}"),
                    match base_name {
                        "tail_offset" => Type::size_t(),
                        _ => byte_ptr(),
                    },
                )
            };
            let vtable = param("vtable").cast_to(Type::void_pointer());
            let loc = Location::none();
            let body = match kind {
                CheckKind::Vtable => {
                    let valid = self
                        .valid_vtables_for(key)
                        .into_iter()
                        .map(|(_, addr)| vtable.clone().eq(addr.cast_to(Type::void_pointer())))
                        .reduce(Expr::or)
                        .unwrap_or_else(Expr::bool_false);
                    Stmt::ret(Some(valid.cast_to(Type::c_bool())), loc)
                }
                CheckKind::DynValue => {
                    let mut stmts = vec![];
                    for (concrete_ty, addr) in self.valid_vtables_for(key) {
                        let is_this = vtable.clone().eq(addr.cast_to(Type::void_pointer()));
                        let align = concrete_ty.layout().unwrap().shape().abi_align;
                        // `data + ((tail_offset + align - 1) & !(align - 1))`
                        let offset = param("tail_offset")
                            .plus(Expr::int_constant(align - 1, Type::size_t()))
                            .bitand(Expr::int_constant(!(align - 1), Type::size_t()));
                        let value = param("data").plus(offset);
                        let check = match self.codegen_value_validity(value, concrete_ty) {
                            Ok(valid) => Stmt::ret(Some(valid.cast_to(Type::c_bool())), loc),
                            Err(msg) => {
                                let msg = format!(
                                    "Kani currently doesn't support checking validity of a trait \
                                     object of type `{concrete_ty}`. {msg}"
                                );
                                Stmt::block(
                                    vec![
                                        self.codegen_assert_assume_false(
                                            PropertyClass::UnsupportedConstruct,
                                            &msg,
                                            loc,
                                        ),
                                        Stmt::ret(Some(Expr::c_true()), loc),
                                    ],
                                    loc,
                                )
                            }
                        };
                        stmts.push(Stmt::if_then_else(is_this, check, None, loc));
                    }
                    stmts.push(Stmt::ret(Some(Expr::c_false()), loc));
                    Stmt::block(stmts, loc)
                }
            };
            self.symbol_table.update_fn_declaration_with_definition(name, body);
            idx += 1;
        }
    }

    /// Whether the `ty` at the byte pointer `ptr` is valid, following the requirements
    /// `ty_validity_per_offset` computes for the MIR instrumentation.
    fn codegen_value_validity(&mut self, ptr: Expr, ty: TyStable) -> Result<Expr, String> {
        let machine = MachineInfo::target();
        let reqs = ty_validity_per_offset(self.tcx, &machine, ty, 0)?;
        let read = |offset: usize, typ: Type| {
            ptr.clone()
                .plus(Expr::int_constant(offset, Type::size_t()))
                .cast_to(typ.to_pointer())
                .dereference()
        };
        let in_range = |value: Expr, range: &rustc_public::abi::WrappingRange, typ: &Type| {
            let start = value.clone().ge(Expr::int_constant(range.start, typ.clone()));
            let end = value.le(Expr::int_constant(range.end, typ.clone()));
            if range.wraps_around() { start.or(end) } else { start.and(end) }
        };
        let mut valid = Expr::bool_true();
        for req in reqs {
            let uint = Type::unsigned_int(req.size().bits() as u64);
            let mut req_valid = match req.valid_range() {
                ValidityRange::Single(range) => {
                    in_range(read(req.offset(), uint.clone()), range, &uint)
                }
                ValidityRange::Multiple(ranges) => ranges
                    .iter()
                    .map(|range| in_range(read(req.offset(), uint.clone()), range, &uint))
                    .reduce(Expr::or)
                    .unwrap_or_else(Expr::bool_false),
                ValidityRange::Vtable(dyn_ty) => {
                    let check = self.valid_vtable_fn(*dyn_ty);
                    check.call(vec![read(req.offset(), byte_ptr())]).cast_to(Type::bool())
                }
                ValidityRange::DynValue(_) => {
                    unreachable!("the type of a vtable is sized: `{ty}`")
                }
            };
            for (offset, size, range) in req.guards() {
                let uint = Type::unsigned_int(size.bits() as u64);
                req_valid = in_range(read(offset, uint.clone()), &range, &uint).not().or(req_valid);
            }
            valid = valid.and(req_valid);
        }
        Ok(valid)
    }
}

fn byte_ptr() -> Type {
    Type::unsigned_int(8).to_pointer()
}
