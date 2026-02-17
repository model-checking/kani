// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscrowState {
    Active,
    Disputed,
    Released,
    Resolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscrowAction {
    Release,
    MarkDisputed,
    Resolve,
    ClaimExpired,
}

pub fn is_terminal(state: EscrowState) -> bool {
    matches!(state, EscrowState::Released | EscrowState::Resolved)
}

pub fn valid_transition(from: EscrowState, to: EscrowState) -> bool {
    matches!(
        (from, to),
        (EscrowState::Active, EscrowState::Disputed)
            | (EscrowState::Active, EscrowState::Released)
            | (EscrowState::Active, EscrowState::Resolved)
            | (EscrowState::Disputed, EscrowState::Resolved)
    )
}

pub fn step(state: EscrowState, action: EscrowAction) -> Option<EscrowState> {
    if is_terminal(state) {
        return None;
    }

    match (state, action) {
        (EscrowState::Active, EscrowAction::Release) => Some(EscrowState::Released),
        (EscrowState::Active, EscrowAction::MarkDisputed) => Some(EscrowState::Disputed),
        (EscrowState::Active, EscrowAction::Resolve) => Some(EscrowState::Resolved),
        (EscrowState::Active, EscrowAction::ClaimExpired) => Some(EscrowState::Resolved),
        (EscrowState::Disputed, EscrowAction::Resolve) => Some(EscrowState::Resolved),
        (EscrowState::Disputed, EscrowAction::ClaimExpired) => Some(EscrowState::Resolved),
        _ => None,
    }
}
