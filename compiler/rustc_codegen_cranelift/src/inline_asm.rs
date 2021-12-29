//! Codegen of `asm!` invocations.

use crate::prelude::*;

use std::fmt::Write;

use rustc_ast::ast::{InlineAsmOptions, InlineAsmTemplatePiece};
use rustc_middle::mir::InlineAsmOperand;
use rustc_span::Symbol;
use rustc_target::asm::*;

pub(crate) fn codegen_inline_asm<'tcx>(
    fx: &mut FunctionCx<'_, '_, 'tcx>,
    _span: Span,
    template: &[InlineAsmTemplatePiece],
    operands: &[InlineAsmOperand<'tcx>],
    options: InlineAsmOptions,
) {
    // FIXME add .eh_frame unwind info directives

    if template[0] == InlineAsmTemplatePiece::String("int $$0x29".to_string()) {
        let true_ = fx.bcx.ins().iconst(types::I32, 1);
        fx.bcx.ins().trapnz(true_, TrapCode::User(1));
        return;
    } else if template[0] == InlineAsmTemplatePiece::String("movq %rbx, ".to_string())
        && matches!(
            template[1],
            InlineAsmTemplatePiece::Placeholder { operand_idx: 0, modifier: Some('r'), span: _ }
        )
        && template[2] == InlineAsmTemplatePiece::String("\n".to_string())
        && template[3] == InlineAsmTemplatePiece::String("cpuid".to_string())
        && template[4] == InlineAsmTemplatePiece::String("\n".to_string())
        && template[5] == InlineAsmTemplatePiece::String("xchgq %rbx, ".to_string())
        && matches!(
            template[6],
            InlineAsmTemplatePiece::Placeholder { operand_idx: 0, modifier: Some('r'), span: _ }
        )
    {
        assert_eq!(operands.len(), 4);
        let (leaf, eax_place) = match operands[1] {
            InlineAsmOperand::InOut { reg, late: true, ref in_value, out_place } => {
                assert_eq!(
                    reg,
                    InlineAsmRegOrRegClass::Reg(InlineAsmReg::X86(X86InlineAsmReg::ax))
                );
                (
                    crate::base::codegen_operand(fx, in_value).load_scalar(fx),
                    crate::base::codegen_place(fx, out_place.unwrap()),
                )
            }
            _ => unreachable!(),
        };
        let ebx_place = match operands[0] {
            InlineAsmOperand::Out { reg, late: true, place } => {
                assert_eq!(
                    reg,
                    InlineAsmRegOrRegClass::RegClass(InlineAsmRegClass::X86(
                        X86InlineAsmRegClass::reg
                    ))
                );
                crate::base::codegen_place(fx, place.unwrap())
            }
            _ => unreachable!(),
        };
        let (sub_leaf, ecx_place) = match operands[2] {
            InlineAsmOperand::InOut { reg, late: true, ref in_value, out_place } => {
                assert_eq!(
                    reg,
                    InlineAsmRegOrRegClass::Reg(InlineAsmReg::X86(X86InlineAsmReg::cx))
                );
                (
                    crate::base::codegen_operand(fx, in_value).load_scalar(fx),
                    crate::base::codegen_place(fx, out_place.unwrap()),
                )
            }
            _ => unreachable!(),
        };
        let edx_place = match operands[3] {
            InlineAsmOperand::Out { reg, late: true, place } => {
                assert_eq!(
                    reg,
                    InlineAsmRegOrRegClass::Reg(InlineAsmReg::X86(X86InlineAsmReg::dx))
                );
                crate::base::codegen_place(fx, place.unwrap())
            }
            _ => unreachable!(),
        };

        let (eax, ebx, ecx, edx) = crate::intrinsics::codegen_cpuid_call(fx, leaf, sub_leaf);

        eax_place.write_cvalue(fx, CValue::by_val(eax, fx.layout_of(fx.tcx.types.u32)));
        ebx_place.write_cvalue(fx, CValue::by_val(ebx, fx.layout_of(fx.tcx.types.u32)));
        ecx_place.write_cvalue(fx, CValue::by_val(ecx, fx.layout_of(fx.tcx.types.u32)));
        edx_place.write_cvalue(fx, CValue::by_val(edx, fx.layout_of(fx.tcx.types.u32)));
        return;
    } else if fx.tcx.symbol_name(fx.instance).name.starts_with("___chkstk") {
        // ___chkstk, ___chkstk_ms and __alloca are only used on Windows
        crate::trap::trap_unimplemented(fx, "Stack probes are not supported");
    } else if fx.tcx.symbol_name(fx.instance).name == "__alloca" {
        crate::trap::trap_unimplemented(fx, "Alloca is not supported");
    }

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    let mut asm_gen = InlineAssemblyGenerator {
        tcx: fx.tcx,
        arch: fx.tcx.sess.asm_arch.unwrap(),
        template,
        operands,
        options,
        registers: Vec::new(),
        stack_slots_clobber: Vec::new(),
        stack_slots_input: Vec::new(),
        stack_slots_output: Vec::new(),
        stack_slot_size: Size::from_bytes(0),
    };
    asm_gen.allocate_registers();
    asm_gen.allocate_stack_slots();

    let inline_asm_index = fx.cx.inline_asm_index.get();
    fx.cx.inline_asm_index.set(inline_asm_index + 1);
    let asm_name = format!(
        "__inline_asm_{}_n{}",
        fx.cx.cgu_name.as_str().replace('.', "__").replace('-', "_"),
        inline_asm_index
    );

    let generated_asm = asm_gen.generate_asm_wrapper(&asm_name);
    fx.cx.global_asm.push_str(&generated_asm);

    for (i, operand) in operands.iter().enumerate() {
        match *operand {
            InlineAsmOperand::In { reg: _, ref value } => {
                inputs.push((
                    asm_gen.stack_slots_input[i].unwrap(),
                    crate::base::codegen_operand(fx, value).load_scalar(fx),
                ));
            }
            InlineAsmOperand::Out { reg: _, late: _, place } => {
                if let Some(place) = place {
                    outputs.push((
                        asm_gen.stack_slots_output[i].unwrap(),
                        crate::base::codegen_place(fx, place),
                    ));
                }
            }
            InlineAsmOperand::InOut { reg: _, late: _, ref in_value, out_place } => {
                inputs.push((
                    asm_gen.stack_slots_input[i].unwrap(),
                    crate::base::codegen_operand(fx, in_value).load_scalar(fx),
                ));
                if let Some(out_place) = out_place {
                    outputs.push((
                        asm_gen.stack_slots_output[i].unwrap(),
                        crate::base::codegen_place(fx, out_place),
                    ));
                }
            }
            InlineAsmOperand::Const { value: _ } => todo!(),
            InlineAsmOperand::SymFn { value: _ } => todo!(),
            InlineAsmOperand::SymStatic { def_id: _ } => todo!(),
        }
    }

    call_inline_asm(fx, &asm_name, asm_gen.stack_slot_size, inputs, outputs);
}

