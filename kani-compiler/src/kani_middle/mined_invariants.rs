// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Mine type invariants from a type's own assertions (C14 static form).
//!
//! Source: `&self` methods of the type's inherent impls whose `kani::assert` calls (Kani's
//! macro overrides have rewritten user asserts/panics into these) satisfy all of:
//! - the assert's block *post-dominates* the entry block (the claim holds on every normal
//!   return, structurally rejecting mode-guarded asserts like `if ready { assert!(..) }`);
//! - the condition's backward slice is pure and call-free: leaves are field projections of
//!   `self` or constants, interior nodes single-assignment temporaries combined with
//!   whitelisted operators.
//! Conditions are extracted into a small expression AST ([MinedExpr]), which provides a
//! canonical form for the frequency filter (a conjunct must be asserted in at least
//! [MIN_ASSERTING_METHODS] distinct methods, guarding against method-local preconditions
//! masquerading as type invariants) and is trivially total when re-materialized as MIR.

use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    BinOp, Body, ConstOperand, Operand, Place, Rvalue, StatementKind, TerminatorKind, UnOp,
};
use rustc_public::ty::{FnDef, RigidTy, Ty, TyKind};
use rustc_public_bridge::IndexedVal;

/// A conjunct must be asserted in at least this many distinct methods to be considered a
/// type invariant rather than a method-local precondition.
pub const MIN_ASSERTING_METHODS: usize = 2;

/// A pure expression over the fields of a value of the mined type.
#[derive(Clone, Debug)]
pub enum MinedExpr {
    /// A chain of field projections starting at the value itself, with the field type.
    Field(Vec<(usize, Ty)>),
    /// A field chain under an enum variant downcast: only meaningful when the value's
    /// discriminant equals the variant index (consumers guard with an implication).
    DowncastField(usize, Vec<(usize, Ty)>),
    /// A constant: the canonical token (for equality/hashing across methods) plus the
    /// original operand (for re-materialization).
    Const(String, ConstOperand),
    BinOp(BinOp, Box<MinedExpr>, Box<MinedExpr>),
    UnOp(UnOp, Box<MinedExpr>),
}

impl PartialEq for MinedExpr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MinedExpr::Field(a), MinedExpr::Field(b)) => a == b,
            (MinedExpr::DowncastField(v1, a), MinedExpr::DowncastField(v2, b)) => {
                v1 == v2 && a == b
            }
            (MinedExpr::Const(a, _), MinedExpr::Const(b, _)) => a == b,
            (MinedExpr::BinOp(o1, a1, b1), MinedExpr::BinOp(o2, a2, b2)) => {
                o1 == o2 && a1 == a2 && b1 == b2
            }
            (MinedExpr::UnOp(o1, a1), MinedExpr::UnOp(o2, a2)) => o1 == o2 && a1 == a2,
            _ => false,
        }
    }
}
impl Eq for MinedExpr {}
impl std::hash::Hash for MinedExpr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            MinedExpr::Field(path) => path.hash(state),
            MinedExpr::DowncastField(v, path) => {
                v.hash(state);
                path.hash(state);
            }
            MinedExpr::Const(token, _) => token.hash(state),
            MinedExpr::BinOp(op, a, b) => {
                format!("{op:?}").hash(state);
                a.hash(state);
                b.hash(state);
            }
            MinedExpr::UnOp(op, a) => {
                format!("{op:?}").hash(state);
                a.hash(state);
            }
        }
    }
}

impl MinedExpr {
    pub fn ty(&self) -> Option<Ty> {
        match self {
            MinedExpr::Field(path) => path.last().map(|(_, t)| *t),
            MinedExpr::DowncastField(_, path) => path.last().map(|(_, t)| *t),
            MinedExpr::Const(_, c) => Some(c.const_.ty()),
            MinedExpr::BinOp(op, a, _) => match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    Some(Ty::bool_ty())
                }
                _ => a.ty(),
            },
            MinedExpr::UnOp(_, a) => a.ty(),
        }
    }
}

/// A mined invariant conjunct: the boolean expression plus provenance for diagnostics.
#[derive(Clone, Debug)]
pub struct MinedConjunct {
    pub expr: MinedExpr,
    /// Methods (pretty names) that assert this conjunct.
    pub asserted_in: Vec<String>,
}

/// Whether the expression contains a variant-downcast field read.
pub fn conjunct_has_downcast(expr: &MinedExpr) -> bool {
    match expr {
        MinedExpr::Field(_) | MinedExpr::Const(_, _) => false,
        MinedExpr::DowncastField(..) => true,
        MinedExpr::BinOp(_, a, b) => conjunct_has_downcast(a) || conjunct_has_downcast(b),
        MinedExpr::UnOp(_, a) => conjunct_has_downcast(a),
    }
}

