// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::InternedString;
use cbmc::goto_program::Stmt;
use rustc_middle::ty::Instance as InstanceInternal;
use rustc_middle::ty::{TyCtxt, UpvarCapture};
use rustc_public::CrateDef;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    Body, Local, LocalDecl, Operand, Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind,
    visit::Location, visit::MirVisitor,
};
use rustc_public::rustc_internal;
use rustc_public::ty::{RigidTy, TyKind};
use std::collections::{HashMap, HashSet};

/// This structure represents useful data about the function we are currently compiling.
#[derive(Debug)]
pub struct CurrentFnCtx<'tcx> {
    /// The GOTO block we are compiling into
    block: Vec<Stmt>,
    /// The codegen instance for the current function
    instance: Instance,
    /// The crate this function is from
    krate: String,
    /// The current instance. This is using the internal representation.
    instance_internal: InstanceInternal<'tcx>,
    /// A list of local declarations used to retrieve MIR component types.
    locals: Vec<LocalDecl>,
    /// The number of formal arguments of the current function (locals `1..=arg_count`).
    arg_count: usize,
    /// A list of pretty names for locals that corrspond to user variables.
    local_names: HashMap<Local, InternedString>,
    /// Collection of variables that are used in a reference or address-of expression.
    address_taken_locals: HashSet<Local>,
    /// Locals of contract-clause closures that hold copies of by-reference closure
    /// captures. See [CurrentFnCtx::is_capture_ref_local] for what qualifies.
    capture_ref_locals: HashSet<Local>,
    /// The symbol name of the current function
    name: String,
    /// A human readable pretty name for the current function
    readable_name: String,
    /// A counter to enable creating temporary variables
    temp_var_counter: u64,
}

struct AddressTakenLocalsCollector {
    /// Locals that appear in `Rvalue::Ref` or `Rvalue::AddressOf` expressions.
    address_taken_locals: HashSet<Local>,
}

impl MirVisitor for AddressTakenLocalsCollector {
    fn visit_rvalue(&mut self, rvalue: &Rvalue, _location: Location) {
        match rvalue {
            Rvalue::Ref(_, _, p) | Rvalue::AddressOf(_, p) => {
                if p.projection.is_empty() {
                    self.address_taken_locals.insert(p.local);
                }
            }
            _ => (),
        }
    }
}

/// Constructor
impl<'tcx> CurrentFnCtx<'tcx> {
    pub fn new(instance: Instance, gcx: &GotocCtx<'tcx, '_>, body: &Body) -> Self {
        let instance_internal = rustc_internal::internal(gcx.tcx, instance);
        let readable_name = crate::kani_middle::readable_name(instance);
        let name = instance.mangled_name();
        let locals = body.locals().to_vec();
        let arg_count = body.arg_locals().len();
        let local_names = body
            .var_debug_info
            .iter()
            .filter_map(|info| info.local().map(|local| (local, (&info.name).into())))
            .collect::<HashMap<_, _>>();
        let mut visitor = AddressTakenLocalsCollector { address_taken_locals: HashSet::new() };
        visitor.visit_body(body);
        let capture_ref_locals = compute_capture_ref_locals(
            gcx.tcx,
            instance_internal,
            body,
            &visitor.address_taken_locals,
        );
        Self {
            block: vec![],
            instance,
            instance_internal,
            krate: instance.def.krate().name,
            locals,
            arg_count,
            local_names,
            address_taken_locals: visitor.address_taken_locals,
            capture_ref_locals,
            name,
            readable_name,
            temp_var_counter: 0,
        }
    }
}

/// Setters
impl CurrentFnCtx<'_> {
    /// Returns the current block, replacing it with an empty vector.
    pub fn extract_block(&mut self) -> Vec<Stmt> {
        std::mem::take(&mut self.block)
    }

    pub fn get_and_incr_counter(&mut self) -> u64 {
        let rval = self.temp_var_counter;
        self.temp_var_counter += 1;
        rval
    }

    pub fn push_onto_block(&mut self, s: Stmt) {
        self.block.push(s)
    }
}

