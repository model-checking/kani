// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MIR to Strata translation

use crate::codegen_strata::strata_builder::StrataBuilder;
use rustc_middle::mir::*;
use rustc_middle::ty::{Ty, TyCtxt, TyKind};
use std::collections::{HashMap, HashSet};

pub struct MirToStrata<'tcx> {
    tcx: TyCtxt<'tcx>,
    builder: StrataBuilder,
    local_names: HashMap<Local, String>,
    loop_headers: HashSet<BasicBlock>,
}

impl<'tcx> MirToStrata<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            builder: StrataBuilder::new(),
            local_names: HashMap::new(),
            loop_headers: HashSet::new(),
        }
    }

    pub fn translate_body(&mut self, body: &Body<'tcx>, fn_name: &str) {
        // Initialize local variable names
        for (local, decl) in body.local_decls.iter_enumerated() {
            let name = format!("_{}", local.as_u32());
            self.local_names.insert(local, name);
        }

        // Detect loops (basic blocks with back-edges)
        self.detect_loops(body);

        let mut body_str = String::new();

        // Declare local variables
        for (local, decl) in body.local_decls.iter_enumerated() {
            if local.as_u32() > 0 { // Skip return place
                let ty = self.translate_type(decl.ty);
                let name = &self.local_names[&local];
                body_str.push_str(&format!("  var {} : {};\n", name, ty));
            }
        }
        body_str.push('\n');

        // Translate basic blocks
        for (bb, data) in body.basic_blocks.iter_enumerated() {
            // Add loop invariant comment if this is a loop header
            if self.loop_headers.contains(&bb) {
                body_str.push_str(&format!("  // Loop header bb{}\n", bb.as_u32()));
                body_str.push_str("  // invariant: (add loop invariant here)\n");
            }

            body_str.push_str(&format!("  // bb{}\n", bb.as_u32()));

            for stmt in &data.statements {
                body_str.push_str(&self.translate_statement(stmt));
            }

            if let Some(term) = &data.terminator {
                body_str.push_str(&self.translate_terminator(term));
            }
            body_str.push('\n');
        }

        let params = vec![];
        let returns = vec![];
        self.builder.add_procedure(fn_name, &params, &returns, &body_str);
    }

    fn detect_loops(&mut self, body: &Body<'tcx>) {
        // Find basic blocks that have predecessors with higher indices (back-edges)
        for (bb, data) in body.basic_blocks.iter_enumerated() {
            if let Some(term) = &data.terminator {
                match &term.kind {
                    TerminatorKind::Goto { target } => {
                        if target.as_u32() <= bb.as_u32() {
                            self.loop_headers.insert(*target);
                        }
                    }
                    TerminatorKind::SwitchInt { targets, .. } => {
                        for target in targets.all_targets() {
                            if target.as_u32() <= bb.as_u32() {
                                self.loop_headers.insert(*target);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn translate_type(&self, ty: Ty<'tcx>) -> String {
        match ty.kind() {
            TyKind::Bool => "bool".to_string(),
            TyKind::Int(int_ty) => {
                use rustc_middle::ty::IntTy;
                match int_ty {
                    IntTy::I8 => "bv8".to_string(),
                    IntTy::I16 => "bv16".to_string(),
                    IntTy::I32 => "bv32".to_string(),
                    IntTy::I64 => "bv64".to_string(),
                    IntTy::I128 => "bv128".to_string(),
                    IntTy::Isize => "bv64".to_string(),
                }
            }
            TyKind::Uint(uint_ty) => {
                use rustc_middle::ty::UintTy;
                match uint_ty {
                    UintTy::U8 => "bv8".to_string(),
                    UintTy::U16 => "bv16".to_string(),
                    UintTy::U32 => "bv32".to_string(),
                    UintTy::U64 => "bv64".to_string(),
                    UintTy::U128 => "bv128".to_string(),
                    UintTy::Usize => "bv64".to_string(),
                }
            }
            TyKind::Tuple(fields) if fields.is_empty() => "unit".to_string(),
            TyKind::Array(elem_ty, _) => {
                // Array type: [T; N] -> map from int to T
                let elem_type = self.translate_type(*elem_ty);
                format!("[int]{}", elem_type)
            }
            TyKind::Ref(_, inner_ty, _) | TyKind::RawPtr(inner_ty, _) => {
                // Reference or pointer - represent as pointer to inner type
                let inner = self.translate_type(*inner_ty);
                format!("Ref_{}", inner)
            }
            TyKind::Adt(adt_def, _) => {
                if adt_def.is_enum() {
                    "int".to_string() // Represent enum as integer discriminant
                } else if adt_def.is_struct() {
                    // Struct - represent as record type
                    let struct_name = self.tcx.def_path_str(adt_def.did());
                    format!("Struct_{}", struct_name.replace("::", "_"))
                } else {
                    format!("/* adt {} */", adt_def.did())
                }
            }
            _ => "int".to_string(), // fallback
        }
    }

    fn translate_place(&self, place: &Place<'tcx>) -> String {
        let mut result = self.local_names.get(&place.local)
            .cloned()
            .unwrap_or_else(|| format!("_{}", place.local.as_u32()));

        // Handle projections (field access, array indexing, etc.)
        for proj in place.projection.iter() {
            match proj {
                PlaceElem::Index(local) => {
                    // Array indexing: arr[i]
                    let index = self.local_names.get(local)
                        .cloned()
                        .unwrap_or_else(|| format!("_{}", local.as_u32()));
                    result = format!("{}[{}]", result, index);
                }
                PlaceElem::ConstantIndex { offset, .. } => {
                    // Constant array index: arr[0]
                    result = format!("{}[{}]", result, offset);
                }
                PlaceElem::Field(field, _) => {
                    // Struct field access: s.field
                    result = format!("{}.{}", result, field.as_u32());
                }
                PlaceElem::Deref => {
                    // Pointer dereference: *ptr
                    result = format!("(*{})", result);
                }
                _ => {
                    // Other projections
                    result = format!("{}/* {:?} */", result, proj);
                }
            }
        }

        result
    }

    fn translate_operand(&self, operand: &Operand<'tcx>) -> String {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.translate_place(place),
            Operand::Constant(constant) => {
                self.translate_constant(constant)
            }
        }
    }

    fn translate_constant(&self, constant: &ConstOperand<'tcx>) -> String {
        use rustc_middle::mir::Const;
        use rustc_middle::ty::ScalarInt;

        match constant.const_ {
            Const::Val(const_val, ty) => {
                // Try to extract the actual value
                match ty.kind() {
                    TyKind::Bool => {
                        if let Ok(scalar) = const_val.try_to_scalar() {
                            if scalar.to_bool().unwrap_or(false) {
                                return "true".to_string();
                            } else {
                                return "false".to_string();
                            }
                        }
                    }
                    TyKind::Int(_) | TyKind::Uint(_) => {
                        if let Ok(scalar) = const_val.try_to_scalar() {
                            if let Ok(int) = scalar.try_to_int() {
                                return int.to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Fallback: use debug format
        format!("{:?}", constant.const_)
    }

    fn translate_rvalue(&self, rvalue: &Rvalue<'tcx>) -> String {
        match rvalue {
            Rvalue::Use(operand) => self.translate_operand(operand),

            Rvalue::BinaryOp(bin_op, box (left, right)) |
            Rvalue::CheckedBinaryOp(bin_op, box (left, right)) => {
                let left_str = self.translate_operand(left);
                let right_str = self.translate_operand(right);
                let op_str = match bin_op {
                    BinOp::Add | BinOp::AddUnchecked => "+",
                    BinOp::Sub | BinOp::SubUnchecked => "-",
                    BinOp::Mul | BinOp::MulUnchecked => "*",
                    BinOp::Div => "/",
                    BinOp::Rem => "%",
                    BinOp::BitAnd => "&",
                    BinOp::BitOr => "|",
                    BinOp::BitXor => "^",
                    BinOp::Shl | BinOp::ShlUnchecked => "<<",
                    BinOp::Shr | BinOp::ShrUnchecked => ">>",
                    BinOp::Eq => "==",
                    BinOp::Ne => "!=",
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    BinOp::Ge => ">=",
                    _ => "?",
                };
                format!("({} {} {})", left_str, op_str, right_str)
            }

            Rvalue::UnaryOp(un_op, operand) => {
                let operand_str = self.translate_operand(operand);
                let op_str = match un_op {
                    UnOp::Not => "!",
                    UnOp::Neg => "-",
                    _ => "?",
                };
                format!("({}{})", op_str, operand_str)
            }

            Rvalue::Discriminant(place) => {
                // Get enum discriminant
                let place_str = self.translate_place(place);
                format!("discriminant({})", place_str)
            }

            Rvalue::Aggregate(box kind, operands) => {
                match kind {
                    AggregateKind::Adt(adt_def, variant_idx, _, _, _) => {
                        if adt_def.is_enum() {
                            // Enum variant construction - use discriminant
                            format!("{}", variant_idx.as_u32())
                        } else {
                            // Struct construction
                            let fields: Vec<String> = operands.iter()
                                .map(|op| self.translate_operand(&op.node))
                                .collect();
                            format!("{{ {} }}", fields.join(", "))
                        }
                    }
                    AggregateKind::Array(_) => {
                        // Array literal: [1, 2, 3]
                        let elements: Vec<String> = operands.iter()
                            .map(|op| self.translate_operand(&op.node))
                            .collect();
                        format!("[{}]", elements.join(", "))
                    }
                    AggregateKind::Tuple => {
                        // Tuple construction: (a, b, c)
                        let elements: Vec<String> = operands.iter()
                            .map(|op| self.translate_operand(&op.node))
                            .collect();
                        format!("({})", elements.join(", "))
                    }
                    _ => format!("/* aggregate {:?} */", kind),
                }
            }

            Rvalue::Len(place) => {
                // Array length
                let place_str = self.translate_place(place);
                format!("len({})", place_str)
            }

            Rvalue::Ref(_, _, place) => {
                // Create reference: &x or &mut x
                let place_str = self.translate_place(place);
                format!("ref({})", place_str)
            }

            Rvalue::AddressOf(_, place) => {
                // Raw pointer: &raw const x or &raw mut x
                let place_str = self.translate_place(place);
                format!("addr({})", place_str)
            }

            _ => format!("/* {:?} */", rvalue),
        }
    }

    fn translate_statement(&self, stmt: &Statement<'tcx>) -> String {
        match &stmt.kind {
            StatementKind::Assign(box (place, rvalue)) => {
                let lhs = self.translate_place(place);
                let rhs = self.translate_rvalue(rvalue);
                format!("  {} := {};\n", lhs, rhs)
            }
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {
                String::new()
            }
            _ => format!("  // {:?};\n", stmt.kind),
        }
    }

    fn translate_terminator(&self, term: &Terminator<'tcx>) -> String {
        match &term.kind {
            TerminatorKind::Return => "  return;\n".to_string(),

            TerminatorKind::Goto { target } => {
                format!("  goto bb{};\n", target.as_u32())
            }

            TerminatorKind::SwitchInt { discr, targets } => {
                let mut result = String::new();
                let discr_str = self.translate_operand(discr);

                for (value, target) in targets.iter() {
                    result.push_str(&format!("  if ({} == {}) {{ goto bb{}; }}\n",
                        discr_str, value, target.as_u32()));
                }
                result.push_str(&format!("  goto bb{};\n", targets.otherwise().as_u32()));
                result
            }

            TerminatorKind::Assert { cond, expected, target, .. } => {
                let cond_str = self.translate_operand(cond);
                let assertion = if *expected {
                    cond_str
                } else {
                    format!("!({})", cond_str)
                };
                format!("  assert {};\n  goto bb{};\n", assertion, target.as_u32())
            }

            TerminatorKind::Call { func, args, destination, target, .. } => {
                let mut result = String::new();

                // Get function name
                let func_name = self.get_function_name(func);

                // Translate arguments
                let arg_strs: Vec<String> = args.iter()
                    .map(|arg| self.translate_operand(&arg.node))
                    .collect();

                // Generate call
                if let Some(dest) = destination {
                    let dest_str = self.translate_place(dest);
                    result.push_str(&format!("  call {} := {}({});\n",
                        dest_str, func_name, arg_strs.join(", ")));
                } else {
                    result.push_str(&format!("  call {}({});\n",
                        func_name, arg_strs.join(", ")));
                }

                // Continue to next block
                if let Some(target) = target {
                    result.push_str(&format!("  goto bb{};\n", target.as_u32()));
                }

                result
            }

            _ => format!("  // {:?};\n", term.kind),
        }
    }

    fn get_function_name(&self, func: &Operand<'tcx>) -> String {
        match func {
            Operand::Constant(constant) => {
                // Try to extract function name from constant
                let func_str = format!("{:?}", constant.const_);

                // Check for Kani intrinsics
                if func_str.contains("kani::any") {
                    return "havoc".to_string();
                }
                if func_str.contains("kani::assume") {
                    return "assume".to_string();
                }

                // Extract function name (simplified)
                if let Some(start) = func_str.find("fn(") {
                    if let Some(name_start) = func_str[..start].rfind(' ') {
                        let name = &func_str[name_start + 1..start];
                        return name.trim_matches(|c| c == '{' || c == '}').to_string();
                    }
                }

                // Fallback: use debug representation
                func_str
            }
            _ => format!("{:?}", func),
        }
    }

    pub fn finish(self) -> String {
        self.builder.build()
    }
}