/// Whether `bb` post-dominates `from`/// Whether `bb` post-dominates `from` in `body` w.r.t. normal returns:
/// every path from `from` to a `Return` terminator passes through `bb`.
fn postdominates(body: &Body, from: usize, bb: usize) -> bool {
    // BFS from `from` avoiding `bb`; if any Return block is reachable, `bb` does not
    // post-dominate.
    let mut seen = vec![false; body.blocks.len()];
    let mut queue = vec![from];
    seen[from] = true;
    if bb == from {
        return true;
    }
    while let Some(cur) = queue.pop() {
        let term = &body.blocks[cur].terminator;
        if matches!(term.kind, TerminatorKind::Return) {
            return false;
        }
        let mut push = |t: usize| {
            if t != bb && !seen[t] {
                seen[t] = true;
                queue.push(t);
            }
        };
        match &term.kind {
            TerminatorKind::Goto { target } => push(*target),
            TerminatorKind::SwitchInt { targets, .. } => {
                for (_, t) in targets.branches() {
                    push(t);
                }
                push(targets.otherwise());
            }
            TerminatorKind::Call { target, .. } => {
                if let Some(t) = target {
                    push(*t);
                }
            }
            TerminatorKind::Drop { target, .. } => push(*target),
            TerminatorKind::Assert { target, .. } => push(*target),
            _ => {}
        }
    }
    true
}

/// Extract a [MinedExpr] for `op` in `body`, where local `_1` is `&self` (or `self`).
/// Returns None (bail) when the slice leaves the pure/call-free/single-assignment fragment.
fn extract_expr(body: &Body, op: &Operand, depth: usize) -> Option<MinedExpr> {
    if depth > 24 {
        return None;
    }
    match op {
        Operand::Constant(c) => extract_const(c),
        Operand::Copy(place) | Operand::Move(place) => extract_place(body, place, depth),
        Operand::RuntimeChecks(_) => None,
    }
}

fn extract_const(c: &ConstOperand) -> Option<MinedExpr> {
    // The Debug rendering of the MIR constant is the canonical token for cross-method
    // equality; the operand itself is kept for re-materialization.
    Some(MinedExpr::Const(format!("{:?}", c.const_), c.clone()))
}

/// `self` is local 1; a place rooted at it with Deref/Field projections is a field path
/// (an optional leading Downcast records the enum variant, for variant-guarded conjuncts).
fn extract_place(body: &Body, place: &Place, depth: usize) -> Option<MinedExpr> {
    use rustc_public::mir::ProjectionElem;
    if place.local == 1 {
        let mut path = vec![];
        let mut variant: Option<usize> = None;
        for elem in &place.projection {
            match elem {
                ProjectionElem::Deref => {}
                ProjectionElem::Field(idx, ty) => path.push((*idx, *ty)),
                ProjectionElem::Downcast(v) if path.is_empty() && variant.is_none() => {
                    variant = Some(v.to_index());
                }
                _ => return None,
            }
        }
        if path.is_empty() {
            return None; // whole-self uses are not field conditions
        }
        return Some(match variant {
            Some(v) => MinedExpr::DowncastField(v, path),
            None => MinedExpr::Field(path),
        });
    }
    // A temporary dereferenced once: if it uniquely holds a reference, the deref cancels
    // (match ergonomics bind payloads by reference: `v = &((*self) as Variant).0`).
    if place.projection.len() == 1 && matches!(place.projection[0], ProjectionElem::Deref) {
        let base = Place { local: place.local, projection: vec![] };
        if let Some(inner) = unique_ref_def(body, &base) {
            return extract_place(body, &inner, depth + 1);
        }
        return None;
    }
    // A temporary: find its unique defining assignment or defining call.
    if !place.projection.is_empty() {
        return None;
    }
    let mut def: Option<&Rvalue> = None;
    let mut call_def: Option<&rustc_public::mir::Terminator> = None;
    for block in &body.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(p, rv) = &stmt.kind
                && p.local == place.local
            {
                if p.projection.is_empty() {
                    if def.is_some() || call_def.is_some() {
                        return None; // multiple assignments (e.g. short-circuit merge)
                    }
                    def = Some(rv);
                } else {
                    return None;
                }
            }
        }
        if let TerminatorKind::Call { destination, .. } = &block.terminator.kind
            && destination.local == place.local
        {
            if def.is_some() || call_def.is_some() {
                return None;
            }
            call_def = Some(&block.terminator);
        }
    }
    if let Some(term) = call_def {
        // One-level getter inlining: a call to a pure accessor of `self` whose body is
        // itself extractable (e.g. `self.len()` where len returns a field expression).
        return extract_getter_call(body, term, depth);
    }
    match def? {
        Rvalue::Use(inner) => extract_expr(body, inner, depth + 1),
        Rvalue::BinaryOp(bop, a, b) => Some(MinedExpr::BinOp(
            *bop,
            Box::new(extract_expr(body, a, depth + 1)?),
            Box::new(extract_expr(body, b, depth + 1)?),
        )),
        Rvalue::UnaryOp(uop, a) => {
            Some(MinedExpr::UnOp(*uop, Box::new(extract_expr(body, a, depth + 1)?)))
        }
        Rvalue::CopyForDeref(p) => extract_place(body, p, depth + 1),
        _ => None,
    }
}

