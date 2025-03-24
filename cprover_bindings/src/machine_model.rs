// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Represents the machine specific information necessary to generate an Irep.
use num::bigint::BigInt;
#[derive(Clone, Debug)]
pub struct MachineModel {
    /// Is the architecture big endian?
    /// Minimum architectural alignment, in bytes
    /// The name of the architecture
    /// Width of a pointer, in bits
    pub alignment: u64,
    pub architecture: String,
    pub bool_width: u64,
    pub char_is_unsigned: bool,
    pub char_width: u64,
    pub double_width: u64,
    pub float_width: u64,
    pub int_width: u64,
    pub is_big_endian: bool,
    pub long_double_width: u64,
    pub long_int_width: u64,
    pub long_long_int_width: u64,
    pub memory_operand_size: u64,
    pub null_is_zero: bool,
    pub pointer_width: u64,
    pub rounding_mode: RoundingMode,
    pub short_int_width: u64,
    pub single_width: u64,
    pub wchar_t_is_unsigned: bool,
    pub wchar_t_width: u64,
    pub word_size: u64,
}

impl MachineModel {
    pub fn pointer_width_in_bytes(&self) -> usize {
        self.pointer_width as usize / 8
    }
}

/// The different rounding modes supported by cbmc.
/// <https://github.com/diffblue/cbmc/blob/2bc93c24ea6c09b5fc99b31df682ec5b31c4b162/src/ansi-c/library/fenv.c#L7>
#[derive(Clone, Copy, Debug)]
pub enum RoundingMode {
    ToNearest = 0,
    Downward = 1,
    Upward = 2,
    TowardsZero = 3,
}

impl From<RoundingMode> for BigInt {
    fn from(rm: RoundingMode) -> Self {
        (rm as i32).into()
    }
}

impl From<RoundingMode> for i32 {
    fn from(rm: RoundingMode) -> Self {
        rm as Self
    }
}

impl From<RoundingMode> for i128 {
    fn from(rm: RoundingMode) -> Self {
        rm as Self
    }
}

#[cfg(test)]
pub mod test_util {
    use super::MachineModel;
    use super::RoundingMode;

    pub fn machine_model_test_stub() -> MachineModel {
        MachineModel {
            alignment: 1,
            architecture: "x86_64".to_string(),
            bool_width: 8,
            char_is_unsigned: false,
            char_width: 8,
            double_width: 64,
            float_width: 32,
            int_width: 32,
            is_big_endian: false,
            long_double_width: 128,
            long_int_width: 64,
            long_long_int_width: 64,
            memory_operand_size: 4,
            null_is_zero: true,
            pointer_width: 64,
            rounding_mode: RoundingMode::ToNearest,
            short_int_width: 16,
            single_width: 32,
            wchar_t_is_unsigned: false,
            wchar_t_width: 32,
            word_size: 32,
        }
    }
}
