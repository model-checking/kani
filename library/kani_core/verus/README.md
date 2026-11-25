# Verus proof of the aliasing monitor

`aliasing_monitor.rs` proves that the transition core of Kani's aliasing checks
(`kani_aliasing_monitor!` in `../src/aliasing_monitor.rs`) implements Tree Borrows' rules at one
memory location, as Miri implements them (`miri/src/borrow_tracker/tree_borrows`) and as this file
restates them.

The proof is about the code Kani runs. This crate includes `../src/aliasing_monitor.rs` and
expands the macro, so Verus verifies the same functions Kani compiles into its library. Their
specifications and proof calls are attributes and statements behind `cfg(verus_keep_ghost)`,
which only Verus sets; Kani's build ignores them.

## Running

With Verus 0.2026.10.04.426d8b0 (Rust 1.98.1):

```
verus library/kani_core/verus/aliasing_monitor.rs
```

Expected: `verification results:: 163 verified, 0 errors`. The proof uses no `assume`, `admit` or
`external_body`.

## What is proved

The abstract state (`AbsState`) is the set of trees of tags at the tracked location: the
allocation's tree, rooted at tag 0, and the wildcard trees that references created from pointers
with exposed provenance start. Each tag has its ancestors, a Tree Borrows permission (Reserved,
possibly conflicted or interior-mutable; Active; Frozen; Disabled), and whether it is protected
(strongly or weakly), initialized at the location (Miri's "accessed"), exposed, or a shared
reference to the bytes of an `UnsafeCell` (Miri's `Cell`, which acts through `alias`).
`abstraction` maps the monitor's bit sets to it; `monitor_wf` is the invariant every function
preserves.

The transitions are Miri's `perms.rs` (`local_step` / `foreign_step`, with `ProtectedDisabled`).
Every access goes through one function, `monitor_apply`, proved to apply `local_step` to the tags
of its local set, `foreign_step` to those of its foreign set, and nothing to the others, with
undefined behavior exactly when one of these transitions is. The other functions are proved to
compute the sets that Miri's rules give.

| Function | Theorem (ensures) | Plan stage |
|---|---|---|
| `monitor_initialize` | one root tag, Active and initialized (`initial_spec`) | 0 |
| `monitor_new_borrow` | `retag_spec`: a child of `parent`, or the root of a wildcard tree, with Miri's `NewPermission` (inside and outside the range; protected references are never interior-mutable) | 0, 1, 7, 8, 9, 11 |
| `monitor_apply` | `apply_refines`: the transitions of an access, given its local and foreign tags | 2-7, 9 |
| `monitor_access` | `access_refines`: the access through a tag; from a wildcard tree, also Miri's wildcard access to the other trees | 2-7, 9, 11 |
| `monitor_wildcard_access` | `wildcard_refines`: Miri's `perform_wildcard_access` on every tree (`wild_access`) | 11 |
| `monitor_dealloc` | `dealloc_refines`: a write through the pointer, then `ProtectedDealloc` for strong protectors | 10 |
| `monitor_end_protector` | `end_protector_refines`: Miri's `perform_protector_end_access`, skipping the wildcard trees that may hang below the tag (`min_exposed_child`) | 9 |
| `monitor_expose` | `expose_spec`: the tag accesses act through becomes exposed | 11 |
| `monitor_store` | `store_spec`: the tag of a pointer stored in memory is recorded, so that the frame that created it keeps it | 11 |
| `monitor_return` | `return_spec`: protectors end; unexposed tags of the frame that are not returned are dropped | 9 |
| `monitor_loop_head` | `loop_head_spec`; when its result is true the state is well formed (`lemma_loop_head_wf`) | loops |

All of them also preserve `monitor_wf`.

## Loops with contracts

At the head of a loop with a contract, `monitor_loop_head` replaces the state with any state in
`loop_rel` with the state where the loop started. The tags alive at loop entry keep their
ancestors, protector and cell; their permissions only move down (Reserved, Active, Frozen,
Disabled); and the loop's roots, the ancestors of every tag it accesses through, only see local
accesses. The proof that this is sound has three parts:

1. Every operation keeps `loop_rel` (`lemma_loop_apply`, `lemma_loop_access`,
   `lemma_loop_wildcard`, `lemma_loop_dealloc`, `lemma_loop_end_protector`,
   `lemma_loop_new_borrow`, `lemma_loop_expose`, `lemma_loop_store`, `lemma_loop_return`), from
   the operations' theorems above. The conditions: accesses act through descendants of the
   roots, or the loop has no roots; functions called in the loop return in it.
2. Dropping the tags created by earlier iterations keeps the state well formed and the other
   tags unchanged (`lemma_project`).
3. For every well-formed state in the relation, some choice of the nondeterministic values makes
   `monitor_loop_head` produce it, up to the abstraction, and accept it
   (`lemma_loop_head_complete`, `lemma_loop_head_sound`).

So the havoc covers every state the loop can reach at its head, without the tags of earlier
iterations. Results the checks cannot decide
(`ACCESS_UNKNOWN_*`) carry no theorem beyond `monitor_wf`; the library reports them as
unsupported.

## Mutation check

`exp/alias/verus/mutants.py` (outside the repository) applies one-line bugs to the monitor; the
proof must reject each of them.

## What is not proved (trusted)

- That the specification (`local_step`, `foreign_step`, `retag_node`, `wild_access`, ...) states
  Miri's Tree Borrows correctly. It is a transcription of Miri's code, cited in the comments, and
  is checked against Miri by running Miri's own borrow tests under both tools. The first version
  of this specification had four rules wrong, which only that comparison found.
- The wrappers in `aliasing.rs` (pointer to object/offset, coverage of the tracked byte,
  `UnsafeCell` byte masks, passing tags across calls) and the compiler instrumentation in
  `kani-compiler/src/kani_middle/transform/check_aliasing.rs`.
- That dropping the tags created by earlier iterations of a loop is sound: it relies on the
  instrumentation giving the pointers the loop assigns the unknown tag at the head, and on
  forgetting the shadow tags the loop may overwrite, so that no later access acts through them.
- That `exposed_values` and the tracked byte don't change during a loop except `exposed_values`
  becoming true (`lemma_loop_head_sound` assumes it; the operations' theorems are about the
  abstraction, which doesn't contain these fields).
- That the compiler's roots satisfy the conditions of part 1 (`loop_roots` in
  `check_aliasing.rs`, `aliasing_loop_root_tag` in `aliasing.rs`).
- That dropping the tags of a returning function is sound: it relies on the instrumentation
  recording, with `monitor_store`, every tag it stores in shadow memory, and giving the other
  pointers read from memory the unknown tag.
- Verus, its Z3 back end, and the arithmetic semantics they assume for the monitor's `u64`/`u8`
  operations, which Kani executes through CBMC.
