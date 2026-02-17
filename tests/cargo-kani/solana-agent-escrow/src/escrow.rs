// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub type Key = [u8; 32];

pub fn can_release_funds(caller: Key, agent: Key, api: Key, now: i64, expires_at: i64) -> bool {
    let is_agent = caller == agent;
    let is_api = caller == api;
    let time_lock_expired = now >= expires_at;

    is_agent || (is_api && time_lock_expired)
}

pub fn dispute_settlement(amount: u64, refund_percentage: u8) -> (u64, u64) {
    let refund_amount = (amount as u128 * refund_percentage as u128 / 100) as u64;
    let payment_amount = amount - refund_amount;
    (refund_amount, payment_amount)
}

pub fn inference_settlement(amount: u64, quality_threshold: u8, quality_score: u8) -> (u64, u64) {
    if quality_score >= quality_threshold {
        return (0, amount);
    }

    if quality_score >= 50 {
        let provider_share = (amount as u128 * quality_score as u128 / 100) as u64;
        let user_refund = amount - provider_share;
        return (user_refund, provider_share);
    }

    (amount, 0)
}

pub fn expired_escrow_settlement(amount: u64, was_disputed: bool) -> (u64, u64) {
    if !was_disputed {
        return (amount, 0);
    }

    let half = amount / 2;
    (half, amount - half)
}