/// Extract the expression computed by a getter call `self.m()`: the sole argument must be
/// (a reference to) `self`, and the callee's return local must have a unique, extractable
/// definition in terms of ITS `self` (which is the same value). Depth-limited.
fn extract_getter_call(
    body: &Body,
    term: &rustc_public::mir::Terminator,
    depth: usize,
) -> Option<MinedExpr> {
    if depth > 3 {
        return None;
    }
    let TerminatorKind::Call { func, args, .. } = &term.kind else { return None };
    // Sole argument: self (possibly behind a fresh reference temp).
    if args.len() != 1 {
        return None;
    }
    let self_rooted = match &args[0] {
        Operand::Copy(p) | Operand::Move(p) => p.local == 1 || place_is_ref_to_self(body, p),
        _ => false,
    };
    if !self_rooted {
        return None;
    }
    let fn_ty = func.ty(body.locals()).ok()?;
    let TyKind::RigidTy(RigidTy::FnDef(def, fn_args)) = fn_ty.kind() else { return None };
    let inst = Instance::resolve(def, &fn_args).ok()?;
    let callee = inst.body()?;
    // Unique assignment to the return local, extractable in the callee's own self frame.
    let mut ret_def: Option<MinedExpr> = None;
    for block in &callee.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(p, rv) = &stmt.kind
                && p.local == 0
            {
                if ret_def.is_some() || !p.projection.is_empty() {
                    return None;
                }
                ret_def = Some(match rv {
                    Rvalue::Use(inner) => extract_expr(&callee, inner, depth + 1)?,
                    Rvalue::BinaryOp(bop, a, b) => MinedExpr::BinOp(
                        *bop,
                        Box::new(extract_expr(&callee, a, depth + 1)?),
                        Box::new(extract_expr(&callee, b, depth + 1)?),
                    ),
                    Rvalue::UnaryOp(uop, a) => {
                        MinedExpr::UnOp(*uop, Box::new(extract_expr(&callee, a, depth + 1)?))
                    }
                    _ => return None,
                });
            }
        }
        if let TerminatorKind::Call { destination, .. } = &block.terminator.kind
            && destination.local == 0
        {
            return None;
        }
    }
    ret_def
}

/// If `p` (a projection-free temp) is uniquely defined as `&<place>`, return that place.
fn unique_ref_def(body: &Body, p: &Place) -> Option<Place> {
    let mut found: Option<Place> = None;
    for block in &body.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(dest, rv) = &stmt.kind
                && dest.local == p.local
                && dest.projection.is_empty()
            {
                match rv {
                    Rvalue::Ref(_, _, inner) => {
                        if found.is_some() {
                            return None;
                        }
                        found = Some(inner.clone());
                    }
                    _ => return None,
                }
            }
        }
        if let TerminatorKind::Call { destination, .. } = &block.terminator.kind
            && destination.local == p.local
        {
            return None;
        }
    }
    found
}

/// Whether `p` is a temp holding `&self`/// Whether `p` is a temp holding `&self` (defined once as `Ref(.., self-place)`).
fn place_is_ref_to_self(body: &Body, p: &Place) -> bool {
    if !p.projection.is_empty() {
        return false;
    }
    let mut found = false;
    for block in &body.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(dest, rv) = &stmt.kind
                && dest.local == p.local
            {
                match rv {
                    Rvalue::Ref(_, _, inner) if inner.local == 1 => {
                        if found {
                            return false;
                        }
                        found = true;
                    }
                    _ => return false,
                }
            }
        }
    }
    found
}

