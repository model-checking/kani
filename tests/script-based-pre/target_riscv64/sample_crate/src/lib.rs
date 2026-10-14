// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A crate whose only harness exists when it is compiled for a riscv64 target.

/// Reads the RISC-V `cycle` counter. Kani does not model the instruction, so the harness below
/// stubs it; the point is that this code only compiles for riscv64.
#[cfg(target_arch = "riscv64")]
pub fn cycles() -> u64 {
    let value: u64;
    unsafe { core::arch::asm!("rdcycle {}", out(reg) value) };
    value
}

#[cfg(target_arch = "riscv64")]
pub fn elapsed(start: u64) -> u64 {
    cycles().wrapping_sub(start)
}

#[cfg(all(kani, target_arch = "riscv64"))]
mod verify {
    fn stub_cycles() -> u64 {
        kani::any()
    }

    #[kani::proof]
    #[kani::stub(super::cycles, stub_cycles)]
    fn elapsed_never_panics() {
        let start: u64 = kani::any();
        let _ = super::elapsed(start);
    }
}
