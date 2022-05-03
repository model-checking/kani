// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::virtio_defs::*;

#[derive(Clone, Copy)]
pub enum DescriptorPermission {
    ReadOnly,
    WriteOnly,
}

impl DescriptorPermission {
    pub fn from_flags(flags: u16) -> Self {
        if 0 != (flags & VIRTQ_DESC_F_WRITE) { Self::WriteOnly } else { Self::ReadOnly }
    }
}

// ANCHOR: fsm
#[derive(std::cmp::PartialEq, Clone, Copy)]
enum State {
    ReadOrWriteOk,
    OnlyWriteOk,
    Invalid,
}

/// State machine checker for virtio requirement 2.6.4.2
pub struct DescriptorPermissionChecker {
    state: State,
}

impl DescriptorPermissionChecker {
    pub fn new() -> Self {
        DescriptorPermissionChecker { state: State::ReadOrWriteOk }
    }

    pub fn update(&mut self, next_permission: DescriptorPermission) {
        let next_state = match (self.state, next_permission) {
            (State::ReadOrWriteOk, DescriptorPermission::WriteOnly) => State::OnlyWriteOk,
            (State::OnlyWriteOk, DescriptorPermission::ReadOnly) => State::Invalid,
            (_, _) => self.state,
        };
        self.state = next_state;
    }

    pub fn virtio_2642_holds(&self) -> bool {
        self.state != State::Invalid
    }
}
// ANCHOR_END: fsm