/// Getters
impl<'tcx> CurrentFnCtx<'tcx> {
    /// The function we are currently compiling
    pub fn instance(&self) -> InstanceInternal<'tcx> {
        self.instance_internal
    }

    pub fn instance_stable(&self) -> Instance {
        self.instance
    }

    /// The name of the function we are currently compiling
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The pretty name of the function we are currently compiling
    pub fn readable_name(&self) -> &str {
        &self.readable_name
    }

    pub fn locals(&self) -> &[LocalDecl] {
        &self.locals
    }

    /// The number of formal arguments of the current function; locals `1..=arg_count()`
    /// are the arguments.
    pub fn arg_count(&self) -> usize {
        self.arg_count
    }

    pub fn local_name(&self, local: Local) -> Option<InternedString> {
        self.local_names.get(&local).copied()
    }

    pub fn is_address_taken_local(&self, local: Local) -> bool {
        self.address_taken_locals.contains(&local)
    }

    /// Whether this function has any locals qualifying as capture-reference
    /// copies of a contract-clause closure (see [compute_capture_ref_locals]).
    pub fn has_capture_ref_locals(&self) -> bool {
        !self.capture_ref_locals.is_empty()
    }

    /// Whether `local` is a copy of a by-reference closure capture in a
    /// Kani-generated contract-clause closure. Dereferencing such a local only
    /// reads a local variable of the enclosing (necessarily live) stack frame,
    /// so pointer-validity checks on those dereferences are vacuous by
    /// construction.
    pub fn is_capture_ref_local(&self, local: Local) -> bool {
        self.capture_ref_locals.contains(&local)
    }
}

/// Utility functions
impl CurrentFnCtx<'_> {
    /// Is the current function from the `std` crate?
    pub fn is_std(&self) -> bool {
        self.krate == "std" || self.krate == "core"
    }
}

