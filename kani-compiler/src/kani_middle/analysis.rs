// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! MIR analysis passes that extracts information about the MIR model given as input to codegen.
//!
//! # Performance Impact
//!
//! This module will perform all the analyses requested. Callers are responsible for selecting
//! when the cost of these analyses are worth it.

use rustc_middle::mir::mono::MonoItem as InternalMonoItem;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::MonoItem;
use stable_mir::mir::{
    visit::Location, MirVisitor, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use std::collections::HashMap;
use std::fmt::Display;

/// This function will collect and print some information about the given set of mono items.
///
/// This function will print information like:
///  - Number of items per type (Function / Constant / Shims)
///  - Number of instructions per type.
///  - Total number of MIR instructions.
pub fn print_stats<'tcx>(tcx: TyCtxt<'tcx>, items: &[InternalMonoItem<'tcx>]) {
    rustc_internal::run(tcx, || {
        let items: Vec<MonoItem> = items.iter().map(rustc_internal::stable).collect();
        let item_types = items.iter().collect::<Counter>();
        let visitor = items
            .iter()
            .filter_map(
                |mono| {
                    if let MonoItem::Fn(instance) = mono { Some(instance) } else { None }
                },
            )
            .fold(StatsVisitor::default(), |mut visitor, body| {
                visitor.visit_body(&body.body());
                visitor
            });
        eprintln!("====== Reachability Analysis Result =======");
        eprintln!("Total # items: {}", item_types.total());
        eprintln!("Total # statements: {}", visitor.stmts.total());
        eprintln!("Total # expressions: {}", visitor.exprs.total());
        eprintln!("\nReachable Items:\n{item_types}");
        eprintln!("Statements:\n{}", visitor.stmts);
        eprintln!("Expressions:\n{}", visitor.exprs);
        eprintln!("-------------------------------------------")
    });
}

#[derive(Default)]
/// MIR Visitor that collects information about the body of an item.
struct StatsVisitor {
    /// The types of each statement / terminator visited.
    stmts: Counter,
    /// The kind of each expressions found.
    exprs: Counter,
}

impl MirVisitor for StatsVisitor {
    fn visit_statement(&mut self, statement: &Statement, location: Location) {
        self.stmts.add(statement);
        // Also visit the type of expression.
        self.super_statement(statement, location);
    }

    fn visit_terminator(&mut self, terminator: &Terminator, _location: Location) {
        self.stmts.add(terminator);
        // Stop here since we don't care today about the information inside the terminator.
        // self.super_terminator(terminator, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, _location: Location) {
        self.exprs.add(rvalue);
        // Stop here since we don't care today about the information inside the rvalue.
        // self.super_rvalue(rvalue, location);
    }
}

#[derive(Default)]
struct Counter {
    data: HashMap<Key, usize>,
}

impl Counter {
    fn add<T: Into<Key>>(&mut self, item: T) {
        *self.data.entry(item.into()).or_default() += 1;
    }

    fn total(&self) -> usize {
        self.data.iter().fold(0, |acc, item| acc + item.1)
    }
}

impl Display for Counter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, freq) in &self.data {
            writeln!(f, "  - {}: {freq}", name.0)?;
        }
        std::fmt::Result::Ok(())
    }
}

impl<T: Into<Key>> FromIterator<T> for Counter {
    // Required method
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut counter = Counter::default();
        for item in iter {
            counter.add(item.into())
        }
        counter
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct Key(pub &'static str);

impl From<&MonoItem> for Key {
    fn from(value: &stable_mir::mir::mono::MonoItem) -> Self {
        match value {
            MonoItem::Fn(_) => Key("function"),
            MonoItem::GlobalAsm(_) => Key("global assembly"),
            MonoItem::Static(_) => Key("static item"),
        }
    }
}

impl From<&Statement> for Key {
    fn from(value: &Statement) -> Self {
        match value.kind {
            StatementKind::Assign(..) => Key("Assign"),
            StatementKind::Deinit(_) => Key("Deinit"),
            StatementKind::Intrinsic(_) => Key("Intrinsic"),
            StatementKind::SetDiscriminant { .. } => Key("SetDiscriminant"),
            // For now, we don't care about the ones below.
            StatementKind::AscribeUserType { .. }
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::FakeRead(..)
            | StatementKind::Nop
            | StatementKind::PlaceMention(_)
            | StatementKind::Retag(_, _)
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_) => Key("Ignored"),
        }
    }
}

impl From<&Terminator> for Key {
    fn from(value: &Terminator) -> Self {
        match value.kind {
            TerminatorKind::Abort => Key("Abort"),
            TerminatorKind::Assert { .. } => Key("Assert"),
            TerminatorKind::Call { .. } => Key("Call"),
            TerminatorKind::Drop { .. } => Key("Drop"),
            TerminatorKind::CoroutineDrop => Key("CoroutineDrop"),
            TerminatorKind::Goto { .. } => Key("Goto"),
            TerminatorKind::InlineAsm { .. } => Key("InlineAsm"),
            TerminatorKind::Resume { .. } => Key("Resume"),
            TerminatorKind::Return => Key("Return"),
            TerminatorKind::SwitchInt { .. } => Key("SwitchInt"),
            TerminatorKind::Unreachable => Key("Unreachable"),
        }
    }
}

impl From<&Rvalue> for Key {
    fn from(value: &Rvalue) -> Self {
        match value {
            Rvalue::Use(_) => Key("Use"),
            Rvalue::Repeat(_, _) => Key("Repeat"),
            Rvalue::Ref(_, _, _) => Key("Ref"),
            Rvalue::ThreadLocalRef(_) => Key("ThreadLocalRef"),
            Rvalue::AddressOf(_, _) => Key("AddressOf"),
            Rvalue::Len(_) => Key("Len"),
            Rvalue::Cast(_, _, _) => Key("Cast"),
            Rvalue::BinaryOp(..) => Key("BinaryOp"),
            Rvalue::CheckedBinaryOp(..) => Key("CheckedBinaryOp"),
            Rvalue::NullaryOp(_, _) => Key("NullaryOp"),
            Rvalue::UnaryOp(_, _) => Key("UnaryOp"),
            Rvalue::Discriminant(_) => Key("Discriminant"),
            Rvalue::Aggregate(_, _) => Key("Aggregate"),
            Rvalue::ShallowInitBox(_, _) => Key("ShallowInitBox"),
            Rvalue::CopyForDeref(_) => Key("CopyForDeref"),
        }
    }
}
