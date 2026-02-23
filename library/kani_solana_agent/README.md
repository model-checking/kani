# Solana AccountInfo Utilities for Kani Proofs

This crate provides small helpers to reduce the boilerplate involved in writing Kani proof harnesses for code that uses Solana's `AccountInfo` API.

Non-goals:
- Modeling Solana syscalls / runtime behavior
- Anchor-specific modeling
- Token program semantics

The primary entry point is `any_account_info::<DATA_LEN>(...)` (alias: `any_agent_account::<DATA_LEN>(...)`), which constructs an `AccountInfo<'static>` backed by leaked allocations so it can be passed into program logic during verification.

## Example

```rust
#[cfg(kani)]
mod proofs {
    use kani_solana_agent::{AccountConfig, LamportSnapshot, any_agent_account};

    #[kani::proof]
    fn can_build_accounts() {
        let payer = any_agent_account::<0>(AccountConfig::new().payer());
        let escrow = any_agent_account::<128>(AccountConfig::new().writable());

        kani::assert(payer.is_signer, "payer must be a signer");
        kani::assert(escrow.is_writable, "escrow must be writable");

        let before = LamportSnapshot::new(&[&payer, &escrow]);
        before.assert_unchanged(&[&payer, &escrow], "snapshot must be stable without mutation");
    }
}
```
