// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! MIR analysis passes that extracts information about the MIR model given as input to codegen.
//!
//! # Performance Impact
//!
//! This module will perform all the analysis requested. Callers are responsible for selecting
//! when the cost of these analysis are worth it.

use rustc_middle::mir::mono::MonoItem;
use rustc_middle::mir::visit::Visitor as MirVisitor;
use rustc_middle::mir::{Location, Rvalue, Statement, StatementKind, Terminator, TerminatorKind};
use rustc_middle::ty::TyCtxt;
use std::collections::HashMap;
use std::fmt::Display;

/// This function will collect and print some information about the given set of mono items.
///
/// This function will print information like:
///  - Number of items per type (Function / Constant / Shims)
///  - Number of instructions per type.
///  - Total number of MIR instructions.
pub fn print_stats<'tcx>(tcx: TyCtxt<'tcx>, items: &[MonoItem<'tcx>]) {
    let item_types = items.iter().collect::<Counter>();
    let visitor = items
        .iter()
        .filter_map(|&mono| {
            if let MonoItem::Fn(instance) = mono {
                Some(tcx.instance_mir(instance.def))
            } else {
                None
            }
        })
        .fold(StatsVisitor::default(), |mut visitor, body| {
            visitor.visit_body(body);
            visitor
        });
    println!("====== Reachability Analysis Result =======");
    println!("Total # items: {}", item_types.total());
    println!("Total # statements: {}", visitor.stmts.total());
    println!("Total # expressions: {}", visitor.exprs.total());
    println!("\nReachable Items:\n{item_types}");
    println!("Statements:\n{}", visitor.stmts);
    println!("Expressions:\n{}", visitor.exprs);
    println!("-------------------------------------------")
}

#[derive(Default)]
/// MIR Visitor that collects information about the body of an item.
struct StatsVisitor {
    /// The types of each statement / terminator visited.
    stmts: Counter,
    /// The kind of each expressions found.
    exprs: Counter,
}

impl<'tcx> MirVisitor<'tcx> for StatsVisitor {
    fn visit_statement(&mut self, statement: &Statement<'tcx>, location: Location) {
        self.stmts.add(statement);
        // Also visit the type of expression.
        self.super_statement(statement, location);
    }

    fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, _location: Location) {
        self.stmts.add(terminator);
        // Stop here since we don't care today about the information inside the terminator.
        // self.super_terminator(terminator, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, _location: Location) {
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

impl<'tcx> From<&MonoItem<'tcx>> for Key {
    fn from(value: &MonoItem) -> Self {
        match value {
            MonoItem::Fn(_) => Key("function"),
            MonoItem::GlobalAsm(_) => Key("global assembly"),
            MonoItem::Static(_) => Key("static item"),
        }
    }
}

impl<'tcx> From<&Statement<'tcx>> for Key {
    fn from(value: &Statement<'tcx>) -> Self {
        match value.kind {
            StatementKind::Assign(_) => Key("Assign"),
            StatementKind::Deinit(_) => Key("Deinit"),
            StatementKind::Intrinsic(_) => Key("Intrinsic"),
            StatementKind::SetDiscriminant { .. } => Key("SetDiscriminant"),
            // For now, we don't care about the ones below.
            StatementKind::AscribeUserType(_, _)
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::FakeRead(_)
            | StatementKind::Nop
            | StatementKind::Retag(_, _)
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_) => Key("Ignored"),
        }
    }
}

impl<'tcx> From<&Terminator<'tcx>> for Key {
    fn from(value: &Terminator<'tcx>) -> Self {
        match value.kind {
            TerminatorKind::Abort => Key("Abort"),
            TerminatorKind::Assert { .. } => Key("Assert"),
            TerminatorKind::Call { .. } => Key("Call"),
            TerminatorKind::Drop { .. } => Key("Drop"),
            TerminatorKind::DropAndReplace { .. } => Key("DropAndReplace"),
            TerminatorKind::GeneratorDrop => Key("GeneratorDrop"),
            TerminatorKind::Goto { .. } => Key("Goto"),
            TerminatorKind::FalseEdge { .. } => Key("FalseEdge"),
            TerminatorKind::FalseUnwind { .. } => Key("FalseUnwind"),
            TerminatorKind::InlineAsm { .. } => Key("InlineAsm"),
            TerminatorKind::Resume => Key("Resume"),
            TerminatorKind::Return => Key("Return"),
            TerminatorKind::SwitchInt { .. } => Key("SwitchInt"),
            TerminatorKind::Unreachable => Key("Unreachable"),
            TerminatorKind::Yield { .. } => Key("Yield"),
        }
    }
}

impl<'tcx> From<&Rvalue<'tcx>> for Key {
    fn from(value: &Rvalue<'tcx>) -> Self {
        match value {
            Rvalue::Use(_) => Key("Use"),
            Rvalue::Repeat(_, _) => Key("Repeat"),
            Rvalue::Ref(_, _, _) => Key("Ref"),
            Rvalue::ThreadLocalRef(_) => Key("ThreadLocalRef"),
            Rvalue::AddressOf(_, _) => Key("AddressOf"),
            Rvalue::Len(_) => Key("Len"),
            Rvalue::Cast(_, _, _) => Key("Cast"),
            Rvalue::BinaryOp(_, _) => Key("BinaryOp"),
            Rvalue::CheckedBinaryOp(_, _) => Key("CheckedBinaryOp"),
            Rvalue::NullaryOp(_, _) => Key("NullaryOp"),
            Rvalue::UnaryOp(_, _) => Key("UnaryOp"),
            Rvalue::Discriminant(_) => Key("Discriminant"),
            Rvalue::Aggregate(_, _) => Key("Aggregate"),
            Rvalue::ShallowInitBox(_, _) => Key("ShallowInitBox"),
            Rvalue::CopyForDeref(_) => Key("CopyForDeref"),
        }
    }
}