/// Mine the invariant conjuncts of `ty` (a struct ADT) from its inherent `&self` methods.
/// Results are cached by the caller.
pub fn mine_self_assert_conjuncts(tcx: TyCtxt, ty: Ty, kani_assert: FnDef) -> Vec<MinedConjunct> {
    let TyKind::RigidTy(RigidTy::Adt(adt_def, ref adt_args)) = ty.kind() else {
        return vec![];
    };
    if !adt_args.0.is_empty() {
        return vec![];
    }
    let adt_did = rustc_public::rustc_internal::internal(tcx, adt_def.def_id());
    let mut by_expr: FxHashMap<MinedExpr, Vec<String>> = FxHashMap::default();
    for &impl_did in tcx.inherent_impls(adt_did) {
        for &item in tcx.associated_item_def_ids(impl_did) {
            if !tcx.def_kind(item).is_fn_like() || !tcx.associated_item(item).is_method() {
                continue;
            }
            // Skip generic methods; instantiate with no args.
            if tcx
                .generics_of(item)
                .own_params
                .iter()
                .any(|p| !matches!(p.kind, rustc_middle::ty::GenericParamDefKind::Lifetime))
            {
                continue;
            }
            let Some(fn_def) = crate::kani_middle::stable_fn_def(tcx, item) else { continue };
            let Ok(inst) = Instance::resolve(fn_def, &rustc_public::ty::GenericArgs(vec![])) else {
                continue;
            };
            let Some(body) = inst.body() else { continue };
            // First parameter must be self by-ref or by-value of our type.
            let Some(self_decl) = body.arg_locals().first() else { continue };
            let self_ok = self_decl.ty == ty
                || matches!(self_decl.ty.kind(),
                    TyKind::RigidTy(RigidTy::Ref(_, inner, _)) if inner == ty);
            if !self_ok {
                continue;
            }
            let method_name = fn_def.name();
            for (bb_idx, block) in body.blocks.iter().enumerate() {
                let TerminatorKind::Call { func, args, .. } = &block.terminator.kind else {
                    continue;
                };
                let Ok(fn_ty) = func.ty(body.locals()) else { continue };
                let TyKind::RigidTy(RigidTy::FnDef(def, _)) = fn_ty.kind() else { continue };
                if def != kani_assert {
                    continue;
                }
                // Unconditional claim (post-dominates entry), or a claim guarded by a
                // match on self's discriminant (post-dominates that arm's entry): the
                // latter yields a variant-guarded conjunct via the DowncastField reads in
                // its expression.
                let unconditional = postdominates(&body, 0, bb_idx);
                let mut arm_guarded = false;
                if !unconditional {
                    'guard: for block in &body.blocks {
                        let TerminatorKind::SwitchInt { discr, targets } = &block.terminator.kind
                        else {
                            continue;
                        };
                        // The switch must be on self's discriminant.
                        let is_self_discr = match discr {
                            Operand::Copy(p) | Operand::Move(p) => {
                                p.projection.is_empty()
                                    && body.blocks.iter().any(|b| {
                                        b.statements.iter().any(|st| {
                                            matches!(&st.kind,
                                                StatementKind::Assign(d, Rvalue::Discriminant(src))
                                                    if d.local == p.local && src.local == 1)
                                        })
                                    })
                            }
                            _ => false,
                        };
                        if !is_self_discr {
                            continue;
                        }
                        for (_, target) in targets.branches() {
                            if postdominates(&body, target, bb_idx) {
                                arm_guarded = true;
                                break 'guard;
                            }
                        }
                        if postdominates(&body, targets.otherwise(), bb_idx) {
                            arm_guarded = true;
                            break 'guard;
                        }
                    }
                }
                if !unconditional && !arm_guarded {
                    continue;
                }
                let Some(expr) = extract_expr(&body, &args[0], 0) else { continue };
                // Arm-guarded conjuncts must carry the variant via their downcast reads;
                // unconditional conjuncts must not (a bare downcast read without its match
                // would be under-guarded).
                match (unconditional, conjunct_has_downcast(&expr)) {
                    (true, true) | (false, false) => continue,
                    _ => {}
                }
                if expr.ty() != Some(Ty::bool_ty()) {
                    continue;
                }
                let entry = by_expr.entry(expr).or_default();
                if !entry.contains(&method_name) {
                    entry.push(method_name.clone());
                }
            }
        }
    }
    by_expr
        .into_iter()
        .filter(|(_, methods)| methods.len() >= MIN_ASSERTING_METHODS)
        .map(|(expr, asserted_in)| MinedConjunct { expr, asserted_in })
        .collect()
}