struct InlineAssemblyGenerator<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    arch: InlineAsmArch,
    template: &'a [InlineAsmTemplatePiece],
    operands: &'a [InlineAsmOperand<'tcx>],
    options: InlineAsmOptions,
    registers: Vec<Option<InlineAsmReg>>,
    stack_slots_clobber: Vec<Option<Size>>,
    stack_slots_input: Vec<Option<Size>>,
    stack_slots_output: Vec<Option<Size>>,
    stack_slot_size: Size,
}

impl<'tcx> InlineAssemblyGenerator<'_, 'tcx> {
    fn allocate_registers(&mut self) {
        let sess = self.tcx.sess;
        let map = allocatable_registers(
            self.arch,
            |feature| sess.target_features.contains(&Symbol::intern(feature)),
            &sess.target,
        );
        let mut allocated = FxHashMap::<_, (bool, bool)>::default();
        let mut regs = vec![None; self.operands.len()];

        // Add explicit registers to the allocated set.
        for (i, operand) in self.operands.iter().enumerate() {
            match *operand {
                InlineAsmOperand::In { reg: InlineAsmRegOrRegClass::Reg(reg), .. } => {
                    regs[i] = Some(reg);
                    allocated.entry(reg).or_default().0 = true;
                }
                InlineAsmOperand::Out {
                    reg: InlineAsmRegOrRegClass::Reg(reg), late: true, ..
                } => {
                    regs[i] = Some(reg);
                    allocated.entry(reg).or_default().1 = true;
                }
                InlineAsmOperand::Out { reg: InlineAsmRegOrRegClass::Reg(reg), .. }
                | InlineAsmOperand::InOut { reg: InlineAsmRegOrRegClass::Reg(reg), .. } => {
                    regs[i] = Some(reg);
                    allocated.insert(reg, (true, true));
                }
                _ => (),
            }
        }

        // Allocate out/inout/inlateout registers first because they are more constrained.
        for (i, operand) in self.operands.iter().enumerate() {
            match *operand {
                InlineAsmOperand::Out {
                    reg: InlineAsmRegOrRegClass::RegClass(class),
                    late: false,
                    ..
                }
                | InlineAsmOperand::InOut {
                    reg: InlineAsmRegOrRegClass::RegClass(class), ..
                } => {
                    let mut alloc_reg = None;
                    for &reg in &map[&class] {
                        let mut used = false;
                        reg.overlapping_regs(|r| {
                            if allocated.contains_key(&r) {
                                used = true;
                            }
                        });

                        if !used {
                            alloc_reg = Some(reg);
                            break;
                        }
                    }

                    let reg = alloc_reg.expect("cannot allocate registers");
                    regs[i] = Some(reg);
                    allocated.insert(reg, (true, true));
                }
                _ => (),
            }
        }

        // Allocate in/lateout.
        for (i, operand) in self.operands.iter().enumerate() {
            match *operand {
                InlineAsmOperand::In { reg: InlineAsmRegOrRegClass::RegClass(class), .. } => {
                    let mut alloc_reg = None;
                    for &reg in &map[&class] {
                        let mut used = false;
                        reg.overlapping_regs(|r| {
                            if allocated.get(&r).copied().unwrap_or_default().0 {
                                used = true;
                            }
                        });

                        if !used {
                            alloc_reg = Some(reg);
                            break;
                        }
                    }

                    let reg = alloc_reg.expect("cannot allocate registers");
                    regs[i] = Some(reg);
                    allocated.entry(reg).or_default().0 = true;
                }
                InlineAsmOperand::Out {
                    reg: InlineAsmRegOrRegClass::RegClass(class),
                    late: true,
                    ..
                } => {
                    let mut alloc_reg = None;
                    for &reg in &map[&class] {
                        let mut used = false;
                        reg.overlapping_regs(|r| {
                            if allocated.get(&r).copied().unwrap_or_default().1 {
                                used = true;
                            }
                        });

                        if !used {
                            alloc_reg = Some(reg);
                            break;
                        }
                    }

                    let reg = alloc_reg.expect("cannot allocate registers");
                    regs[i] = Some(reg);
                    allocated.entry(reg).or_default().1 = true;
                }
                _ => (),
            }
        }

        self.registers = regs;
    }

    fn allocate_stack_slots(&mut self) {
        let mut slot_size = Size::from_bytes(0);
        let mut slots_clobber = vec![None; self.operands.len()];
        let mut slots_input = vec![None; self.operands.len()];
        let mut slots_output = vec![None; self.operands.len()];

        let new_slot_fn = |slot_size: &mut Size, reg_class: InlineAsmRegClass| {
            let reg_size =
                reg_class.supported_types(self.arch).iter().map(|(ty, _)| ty.size()).max().unwrap();
            let align = rustc_target::abi::Align::from_bytes(reg_size.bytes()).unwrap();
            let offset = slot_size.align_to(align);
            *slot_size = offset + reg_size;
            offset
        };
        let mut new_slot = |x| new_slot_fn(&mut slot_size, x);

        // Allocate stack slots for saving clobbered registers
        let abi_clobber = InlineAsmClobberAbi::parse(
            self.arch,
            |feature| self.tcx.sess.target_features.contains(&Symbol::intern(feature)),
            &self.tcx.sess.target,
            Symbol::intern("C"),
        )
        .unwrap()
        .clobbered_regs();
        for (i, reg) in self.registers.iter().enumerate().filter_map(|(i, r)| r.map(|r| (i, r))) {
            let mut need_save = true;
            // If the register overlaps with a register clobbered by function call, then
            // we don't need to save it.
            for r in abi_clobber {
                r.overlapping_regs(|r| {
                    if r == reg {
                        need_save = false;
                    }
                });

                if !need_save {
                    break;
                }
            }

            if need_save {
                slots_clobber[i] = Some(new_slot(reg.reg_class()));
            }
        }

        // Allocate stack slots for inout
        for (i, operand) in self.operands.iter().enumerate() {
            match *operand {
                InlineAsmOperand::InOut { reg, out_place: Some(_), .. } => {
                    let slot = new_slot(reg.reg_class());
                    slots_input[i] = Some(slot);
                    slots_output[i] = Some(slot);
                }
                _ => (),
            }
        }

        let slot_size_before_input = slot_size;
        let mut new_slot = |x| new_slot_fn(&mut slot_size, x);

        // Allocate stack slots for input
        for (i, operand) in self.operands.iter().enumerate() {
            match *operand {
                InlineAsmOperand::In { reg, .. }
                | InlineAsmOperand::InOut { reg, out_place: None, .. } => {
                    slots_input[i] = Some(new_slot(reg.reg_class()));
                }
                _ => (),
            }
        }

        // Reset slot size to before input so that input and output operands can overlap
        // and save some memory.
        let slot_size_after_input = slot_size;
        slot_size = slot_size_before_input;
        let mut new_slot = |x| new_slot_fn(&mut slot_size, x);

        // Allocate stack slots for output
        for (i, operand) in self.operands.iter().enumerate() {
            match *operand {
                InlineAsmOperand::Out { reg, place: Some(_), .. } => {
                    slots_output[i] = Some(new_slot(reg.reg_class()));
                }
                _ => (),
            }
        }

        slot_size = slot_size.max(slot_size_after_input);

        self.stack_slots_clobber = slots_clobber;
        self.stack_slots_input = slots_input;
        self.stack_slots_output = slots_output;
        self.stack_slot_size = slot_size;
    }

    fn generate_asm_wrapper(&self, asm_name: &str) -> String {
        let mut generated_asm = String::new();
        writeln!(generated_asm, ".globl {}", asm_name).unwrap();
        writeln!(generated_asm, ".type {},@function", asm_name).unwrap();
        writeln!(generated_asm, ".section .text.{},\"ax\",@progbits", asm_name).unwrap();
        writeln!(generated_asm, "{}:", asm_name).unwrap();

        let is_x86 = matches!(self.arch, InlineAsmArch::X86 | InlineAsmArch::X86_64);

        if is_x86 {
            generated_asm.push_str(".intel_syntax noprefix\n");
        }
        Self::prologue(&mut generated_asm, self.arch);

        // Save clobbered registers
        if !self.options.contains(InlineAsmOptions::NORETURN) {
            for (reg, slot) in self
                .registers
                .iter()
                .zip(self.stack_slots_clobber.iter().copied())
                .filter_map(|(r, s)| r.zip(s))
            {
                Self::save_register(&mut generated_asm, self.arch, reg, slot);
            }
        }

        // Write input registers
        for (reg, slot) in self
            .registers
            .iter()
            .zip(self.stack_slots_input.iter().copied())
            .filter_map(|(r, s)| r.zip(s))
        {
            Self::restore_register(&mut generated_asm, self.arch, reg, slot);
        }

        if is_x86 && self.options.contains(InlineAsmOptions::ATT_SYNTAX) {
            generated_asm.push_str(".att_syntax\n");
        }

        // The actual inline asm
        for piece in self.template {
            match piece {
                InlineAsmTemplatePiece::String(s) => {
                    generated_asm.push_str(s);
                }
                InlineAsmTemplatePiece::Placeholder { operand_idx, modifier, span: _ } => {
                    if self.options.contains(InlineAsmOptions::ATT_SYNTAX) {
                        generated_asm.push('%');
                    }
                    self.registers[*operand_idx]
                        .unwrap()
                        .emit(&mut generated_asm, self.arch, *modifier)
                        .unwrap();
                }
            }
        }
        generated_asm.push('\n');

        if is_x86 && self.options.contains(InlineAsmOptions::ATT_SYNTAX) {
            generated_asm.push_str(".intel_syntax noprefix\n");
        }

        if !self.options.contains(InlineAsmOptions::NORETURN) {
            // Read output registers
            for (reg, slot) in self
                .registers
                .iter()
                .zip(self.stack_slots_output.iter().copied())
                .filter_map(|(r, s)| r.zip(s))
            {
                Self::save_register(&mut generated_asm, self.arch, reg, slot);
            }

            // Restore clobbered registers
            for (reg, slot) in self
                .registers
                .iter()
                .zip(self.stack_slots_clobber.iter().copied())
                .filter_map(|(r, s)| r.zip(s))
            {
                Self::restore_register(&mut generated_asm, self.arch, reg, slot);
            }

            Self::epilogue(&mut generated_asm, self.arch);
        } else {
            Self::epilogue_noreturn(&mut generated_asm, self.arch);
        }

        if is_x86 {
            generated_asm.push_str(".att_syntax\n");
        }
        writeln!(generated_asm, ".size {name}, .-{name}", name = asm_name).unwrap();
        generated_asm.push_str(".text\n");
        generated_asm.push_str("\n\n");

        generated_asm
    }

    fn prologue(generated_asm: &mut String, arch: InlineAsmArch) {
        match arch {
            InlineAsmArch::X86 => {
                generated_asm.push_str("    push ebp\n");
                generated_asm.push_str("    mov ebp,[esp+8]\n");
            }
            InlineAsmArch::X86_64 => {
                generated_asm.push_str("    push rbp\n");
                generated_asm.push_str("    mov rbp,rdi\n");
            }
            InlineAsmArch::RiscV32 => {
                generated_asm.push_str("    addi sp, sp, -8\n");
                generated_asm.push_str("    sw ra, 4(sp)\n");
                generated_asm.push_str("    sw s0, 0(sp)\n");
                generated_asm.push_str("    mv s0, a0\n");
            }
            InlineAsmArch::RiscV64 => {
                generated_asm.push_str("    addi sp, sp, -16\n");
                generated_asm.push_str("    sd ra, 8(sp)\n");
                generated_asm.push_str("    sd s0, 0(sp)\n");
                generated_asm.push_str("    mv s0, a0\n");
            }
            _ => unimplemented!("prologue for {:?}", arch),
        }
    }

    fn epilogue(generated_asm: &mut String, arch: InlineAsmArch) {
        match arch {
            InlineAsmArch::X86 => {
                generated_asm.push_str("    pop ebp\n");
                generated_asm.push_str("    ret\n");
            }
            InlineAsmArch::X86_64 => {
                generated_asm.push_str("    pop rbp\n");
                generated_asm.push_str("    ret\n");
            }
            InlineAsmArch::RiscV32 => {
                generated_asm.push_str("    lw s0, 0(sp)\n");
                generated_asm.push_str("    lw ra, 4(sp)\n");
                generated_asm.push_str("    addi sp, sp, 8\n");
                generated_asm.push_str("    ret\n");
            }
            InlineAsmArch::RiscV64 => {
                generated_asm.push_str("    ld s0, 0(sp)\n");
                generated_asm.push_str("    ld ra, 8(sp)\n");
                generated_asm.push_str("    addi sp, sp, 16\n");
                generated_asm.push_str("    ret\n");
            }
            _ => unimplemented!("epilogue for {:?}", arch),
        }
    }

    fn epilogue_noreturn(generated_asm: &mut String, arch: InlineAsmArch) {
        match arch {
            InlineAsmArch::X86 | InlineAsmArch::X86_64 => {
                generated_asm.push_str("    ud2\n");
            }
            InlineAsmArch::RiscV32 | InlineAsmArch::RiscV64 => {
                generated_asm.push_str("    ebreak\n");
            }
            _ => unimplemented!("epilogue_noreturn for {:?}", arch),
        }
    }

    fn save_register(
        generated_asm: &mut String,
        arch: InlineAsmArch,
        reg: InlineAsmReg,
        offset: Size,
    ) {
        match arch {
            InlineAsmArch::X86 => {
                write!(generated_asm, "    mov [ebp+0x{:x}], ", offset.bytes()).unwrap();
                reg.emit(generated_asm, InlineAsmArch::X86, None).unwrap();
                generated_asm.push('\n');
            }
            InlineAsmArch::X86_64 => {
                write!(generated_asm, "    mov [rbp+0x{:x}], ", offset.bytes()).unwrap();
                reg.emit(generated_asm, InlineAsmArch::X86_64, None).unwrap();
                generated_asm.push('\n');
            }
            InlineAsmArch::RiscV32 => {
                generated_asm.push_str("    sw ");
                reg.emit(generated_asm, InlineAsmArch::RiscV32, None).unwrap();
                writeln!(generated_asm, ", 0x{:x}(s0)", offset.bytes()).unwrap();
            }
            InlineAsmArch::RiscV64 => {
                generated_asm.push_str("    sd ");
                reg.emit(generated_asm, InlineAsmArch::RiscV64, None).unwrap();
                writeln!(generated_asm, ", 0x{:x}(s0)", offset.bytes()).unwrap();
            }
            _ => unimplemented!("save_register for {:?}", arch),
        }
    }

    fn restore_register(
        generated_asm: &mut String,
        arch: InlineAsmArch,
        reg: InlineAsmReg,
        offset: Size,
    ) {
        match arch {
            InlineAsmArch::X86 => {
                generated_asm.push_str("    mov ");
                reg.emit(generated_asm, InlineAsmArch::X86, None).unwrap();
                writeln!(generated_asm, ", [ebp+0x{:x}]", offset.bytes()).unwrap();
            }
            InlineAsmArch::X86_64 => {
                generated_asm.push_str("    mov ");
                reg.emit(generated_asm, InlineAsmArch::X86_64, None).unwrap();
                writeln!(generated_asm, ", [rbp+0x{:x}]", offset.bytes()).unwrap();
            }
            InlineAsmArch::RiscV32 => {
                generated_asm.push_str("    lw ");
                reg.emit(generated_asm, InlineAsmArch::RiscV32, None).unwrap();
                writeln!(generated_asm, ", 0x{:x}(s0)", offset.bytes()).unwrap();
            }
            InlineAsmArch::RiscV64 => {
                generated_asm.push_str("    ld ");
                reg.emit(generated_asm, InlineAsmArch::RiscV64, None).unwrap();
                writeln!(generated_asm, ", 0x{:x}(s0)", offset.bytes()).unwrap();
            }
            _ => unimplemented!("restore_register for {:?}", arch),
        }
    }
}

