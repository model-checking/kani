// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;
use std::ops::RangeInclusive;

#[cfg(kani)]
use solana_program::rent::Rent;
#[cfg(kani)]
use std::cell::RefCell;
#[cfg(kani)]
use std::rc::Rc;

#[derive(Clone, Debug)]
enum Lamports {
    Any,
    Exact(u64),
    Range(RangeInclusive<u64>),
}

#[derive(Clone, Debug)]
pub struct AccountConfig {
    key: Option<Pubkey>,
    owner: Option<Pubkey>,
    lamports: Lamports,
    rent_exempt: bool,

    pub is_signer: bool,
    pub is_writable: bool,
    pub executable: bool,
    pub rent_epoch: u64,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            key: None,
            owner: None,
            lamports: Lamports::Any,
            rent_exempt: true,
            is_signer: false,
            is_writable: false,
            executable: false,
            rent_epoch: 0,
        }
    }
}

impl AccountConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key(mut self, key: Pubkey) -> Self {
        self.key = Some(key);
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn signer(mut self) -> Self {
        self.is_signer = true;
        self
    }

    pub fn writable(mut self) -> Self {
        self.is_writable = true;
        self
    }

    pub fn payer(mut self) -> Self {
        self.is_signer = true;
        self.is_writable = true;
        self
    }

    pub fn executable(mut self) -> Self {
        self.executable = true;
        self
    }

    pub fn lamports(mut self, lamports: u64) -> Self {
        self.lamports = Lamports::Exact(lamports);
        self
    }

    pub fn lamports_range(mut self, range: RangeInclusive<u64>) -> Self {
        self.lamports = Lamports::Range(range);
        self
    }

    pub fn rent_exempt(mut self, enabled: bool) -> Self {
        self.rent_exempt = enabled;
        self
    }

    pub fn rent_epoch(mut self, epoch: u64) -> Self {
        self.rent_epoch = epoch;
        self
    }
}

pub fn any_agent_account<const DATA_LEN: usize>(cfg: AccountConfig) -> AccountInfo<'static> {
    any_account_info::<DATA_LEN>(cfg)
}

#[cfg(kani)]
fn any_pubkey() -> Pubkey {
    Pubkey::new_from_array(kani::any())
}

#[cfg(kani)]
fn pick_lamports<const DATA_LEN: usize>(cfg: &AccountConfig) -> u64 {
    let lamports = match &cfg.lamports {
        Lamports::Any => kani::any(),
        Lamports::Exact(v) => *v,
        Lamports::Range(r) => {
            let v: u64 = kani::any();
            kani::assume(r.contains(&v));
            v
        }
    };

    if cfg.rent_exempt {
        kani::assume(Rent::default().is_exempt(lamports, DATA_LEN));
    }

    lamports
}

#[cfg(kani)]
pub fn any_account_info<const DATA_LEN: usize>(cfg: AccountConfig) -> AccountInfo<'static> {
    let key = cfg.key.unwrap_or_else(any_pubkey);
    let owner = cfg.owner.unwrap_or_else(any_pubkey);
    let lamports = pick_lamports::<DATA_LEN>(&cfg);

    let key: &'static Pubkey = Box::leak(Box::new(key));
    let owner: &'static Pubkey = Box::leak(Box::new(owner));

    let lamports: &'static mut u64 = Box::leak(Box::new(lamports));
    let data: &'static mut [u8; DATA_LEN] = Box::leak(Box::new(kani::any()));
    let data: &'static mut [u8] = data;

    AccountInfo {
        key,
        is_signer: cfg.is_signer,
        is_writable: cfg.is_writable,
        lamports: Rc::new(RefCell::new(lamports)),
        data: Rc::new(RefCell::new(data)),
        owner,
        executable: cfg.executable,
        rent_epoch: cfg.rent_epoch,
    }
}

#[cfg(not(kani))]
pub fn any_account_info<const DATA_LEN: usize>(cfg: AccountConfig) -> AccountInfo<'static> {
    let _ = DATA_LEN;
    match cfg.lamports {
        Lamports::Any => {}
        Lamports::Exact(v) => {
            let _ = v;
        }
        Lamports::Range(r) => {
            let _ = r;
        }
    }
    panic!("any_account_info is only available under `cargo kani` (cfg(kani))");
}
