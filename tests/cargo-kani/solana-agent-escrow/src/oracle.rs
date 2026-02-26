// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub const MIN_ORACLES: u8 = 3;
pub const TIER2_ESCROW_THRESHOLD: u64 = 10;
pub const TIER3_ESCROW_THRESHOLD: u64 = 100;

pub fn required_oracle_count(escrow_amount: u64) -> u8 {
    if escrow_amount >= TIER3_ESCROW_THRESHOLD {
        5
    } else if escrow_amount >= TIER2_ESCROW_THRESHOLD {
        4
    } else {
        MIN_ORACLES
    }
}