fn call_inline_asm<'tcx>(
    fx: &mut FunctionCx<'_, '_, 'tcx>,
    asm_name: &str,
    slot_size: Size,
    inputs: Vec<(Size, Value)>,
    outputs: Vec<(Size, CPlace<'tcx>)>,
) {
    let stack_slot = fx.bcx.func.create_stack_slot(StackSlotData {
        kind: StackSlotKind::ExplicitSlot,
        size: u32::try_from(slot_size.bytes()).unwrap(),
    });
    if fx.clif_comments.enabled() {
        fx.add_comment(stack_slot, "inline asm scratch slot");
    }

    let inline_asm_func = fx
        .module
        .declare_function(
            asm_name,
            Linkage::Import,
            &Signature {
                call_conv: CallConv::SystemV,
                params: vec![AbiParam::new(fx.pointer_type)],
                returns: vec![],
            },
        )
        .unwrap();
    let inline_asm_func = fx.module.declare_func_in_func(inline_asm_func, &mut fx.bcx.func);
    if fx.clif_comments.enabled() {
        fx.add_comment(inline_asm_func, asm_name);
    }

    for (offset, value) in inputs {
        fx.bcx.ins().stack_store(value, stack_slot, i32::try_from(offset.bytes()).unwrap());
    }

    let stack_slot_addr = fx.bcx.ins().stack_addr(fx.pointer_type, stack_slot, 0);
    fx.bcx.ins().call(inline_asm_func, &[stack_slot_addr]);

    for (offset, place) in outputs {
        let ty = fx.clif_type(place.layout().ty).unwrap();
        let value = fx.bcx.ins().stack_load(ty, stack_slot, i32::try_from(offset.bytes()).unwrap());
        place.write_cvalue(fx, CValue::by_val(value, place.layout()));
    }
}