/// Compute the set of locals that are plain copies of by-reference closure
/// captures in a Kani-generated contract-clause closure.
///
/// Kani's function-contract instrumentation wraps every `requires`/`ensures`
/// clause in a closure that captures the contracted function's arguments by
/// reference. Evaluating a clause therefore loads each captured argument
/// through a reference that the instrumentation itself just created from a
/// live local of the enclosing frame — such loads cannot fail, yet CBMC's
/// `--pointer-check` instruments each of them with the full set of
/// pointer-validity checks. On contract-heavy code (e.g. the Rust standard
/// library) these vacuous checks make up the majority of all properties when
/// dependency contracts are asserted (the `-Zfunction-contracts` default).
///
/// A local `L` qualifies if all of the following hold, which together
/// guarantee that `*L` only reads a local of the enclosing, live stack frame:
/// * the current instance is a closure that takes its environment *by value*
///   (contract closures are called in place by the instrumentation; escaping
///   closures, which could outlive the frame their captures point into, are
///   called through a reference-typed environment and thus excluded),
/// * the innermost enclosing non-closure item carries a Kani contract
///   (`kanitool::checked_with`), i.e. the closure is contract machinery and
///   not arbitrary user code (we cannot use `kanitool::is_contract_generated`
///   due to <https://github.com/model-checking/kani/issues/3921>),
/// * the closure environment (local `_1`) is never written to,
/// * `L` is reference-typed, never address-taken, and its one and only
///   assignment is a direct copy/move of an environment field (`_1.i`).
///
/// Note that this deliberately only covers the *capture load* itself: if the
/// user-written clause expression dereferences the captured value again (e.g.
/// `#[requires(*ptr == 42)]`), that second dereference targets a different
/// local (assigned from `*L`, not from an environment field) and remains fully
/// checked.
fn compute_capture_ref_locals(
    tcx: TyCtxt,
    instance: InstanceInternal,
    body: &Body,
    address_taken_locals: &HashSet<Local>,
) -> HashSet<Local> {
    let def_id = instance.def_id();
    // Only closures with a by-value environment qualify.
    if !tcx.is_closure_like(def_id) || body.arg_locals().is_empty() {
        return HashSet::new();
    }
    let env_ty = body.locals()[1].ty;
    if !matches!(env_ty.kind(), TyKind::RigidTy(RigidTy::Closure(..))) {
        return HashSet::new();
    }
    // The innermost enclosing non-closure item must have a contract.
    let mut enclosing_id = tcx.opt_parent(def_id);
    while let Some(id) = enclosing_id {
        if !tcx.is_closure_like(id) {
            break;
        }
        enclosing_id = tcx.opt_parent(id);
    }
    let Some(enclosing_id) = enclosing_id else {
        return HashSet::new();
    };
    // Look for `kanitool::checked_with`, which marks functions that carry a
    // Kani contract. This mirrors `KaniAttributes::has_contract`, but works on
    // internal `DefId`s (the parent of a closure instance has no stable
    // counterpart we can reach from here).
    let checked_with_path = [
        rustc_span::symbol::Symbol::intern("kanitool"),
        rustc_span::symbol::Symbol::intern("checked_with"),
    ];
    if tcx.get_attrs_by_path(enclosing_id, &checked_with_path).next().is_none() {
        return HashSet::new();
    }
    // Only *by-reference* captures of unprojected places qualify: for those,
    // the environment field is a reference that closure construction itself
    // created to a local of the enclosing (live) frame. A by-value capture
    // that happens to be of reference type (e.g. a `&mut` argument moved into
    // the closure) holds a caller-provided reference instead, and
    // dereferencing it must remain checked. Similarly, a by-reference capture
    // of a projected place (e.g. capturing `*val`) can point through a user
    // reference. Capture information is only available for local definitions;
    // closures from other crates conservatively get no suppression.
    let Some(local_def_id) = def_id.as_local() else {
        return HashSet::new();
    };
    let captures = tcx.closure_captures(local_def_id);
    let is_safe_capture_field = |idx: usize| -> bool {
        captures.get(idx).is_some_and(|capture| {
            matches!(capture.info.capture_kind, UpvarCapture::ByRef(_))
                && capture.place.projections.is_empty()
        })
    };

    // Count writes to each local and identify environment-field copies.
    let mut writes: HashMap<Local, usize> = HashMap::new();
    let mut candidates: HashSet<Local> = HashSet::new();
    let mut count_write = |place: &Place, candidates: &mut HashSet<Local>| -> bool {
        if place.local == 1 {
            // The closure environment is written to: give up entirely.
            candidates.clear();
            return false;
        }
        if place.projection.is_empty() {
            *writes.entry(place.local).or_insert(0) += 1;
        }
        true
    };
    for block in &body.blocks {
        for stmt in &block.statements {
            if let StatementKind::Assign(place, rvalue) = &stmt.kind {
                if !count_write(place, &mut candidates) {
                    return HashSet::new();
                }
                if place.projection.is_empty()
                    && let Rvalue::Use(Operand::Copy(src) | Operand::Move(src)) = rvalue
                    && src.local == 1
                    && src.projection.len() == 1
                    && let ProjectionElem::Field(field_idx, _) = src.projection[0]
                    && is_safe_capture_field(field_idx)
                {
                    candidates.insert(place.local);
                }
            }
        }
        if let TerminatorKind::Call { destination, .. } = &block.terminator.kind
            && !count_write(destination, &mut candidates)
        {
            return HashSet::new();
        }
    }
    candidates.retain(|local| {
        writes.get(local) == Some(&1)
            && !address_taken_locals.contains(local)
            && matches!(body.locals()[*local].ty.kind(), TyKind::RigidTy(RigidTy::Ref(..)))
    });
    candidates
}
