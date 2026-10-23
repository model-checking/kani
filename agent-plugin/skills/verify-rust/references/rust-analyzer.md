# rust-analyzer as a helper

rust-analyzer gives precise answers about the code that text search does not: the type of an
expression, where a symbol is defined, every caller of a function, which trait impl is used.
Use it (through the editor's LSP integration, if the agent has one) to:

- map the call graph of the code under verification, for the proof strategy
  (`references/strategy.md`): `textDocument/prepareCallHierarchy` on a function, then
  `callHierarchy/outgoingCalls` for its callees and `callHierarchy/incomingCalls` for its
  callers. Expand outgoing calls recursively, stopping at functions outside the crate. Calls
  through trait objects and generic parameters may be missing: check those call sites by hand;
- find the exact path of the function under verification for `--harness`, `#[kani::stub]`, and
  `proof_for_contract`;
- find all callers of a function before adding a contract or a stub;
- check that a type implements `kani::Arbitrary`, or what fields must be derived;
- see which `cfg` features enable the code under verification.

Notes:

- Harnesses live under `#[cfg(kani)]`, which rust-analyzer does not enable by default, so it may
  show them as inactive. To get analysis inside harnesses, the user can add `kani` to
  `rust-analyzer.cargo.cfgs` in their editor settings. Do not change editor settings without
  asking.
- `kani::` items resolve only when the `kani` crate is visible to rust-analyzer; unresolved
  `kani::any` inside `#[cfg(kani)]` code is expected and not an error.
- rust-analyzer never replaces running Kani: it is for navigation and types only.
