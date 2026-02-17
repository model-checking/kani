// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(kani)]
use kani_solana_agent::AccountInfo;

#[cfg(kani)]
fn authorized(now: i64, expires_at: i64, agent: &AccountInfo<'_>, api: &AccountInfo<'_>) -> bool {
    agent.is_signer || (api.is_signer && now >= expires_at)
}

#[cfg(kani)]
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

#[cfg(kani)]
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
    use kani_solana_agent::{AccountConfig, LamportSnapshot, any_agent_account, lamports};

    #[kani::proof]
    fn timelock_policy_matches_release_rule() {
        let now: i64 = kani::any();
        let expires_at: i64 = kani::any();

        let mut agent = any_agent_account::<0>(AccountConfig::new());
        let mut api = any_agent_account::<0>(AccountConfig::new());
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

        let mut agent = any_agent_account::<0>(AccountConfig::new());
        let mut api = any_agent_account::<0>(AccountConfig::new());
        agent.is_signer = kani::any();
        api.is_signer = kani::any();

        let escrow = any_agent_account::<0>(
            AccountConfig::new().writable().lamports_range(0..=(u32::MAX as u64)),
        );
        let payee = any_agent_account::<0>(
            AccountConfig::new().writable().lamports_range(0..=(u32::MAX as u64)),
        );

        let escrow_before = lamports(&escrow);
        let payee_before = lamports(&payee);
        kani::assume(escrow_before >= amount);

        let total_before = LamportSnapshot::new(&[&escrow, &payee]);

        let res = release_funds(now, expires_at, &agent, &api, &escrow, &payee, amount);
        let escrow_after = lamports(&escrow);
        let payee_after = lamports(&payee);

        if res.is_ok() {
            total_before.assert_unchanged(&[&escrow, &payee], "release must conserve lamports");
            kani::assert(escrow_after + amount == escrow_before, "escrow must decrease by amount");
            kani::assert(payee_before + amount == payee_after, "payee must increase by amount");
        } else {
            kani::assert(escrow_after == escrow_before, "failed release must not mutate");
            kani::assert(payee_after == payee_before, "failed release must not mutate");
        }
    }
}
