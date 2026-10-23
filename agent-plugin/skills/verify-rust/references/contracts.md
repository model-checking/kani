# Function and loop contracts

Contracts (unstable, `-Z function-contracts`) specify a function once and let other proofs reuse
that specification instead of re-analyzing the function.

```rust
#[kani::requires(divisor != 0)]                       // precondition
#[kani::ensures(|result: &u32| *result <= dividend)]  // postcondition
fn div(dividend: u32, divisor: u32) -> u32 { dividend / divisor }

#[kani::proof_for_contract(div)]   // verifies `div` satisfies its contract
fn check_div() {
    let _ = div(kani::any(), kani::any());
}

#[kani::proof]
#[kani::stub_verified(div)]        // uses the contract instead of the body
fn check_caller() { ... }
```

```bash
cargo kani -Z function-contracts --harness check_div
```

- `#[kani::modifies(ptr)]` lists memory a function may write through pointers/references; it is
  required for functions that mutate through arguments.
- In `proof_for_contract`, Kani assumes `requires` on the inputs; you usually need no
  `kani::assume` in the harness. A `requires` that is too strong makes the proof cover little,
  and `requires(false)` covers nothing: run `scripts/check-assumptions.py`.
- `stub_verified(f)` is only sound if `f` has a passing `proof_for_contract` harness. Mention
  every `stub_verified` in the report.
- Recursive functions can be verified with contracts (Kani handles the recursion inductively).

## Loop contracts

Loop contracts (unstable, `-Z loop-contracts`) replace unrolling with an inductive invariant,
so the proof is not bounded by an unwinding limit:

```rust
#[kani::loop_invariant(i <= n && sum == i * (i - 1) / 2)]
while i < n { sum += i; i += 1; }
```

A wrong invariant fails verification; a too-weak invariant may fail to prove the property after
the loop. See the Kani book's "Loop contracts" chapter for `loop_modifies`.
