// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use solana_program::account_info::AccountInfo;

pub fn lamports(account: &AccountInfo<'_>) -> u64 {
    **account.lamports.borrow()
}

pub fn sum_lamports(accounts: &[&AccountInfo<'_>]) -> u128 {
    accounts.iter().map(|a| lamports(a) as u128).sum()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LamportSnapshot(pub u128);

impl LamportSnapshot {
    pub fn new(accounts: &[&AccountInfo<'_>]) -> Self {
        Self(sum_lamports(accounts))
    }

    pub fn assert_unchanged(self, accounts: &[&AccountInfo<'_>], msg: &'static str) {
        let after = sum_lamports(accounts);

        #[cfg(kani)]
        kani::assert(after == self.0, msg);

        #[cfg(not(kani))]
        assert!(after == self.0, "{msg}");
    }
}
