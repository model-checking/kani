// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Represents the machine specific information necessary to generate an Irep.
#[derive(Debug)]
pub struct MachineModel {
    /// Is the architecture big endian?
    /// Minimum architectural alignment, in bytes
    /// The name of the architecture
    /// Width of a pointer, in bits
    alignment: u64,
    architecture: String,
    bool_width: u64,
    char_is_unsigned: bool,
    char_width: u64,
    double_width: u64,
    float_width: u64,
    int_width: u64,
    is_big_endian: bool,
    long_double_width: u64,
    long_int_width: u64,
    long_long_int_width: u64,
    memory_operand_size: u64,
    null_is_zero: bool,
    pointer_width: u64,
    rounding_mode: RoundingMode,
    short_int_width: u64,
    single_width: u64,
    wchar_t_is_unsigned: bool,
    wchar_t_width: u64,
    word_size: u64,
}
/// The different rounding modes supported by cbmc.
/// https://github.com/diffblue/cbmc/blob/2bc93c24ea6c09b5fc99b31df682ec5b31c4b162/src/ansi-c/library/fenv.c#L7
#[derive(Clone, Copy, Debug)]
pub enum RoundingMode {
    ToNearest = 0,
    Downward = 1,
    Upward = 2,
    TowardsZero = 3,
}

/// Constructor
impl MachineModel {
    pub fn new(
        alignment: u64,
        architecture: &str,
        bool_width: u64,
        char_is_unsigned: bool,
        char_width: u64,
        double_width: u64,
        float_width: u64,
        int_width: u64,
        is_big_endian: bool,
        long_double_width: u64,
        long_int_width: u64,
        long_long_int_width: u64,
        memory_operand_size: u64,
        null_is_zero: bool,
        pointer_width: u64,
        rounding_mode: RoundingMode,
        short_int_width: u64,
        single_width: u64,
        wchar_t_is_unsigned: bool,
        wchar_t_width: u64,
        word_size: u64,
    ) -> Self {
        MachineModel {
            alignment,
            architecture: architecture.to_string(),
            bool_width,
            char_is_unsigned,
            char_width,
            double_width,
            float_width,
            int_width,
            is_big_endian,
            long_double_width,
            long_int_width,
            long_long_int_width,
            memory_operand_size,
            null_is_zero,
            pointer_width,
            rounding_mode,
            short_int_width,
            single_width,
            wchar_t_is_unsigned,
            wchar_t_width,
            word_size,
        }
    }
}

/// Getters
impl MachineModel {
    pub fn alignment(&self) -> u64 {
        self.alignment
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    pub fn bool_width(&self) -> u64 {
        self.bool_width
    }

    pub fn char_is_unsigned(&self) -> bool {
        self.char_is_unsigned
    }

    pub fn char_width(&self) -> u64 {
        self.char_width
    }

    pub fn double_width(&self) -> u64 {
        self.double_width
    }

    pub fn float_width(&self) -> u64 {
        self.float_width
    }

    pub fn int_width(&self) -> u64 {
        self.int_width
    }

    pub fn is_big_endian(&self) -> bool {
        self.is_big_endian
    }

    pub fn long_double_width(&self) -> u64 {
        self.long_double_width
    }

    pub fn long_int_width(&self) -> u64 {
        self.long_int_width
    }

    pub fn long_long_int_width(&self) -> u64 {
        self.long_long_int_width
    }

    pub fn memory_operand_size(&self) -> u64 {
        self.memory_operand_size
    }

    pub fn null_is_zero(&self) -> bool {
        self.null_is_zero
    }

    pub fn pointer_width(&self) -> u64 {
        self.pointer_width
    }

    pub fn rounding_mode(&self) -> RoundingMode {
        self.rounding_mode
    }

    pub fn short_int_width(&self) -> u64 {
        self.short_int_width
    }

    pub fn single_width(&self) -> u64 {
        self.single_width
    }

    pub fn wchar_t_is_unsigned(&self) -> bool {
        self.wchar_t_is_unsigned
    }

    pub fn wchar_t_width(&self) -> u64 {
        self.wchar_t_width
    }

    pub fn word_size(&self) -> u64 {
        self.word_size
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
