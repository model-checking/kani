// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::escrow::*;
use crate::fsm::*;
use crate::oracle::*;

#[kani::proof]
fn timelock_policy_matches_release_rule() {
    let caller: Key = kani::any();
    let agent: Key = kani::any();
    let api: Key = kani::any();
    let now: i64 = kani::any();
    let expires_at: i64 = kani::any();

    let allowed = can_release_funds(caller, agent, api, now, expires_at);

    if now < expires_at {
        kani::assert(
            !allowed || caller == agent,
            "only agent can release before expiry",
        );
    } else {
        kani::assert(
            !allowed || (caller == agent || caller == api),
            "only agent/api can release after expiry",
        );
    }
}

#[kani::proof]
fn settlement_splits_conserve_value() {
    let amount: u64 = kani::any::<u32>() as u64;

    let refund_percentage: u8 = kani::any();
    kani::assume(refund_percentage <= 100);

    let (refund, payment) = dispute_settlement(amount, refund_percentage);
    kani::assert(
        refund as u128 + payment as u128 == amount as u128,
        "dispute settlement must conserve value",
    );

    let quality_threshold: u8 = kani::any();
    let quality_score: u8 = kani::any();
    kani::assume(quality_threshold <= 100);
    kani::assume(quality_score <= 100);

    let (user_refund, provider_payment) =
        inference_settlement(amount, quality_threshold, quality_score);
    kani::assert(
        user_refund as u128 + provider_payment as u128 == amount as u128,
        "inference settlement must conserve value",
    );

    let was_disputed: bool = kani::any();
    let (agent_amount, api_amount) = expired_escrow_settlement(amount, was_disputed);
    kani::assert(
        agent_amount as u128 + api_amount as u128 == amount as u128,
        "expired escrow claim must conserve value",
    );
}

#[kani::proof]
fn required_oracle_count_is_monotonic_and_bounded() {
    let amount: u64 = kani::any();
    let r = required_oracle_count(amount);
    kani::assert(
        r == MIN_ORACLES || r == 4 || r == 5,
        "required oracle count must be in {3,4,5}",
    );

    let a1: u64 = kani::any();
    let a2: u64 = kani::any();
    kani::assume(a1 <= a2);

    let r1 = required_oracle_count(a1);
    let r2 = required_oracle_count(a2);
    kani::assert(r1 <= r2, "oracle requirement must be monotonic");

    kani::assert(
        required_oracle_count(TIER2_ESCROW_THRESHOLD - 1) == MIN_ORACLES,
        "below tier2 threshold must use minimum",
    );
    kani::assert(
        required_oracle_count(TIER2_ESCROW_THRESHOLD) == 4,
        "tier2 threshold must require 4 oracles",
    );
    kani::assert(
        required_oracle_count(TIER3_ESCROW_THRESHOLD - 1) == 4,
        "just below tier3 threshold must still be tier2",
    );
    kani::assert(
        required_oracle_count(TIER3_ESCROW_THRESHOLD) == 5,
        "tier3 threshold must require 5 oracles",
    );
}

fn any_state() -> EscrowState {
    match kani::any::<u8>() % 4 {
        0 => EscrowState::Active,
        1 => EscrowState::Disputed,
        2 => EscrowState::Released,
        _ => EscrowState::Resolved,
    }
}

fn any_action() -> EscrowAction {
    match kani::any::<u8>() % 4 {
        0 => EscrowAction::Release,
        1 => EscrowAction::MarkDisputed,
        2 => EscrowAction::Resolve,
        _ => EscrowAction::ClaimExpired,
    }
}

#[kani::proof]
fn escrow_fsm_actions_respect_transition_table() {
    let state = any_state();
    let action = any_action();

    let next = step(state, action);

    if is_terminal(state) {
        kani::assert(next.is_none(), "terminal states must not transition");
        return;
    }

    if let Some(s2) = next {
        kani::assert(valid_transition(state, s2), "transition must be valid");
    }
}
