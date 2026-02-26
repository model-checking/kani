# Solana Agent Escrow Example

This is a self-contained example that models a few safety-critical rules common in
Solana-style agent payment flows:

- **Timelock authorization**: only the agent can release funds before expiry; the API
  can release only after expiry.
- **Value conservation**: settlement splits always conserve value (refund + payment == amount).
- **Oracle tiering**: required oracle count is tiered by value-at-risk and is monotonic.
- **Escrow FSM**: only legal state transitions are possible.

The model is intentionally minimal (no Solana runtime / Anchor modeling) and exists
primarily to demonstrate Kani verification on agent-style programs.

## Run

```bash
cargo kani --harness timelock_policy_matches_release_rule
cargo kani --harness settlement_splits_conserve_value
cargo kani --harness required_oracle_count_is_monotonic_and_bounded
cargo kani --harness escrow_fsm_actions_respect_transition_table
```
