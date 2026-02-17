// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use kani_solana_agent::{AccountConfig, AccountInfo, any_account_info};

fn authorized(now: i64, expires_at: i64, agent: &AccountInfo<'_>, api: &AccountInfo<'_>) -> bool {
    agent.is_signer || (api.is_signer && now >= expires_at)
}

fn lamports(account: &AccountInfo<'_>) -> u64 {
    **account.lamports.borrow()
}

fn transfer(from: &AccountInfo<'_>, to: &AccountInfo<'_>, amount: u64) -> Result<(), ()> {
    let mut from_lamports = from.lamports.borrow_mut();
    let Some(new_from) = (**from_lamports).checked_sub(amount) else {
        return Err(());
    };
    **from_lamports = new_from;
    drop(from_lamports);

    let mut to_lamports = to.lamports.borrow_mut();
    let Some(new_to) = (**to_lamports).checked_add(amount) else {
        return Err(());
    };
    **to_lamports = new_to;
    Ok(())
}

fn release_funds(
    now: i64,
    expires_at: i64,
    agent: &AccountInfo<'_>,
    api: &AccountInfo<'_>,
    escrow: &AccountInfo<'_>,
    payee: &AccountInfo<'_>,
    amount: u64,
) -> Result<(), ()> {
    if !authorized(now, expires_at, agent, api) {
        return Err(());
    }

    transfer(escrow, payee, amount)
}

#[cfg(kani)]
mod proofs {
    use super::*;

    #[kani::proof]
    fn timelock_policy_matches_release_rule() {
        let now: i64 = kani::any();
        let expires_at: i64 = kani::any();

        let mut agent = any_account_info::<0>(AccountConfig::new());
        let mut api = any_account_info::<0>(AccountConfig::new());
        agent.is_signer = kani::any();
        api.is_signer = kani::any();

        let allowed = authorized(now, expires_at, &agent, &api);

        if now < expires_at {
            kani::assert(allowed == agent.is_signer, "before expiry only agent can release");
        } else {
            kani::assert(
                allowed == (agent.is_signer || api.is_signer),
                "after expiry agent or api can release",
            );
        }
    }

    #[kani::proof]
    fn release_funds_conserves_lamports() {
        let now: i64 = kani::any();
        let expires_at: i64 = kani::any();
        let amount: u64 = kani::any::<u32>() as u64;

        let mut agent = any_account_info::<0>(AccountConfig::new());
        let mut api = any_account_info::<0>(AccountConfig::new());
        agent.is_signer = kani::any();
        api.is_signer = kani::any();

        let escrow = any_account_info::<0>(
            AccountConfig::new().writable().lamports_range(0..=(u32::MAX as u64)),
        );
        let payee = any_account_info::<0>(
            AccountConfig::new().writable().lamports_range(0..=(u32::MAX as u64)),
        );

        let escrow_before = lamports(&escrow);
        let payee_before = lamports(&payee);
        kani::assume(escrow_before >= amount);

        let total_before = escrow_before as u128 + payee_before as u128;

        let res = release_funds(now, expires_at, &agent, &api, &escrow, &payee, amount);
        let escrow_after = lamports(&escrow);
        let payee_after = lamports(&payee);

        if res.is_ok() {
            let total_after = escrow_after as u128 + payee_after as u128;
            kani::assert(total_after == total_before, "release must conserve lamports");
            kani::assert(escrow_after + amount == escrow_before, "escrow must decrease by amount");
            kani::assert(payee_before + amount == payee_after, "payee must increase by amount");
        } else {
            kani::assert(escrow_after == escrow_before, "failed release must not mutate");
            kani::assert(payee_after == payee_before, "failed release must not mutate");
        }
    }
}
