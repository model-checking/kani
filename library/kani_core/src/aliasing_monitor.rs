// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// The transition core of the aliasing checks (`-Z aliasing-checks`).
//
// This macro generates the state of the aliasing monitor for the one tracked byte and the pure
// functions that update it. It has no statics, pointers or Kani hooks: `kani_aliasing!` keeps the
// global state and turns pointers into the booleans these functions take.
//
// The rules are Tree Borrows' as Miri implements them (`miri/src/borrow_tracker/tree_borrows`).
//
// Every function is exported as `__VERIFIER_kani_aliasing_*`: CBMC's contract instrumentation
// (DFCC) does not instrument functions with that prefix, so a function contract's `modifies`
// check does not check, at great cost, the monitor's writes to its own state, the only memory it
// writes (its functions only take `&mut MonitorState`).
//
// The same code is verified with Verus (`library/kani_core/verus`): the Verus crate includes
// this file and expands this macro, and the `verus_verify` / `verus_spec` attributes below only
// exist when Verus compiles it (`verus_keep_ghost`). A normal build ignores them.
//
// State, for the tags `0..count` (tag 0 is the root of the allocation):
// - `ancestors[t]`: the bit set of the ancestors of `t`, including `t`, within its tree. A
//   reference created from a pointer with exposed provenance starts a tree of its own (a
//   *wildcard tree*, as in Miri): it has no known parent.
// - `main`: the tags of the allocation's own tree, rooted at tag 0.
// - `exposed_values`: a pointer created from an integer (tagged `EXPOSED_TAG`) may exist. Tests
//   of a tag against `EXPOSED_TAG` are made only when it is set, so that they stay concrete
//   when tags are symbolic (after program paths that created different numbers of tags merge).
// - `wildcard`: a wildcard tree may exist. Without one, every tag is in `main`; the flag keeps
//   that test concrete when program paths that created different numbers of tags merge.
// - `valid`, `active`, `frozen`, `conflicted`, `interior`: Tree Borrows' permission of each tag at
//   the tracked byte, as bits. Not valid is Disabled; valid and frozen is Frozen; valid, active
//   and not frozen is Active; otherwise Reserved, conflicted or interior-mutable per its bits.
// - `protected`, `strong`, `initialized`: the tag is protected (a reference or `Box` argument of
//   a running function), strongly so (references, not `Box`es); its permission at the tracked
//   byte is initialized (Miri's "accessed": Tree Borrows initializes lazily).
// - `cell`, `alias`: shared references to the bytes of an `UnsafeCell`, which Tree Borrows gives
//   the `Cell` permission: no access changes them or is rejected by them, so such a tag has no
//   permission of its own here, and accesses through it are accesses through `alias[t]`, its
//   nearest ancestor that is not one of these.
// - `exposed`, `unknown_exposed`: tags whose pointer was exposed, and whether a pointer with an
//   unknown tag was.
// - `stored`: tags of pointers stored in memory, which outlive the function that created them.

#[macro_export]
macro_rules! kani_aliasing_monitor {
    () => {
        /// Maximum number of tags alive at once, including the root.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const MAX_TAGS: u8 = 64;
        /// The tag of the allocation itself.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ROOT_TAG: u8 = 0;
        /// The tag of a pointer whose provenance the instrumentation does not know.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const UNKNOWN_TAG: u8 = u8::MAX;
        /// The tag of a pointer created from an integer: it may be any exposed tag.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const EXPOSED_TAG: u8 = u8::MAX - 1;

        /// Results of an access. The compiler-inserted assertions check the undefined behaviors;
        /// the wrappers report the `UNKNOWN` results as unsupported.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_OK: u8 = 0;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_INVALIDATED: u8 = 1;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_WRITE_FROZEN: u8 = 2;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_PROTECTOR: u8 = 3;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_NOT_EXPOSED: u8 = 4;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_UNKNOWN_TAG: u8 = 5;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_UNKNOWN_EXPOSED: u8 = 6;
        /// A wildcard access whose result depends on details of Miri's traversal that the
        /// checks do not model (see `monitor_wildcard_masks`).
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_UNKNOWN_WILDCARD: u8 = 7;
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const ACCESS_PROTECTED_DEALLOC: u8 = 8;

        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub struct MonitorState {
            pub tracked_object_id: usize,
            pub tracked_offset: usize,
            pub count: u8,
            pub ancestors: [u64; 64],
            pub main: u64,
            pub valid: u64,
            pub active: u64,
            pub frozen: u64,
            pub conflicted: u64,
            pub interior: u64,
            pub protected: u64,
            pub strong: u64,
            pub initialized: u64,
            pub cell: u64,
            pub alias: [u8; 64],
            pub exposed: u64,
            pub unknown_exposed: bool,
            pub stored: u64,
            pub wildcard: bool,
            pub exposed_values: bool,
        }

        /// The state before the harness chooses the tracked byte.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub const fn monitor_new() -> MonitorState {
            MonitorState {
                tracked_object_id: 0,
                tracked_offset: 0,
                count: 1,
                ancestors: [1; 64],
                main: 1,
                valid: 1,
                active: 1,
                frozen: 0,
                conflicted: 0,
                interior: 0,
                protected: 0,
                strong: 0,
                initialized: 1,
                cell: 0,
                alias: [0; 64],
                exposed: 0,
                unknown_exposed: false,
                stored: 0,
                wildcard: false,
                exposed_values: false,
            }
        }

        /// Track the byte at `offset` of object `obj`, with a single root tag that is valid,
        /// active and initialized.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(
            ensures monitor_wf(&*final(s)), initial_spec(abstraction(&*final(s)))
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_initialize"))]
        pub fn monitor_initialize(s: &mut MonitorState, obj: usize, offset: usize) {
            s.tracked_object_id = obj;
            s.tracked_offset = offset;
            s.count = 1;
            s.ancestors[0] = 1;
            s.main = 1;
            s.wildcard = false;
            s.exposed_values = false;
            s.valid = 1;
            s.active = 1;
            s.frozen = 0;
            s.conflicted = 0;
            s.interior = 0;
            s.protected = 0;
            s.strong = 0;
            s.initialized = 1;
            s.cell = 0;
            s.alias[0] = 0;
            s.exposed = 0;
            s.unknown_exposed = false;
            s.stored = 0;
            #[cfg(verus_keep_ghost)]
            proof! { lemma_initialize(*s); }
        }

        /// Whether the `size` bytes at `offset` of object `obj` contain the tracked byte.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_covers"))]
        pub fn monitor_covers(s: &MonitorState, obj: usize, offset: usize, size: usize) -> bool {
            s.tracked_object_id == obj
                && s.tracked_offset >= offset
                && s.tracked_offset - offset < size
        }

        /// The tag that accesses through `tag` act through.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(s), tag < s.count,
            ensures r as nat == eff(abstraction(s), tag as nat), r < s.count, !bit(s.cell, r as u64),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_eff"))]
        pub fn monitor_eff(s: &MonitorState, tag: u8) -> u8 {
            #[cfg(verus_keep_ghost)]
            proof! { lemma_test_bit(s.cell, tag as u64); }
            if s.cell & (1u64 << tag) != 0 { s.alias[tag as usize] } else { tag }
        }

        /// Tree Borrows' transitions for one access to the tracked byte: the tags in `local` see
        /// a local access, those in `foreign` a foreign one, and the others are not visited.
        /// The two sets contain no shared reference to a cell's bytes.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires
                monitor_wf(&*old(s)),
                local & foreign == 0, local & old(s).cell == 0, foreign & old(s).cell == 0,
                local & !live_mask(old(s).count as u64) == 0,
            ensures
                monitor_wf(&*final(s)),
                apply_refines(abstraction(&*old(s)), abstraction(&*final(s)), mask_set(local),
                    mask_set(foreign), is_write, r),
                final(s).tracked_object_id == old(s).tracked_object_id,
                final(s).tracked_offset == old(s).tracked_offset,
                failed_unchanged(r, &*old(s), &*final(s)),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_apply"))]
        pub fn monitor_apply(s: &mut MonitorState, local: u64, foreign: u64, is_write: bool) -> u8 {
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            #[cfg(verus_keep_ghost)]
            proof! {
                if !apply_allowed(&s0, local, foreign & s0.valid, is_write) {
                    lemma_apply_ub(s0, local, foreign, is_write);
                }
            }
            // A local access through a Disabled tag.
            if s.valid & local != local {
                return ACCESS_INVALIDATED;
            }
            // A foreign access does not change a Disabled tag.
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost foreign0 = foreign; }
            let foreign = foreign & s.valid;
            let accessed = s.protected & s.initialized;
            if is_write {
                if s.frozen & local != 0 {
                    return ACCESS_WRITE_FROZEN;
                }
                // A local write to a protected Reserved that saw a foreign read.
                if s.protected & s.conflicted & local != 0 {
                    return ACCESS_PROTECTOR;
                }
                // A foreign write disables everything but an interior-mutable Reserved.
                let disabled = foreign & !(s.interior & !s.active);
                if accessed & disabled != 0 {
                    return ACCESS_PROTECTOR;
                }
                s.valid = s.valid & !disabled;
                s.active = s.active | local;
            } else {
                // A foreign read disables a protected Active (which is initialized), and makes a
                // protected Reserved conflicted; an unprotected Active becomes Frozen.
                if s.protected & s.active & !s.frozen & foreign != 0 {
                    return ACCESS_PROTECTOR;
                }
                s.conflicted = s.conflicted | (s.protected & foreign & !s.active & !s.frozen);
                s.frozen = s.frozen | (s.active & foreign);
            }
            s.initialized = s.initialized | local;
            #[cfg(verus_keep_ghost)]
            proof! { lemma_apply_ok(s0, *s, local, foreign0, is_write); }
            ACCESS_OK
        }

        /// The tags `0..count`. Entries of `ancestors` above `count` are stale.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires 1 <= s.count <= 64,
            ensures r == live_mask(s.count as u64),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_live"))]
        pub fn monitor_live(s: &MonitorState) -> u64 {
            if s.count >= 64 {
                u64::MAX
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_shift_pos(s.count as u64); }
                (1u64 << s.count) - 1
            }
        }

        /// One step of `monitor_holders`: add tag `v` if its ancestors meet `b`.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires v < 64, h == holders_upto(s, b, v as nat),
            ensures r == holders_upto(s, b, (v + 1) as nat),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_holders_step"))]
        pub fn monitor_holders_step(s: &MonitorState, b: u64, v: usize, h: u64) -> u64 {
            #[cfg(verus_keep_ghost)]
            proof! { lemma_or_zero(h); }
            // `v < count` first: `count` is concrete, so symbolic execution skips the free slots.
            if v < s.count as usize && s.ancestors[v] & b != 0 { h | (1u64 << v) } else { h }
        }

        /// The tags whose ancestors meet `b`: with `b` the bit of a tag, its descendants and
        /// itself; with `b` the bit of a root, its tree.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires 1 <= s.count <= 64,
            ensures holders_spec(s, b, r),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_holders"))]
        pub fn monitor_holders(s: &MonitorState, b: u64) -> u64 {
            let mut h = 0u64;
            // Written out instead of looping, so that a harness's unwinding bound does not apply.
            // Only the tags `0..count` (`count` is concrete for symbolic execution).
            let n = s.count as usize;
            if 0 < n {
                h = monitor_holders_step(s, b, 0, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 0, h); }
            }
            if 1 < n {
                h = monitor_holders_step(s, b, 1, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 1, h); }
            }
            if 2 < n {
                h = monitor_holders_step(s, b, 2, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 2, h); }
            }
            if 3 < n {
                h = monitor_holders_step(s, b, 3, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 3, h); }
            }
            if 4 < n {
                h = monitor_holders_step(s, b, 4, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 4, h); }
            }
            if 5 < n {
                h = monitor_holders_step(s, b, 5, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 5, h); }
            }
            if 6 < n {
                h = monitor_holders_step(s, b, 6, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 6, h); }
            }
            if 7 < n {
                h = monitor_holders_step(s, b, 7, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 7, h); }
            }
            if 8 < n {
                h = monitor_holders_step(s, b, 8, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 8, h); }
            }
            if 9 < n {
                h = monitor_holders_step(s, b, 9, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 9, h); }
            }
            if 10 < n {
                h = monitor_holders_step(s, b, 10, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 10, h); }
            }
            if 11 < n {
                h = monitor_holders_step(s, b, 11, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 11, h); }
            }
            if 12 < n {
                h = monitor_holders_step(s, b, 12, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 12, h); }
            }
            if 13 < n {
                h = monitor_holders_step(s, b, 13, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 13, h); }
            }
            if 14 < n {
                h = monitor_holders_step(s, b, 14, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 14, h); }
            }
            if 15 < n {
                h = monitor_holders_step(s, b, 15, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 15, h); }
            }
            if 16 < n {
                h = monitor_holders_step(s, b, 16, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 16, h); }
            }
            if 17 < n {
                h = monitor_holders_step(s, b, 17, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 17, h); }
            }
            if 18 < n {
                h = monitor_holders_step(s, b, 18, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 18, h); }
            }
            if 19 < n {
                h = monitor_holders_step(s, b, 19, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 19, h); }
            }
            if 20 < n {
                h = monitor_holders_step(s, b, 20, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 20, h); }
            }
            if 21 < n {
                h = monitor_holders_step(s, b, 21, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 21, h); }
            }
            if 22 < n {
                h = monitor_holders_step(s, b, 22, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 22, h); }
            }
            if 23 < n {
                h = monitor_holders_step(s, b, 23, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 23, h); }
            }
            if 24 < n {
                h = monitor_holders_step(s, b, 24, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 24, h); }
            }
            if 25 < n {
                h = monitor_holders_step(s, b, 25, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 25, h); }
            }
            if 26 < n {
                h = monitor_holders_step(s, b, 26, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 26, h); }
            }
            if 27 < n {
                h = monitor_holders_step(s, b, 27, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 27, h); }
            }
            if 28 < n {
                h = monitor_holders_step(s, b, 28, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 28, h); }
            }
            if 29 < n {
                h = monitor_holders_step(s, b, 29, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 29, h); }
            }
            if 30 < n {
                h = monitor_holders_step(s, b, 30, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 30, h); }
            }
            if 31 < n {
                h = monitor_holders_step(s, b, 31, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 31, h); }
            }
            if 32 < n {
                h = monitor_holders_step(s, b, 32, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 32, h); }
            }
            if 33 < n {
                h = monitor_holders_step(s, b, 33, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 33, h); }
            }
            if 34 < n {
                h = monitor_holders_step(s, b, 34, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 34, h); }
            }
            if 35 < n {
                h = monitor_holders_step(s, b, 35, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 35, h); }
            }
            if 36 < n {
                h = monitor_holders_step(s, b, 36, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 36, h); }
            }
            if 37 < n {
                h = monitor_holders_step(s, b, 37, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 37, h); }
            }
            if 38 < n {
                h = monitor_holders_step(s, b, 38, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 38, h); }
            }
            if 39 < n {
                h = monitor_holders_step(s, b, 39, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 39, h); }
            }
            if 40 < n {
                h = monitor_holders_step(s, b, 40, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 40, h); }
            }
            if 41 < n {
                h = monitor_holders_step(s, b, 41, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 41, h); }
            }
            if 42 < n {
                h = monitor_holders_step(s, b, 42, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 42, h); }
            }
            if 43 < n {
                h = monitor_holders_step(s, b, 43, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 43, h); }
            }
            if 44 < n {
                h = monitor_holders_step(s, b, 44, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 44, h); }
            }
            if 45 < n {
                h = monitor_holders_step(s, b, 45, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 45, h); }
            }
            if 46 < n {
                h = monitor_holders_step(s, b, 46, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 46, h); }
            }
            if 47 < n {
                h = monitor_holders_step(s, b, 47, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 47, h); }
            }
            if 48 < n {
                h = monitor_holders_step(s, b, 48, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 48, h); }
            }
            if 49 < n {
                h = monitor_holders_step(s, b, 49, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 49, h); }
            }
            if 50 < n {
                h = monitor_holders_step(s, b, 50, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 50, h); }
            }
            if 51 < n {
                h = monitor_holders_step(s, b, 51, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 51, h); }
            }
            if 52 < n {
                h = monitor_holders_step(s, b, 52, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 52, h); }
            }
            if 53 < n {
                h = monitor_holders_step(s, b, 53, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 53, h); }
            }
            if 54 < n {
                h = monitor_holders_step(s, b, 54, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 54, h); }
            }
            if 55 < n {
                h = monitor_holders_step(s, b, 55, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 55, h); }
            }
            if 56 < n {
                h = monitor_holders_step(s, b, 56, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 56, h); }
            }
            if 57 < n {
                h = monitor_holders_step(s, b, 57, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 57, h); }
            }
            if 58 < n {
                h = monitor_holders_step(s, b, 58, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 58, h); }
            }
            if 59 < n {
                h = monitor_holders_step(s, b, 59, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 59, h); }
            }
            if 60 < n {
                h = monitor_holders_step(s, b, 60, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 60, h); }
            }
            if 61 < n {
                h = monitor_holders_step(s, b, 61, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 61, h); }
            }
            if 62 < n {
                h = monitor_holders_step(s, b, 62, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 62, h); }
            }
            if 63 < n {
                h = monitor_holders_step(s, b, 63, h);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_holders_skip(*s, b, 63, h); }
            }
            #[cfg(verus_keep_ghost)]
            proof! {
                lemma_holders_upto(*s, b, 64);
                assert forall|i: u64| i < 64 implies bit(h & live_mask(s.count as u64), i) == (i < s.count
                    && s.ancestors[i as int] & b != 0) by {
                    lemma_bit_and(h, live_mask(s.count as u64), i);
                    lemma_bit_live(s.count as u64, i);
                }
            }
            h & monitor_live(s)
        }

        /// The exposed tags that allow an access through a pointer created from an integer: Miri
        /// counts an exposed tag whose own permission allows the access (`strongest_allowed_local
        /// _access`), not looking at its ancestors.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r => ensures r == cand_mask(s, is_write)))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_candidates"))]
        pub fn monitor_candidates(s: &MonitorState, is_write: bool) -> u64 {
            let readable = s.exposed & s.valid & !s.cell;
            if is_write {
                readable & !s.frozen & !(s.protected & s.conflicted)
            } else {
                readable
            }
        }

        /// One step of `monitor_candidate_paths`: add the ancestors of tag `c` if it is a
        /// candidate.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(
            requires c < 64, *old(all) == all_upto(s, cands, c as nat),
                *old(any) == any_upto(s, cands, c as nat),
            ensures *final(all) == all_upto(s, cands, (c + 1) as nat),
                *final(any) == any_upto(s, cands, (c + 1) as nat),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_paths_step"))]
        pub fn monitor_paths_step(
            s: &MonitorState,
            cands: u64,
            c: usize,
            all: &mut u64,
            any: &mut u64,
        ) {
            #[cfg(verus_keep_ghost)]
            proof! { lemma_and_max(*all); lemma_or_zero(*any); }
            if c < s.count as usize && cands & (1u64 << c) != 0 {
                *all = *all & s.ancestors[c];
                *any = *any | s.ancestors[c];
            }
        }

        /// The intersection and the union of the ancestors of the candidates in `cands`.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            ensures r.0 == all_upto(s, cands, 64), r.1 == any_upto(s, cands, 64),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_candidate_paths"))]
        pub fn monitor_candidate_paths(s: &MonitorState, cands: u64) -> (u64, u64) {
            let mut all = u64::MAX;
            let mut any = 0u64;
            // Written out instead of looping, so that a harness's unwinding bound does not apply.
            // Only the tags `0..count` (`count` is concrete for symbolic execution).
            let n = s.count as usize;
            if 0 < n {
                monitor_paths_step(s, cands, 0, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 0, all, any); }
            }
            if 1 < n {
                monitor_paths_step(s, cands, 1, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 1, all, any); }
            }
            if 2 < n {
                monitor_paths_step(s, cands, 2, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 2, all, any); }
            }
            if 3 < n {
                monitor_paths_step(s, cands, 3, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 3, all, any); }
            }
            if 4 < n {
                monitor_paths_step(s, cands, 4, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 4, all, any); }
            }
            if 5 < n {
                monitor_paths_step(s, cands, 5, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 5, all, any); }
            }
            if 6 < n {
                monitor_paths_step(s, cands, 6, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 6, all, any); }
            }
            if 7 < n {
                monitor_paths_step(s, cands, 7, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 7, all, any); }
            }
            if 8 < n {
                monitor_paths_step(s, cands, 8, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 8, all, any); }
            }
            if 9 < n {
                monitor_paths_step(s, cands, 9, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 9, all, any); }
            }
            if 10 < n {
                monitor_paths_step(s, cands, 10, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 10, all, any); }
            }
            if 11 < n {
                monitor_paths_step(s, cands, 11, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 11, all, any); }
            }
            if 12 < n {
                monitor_paths_step(s, cands, 12, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 12, all, any); }
            }
            if 13 < n {
                monitor_paths_step(s, cands, 13, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 13, all, any); }
            }
            if 14 < n {
                monitor_paths_step(s, cands, 14, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 14, all, any); }
            }
            if 15 < n {
                monitor_paths_step(s, cands, 15, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 15, all, any); }
            }
            if 16 < n {
                monitor_paths_step(s, cands, 16, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 16, all, any); }
            }
            if 17 < n {
                monitor_paths_step(s, cands, 17, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 17, all, any); }
            }
            if 18 < n {
                monitor_paths_step(s, cands, 18, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 18, all, any); }
            }
            if 19 < n {
                monitor_paths_step(s, cands, 19, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 19, all, any); }
            }
            if 20 < n {
                monitor_paths_step(s, cands, 20, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 20, all, any); }
            }
            if 21 < n {
                monitor_paths_step(s, cands, 21, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 21, all, any); }
            }
            if 22 < n {
                monitor_paths_step(s, cands, 22, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 22, all, any); }
            }
            if 23 < n {
                monitor_paths_step(s, cands, 23, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 23, all, any); }
            }
            if 24 < n {
                monitor_paths_step(s, cands, 24, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 24, all, any); }
            }
            if 25 < n {
                monitor_paths_step(s, cands, 25, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 25, all, any); }
            }
            if 26 < n {
                monitor_paths_step(s, cands, 26, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 26, all, any); }
            }
            if 27 < n {
                monitor_paths_step(s, cands, 27, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 27, all, any); }
            }
            if 28 < n {
                monitor_paths_step(s, cands, 28, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 28, all, any); }
            }
            if 29 < n {
                monitor_paths_step(s, cands, 29, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 29, all, any); }
            }
            if 30 < n {
                monitor_paths_step(s, cands, 30, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 30, all, any); }
            }
            if 31 < n {
                monitor_paths_step(s, cands, 31, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 31, all, any); }
            }
            if 32 < n {
                monitor_paths_step(s, cands, 32, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 32, all, any); }
            }
            if 33 < n {
                monitor_paths_step(s, cands, 33, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 33, all, any); }
            }
            if 34 < n {
                monitor_paths_step(s, cands, 34, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 34, all, any); }
            }
            if 35 < n {
                monitor_paths_step(s, cands, 35, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 35, all, any); }
            }
            if 36 < n {
                monitor_paths_step(s, cands, 36, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 36, all, any); }
            }
            if 37 < n {
                monitor_paths_step(s, cands, 37, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 37, all, any); }
            }
            if 38 < n {
                monitor_paths_step(s, cands, 38, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 38, all, any); }
            }
            if 39 < n {
                monitor_paths_step(s, cands, 39, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 39, all, any); }
            }
            if 40 < n {
                monitor_paths_step(s, cands, 40, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 40, all, any); }
            }
            if 41 < n {
                monitor_paths_step(s, cands, 41, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 41, all, any); }
            }
            if 42 < n {
                monitor_paths_step(s, cands, 42, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 42, all, any); }
            }
            if 43 < n {
                monitor_paths_step(s, cands, 43, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 43, all, any); }
            }
            if 44 < n {
                monitor_paths_step(s, cands, 44, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 44, all, any); }
            }
            if 45 < n {
                monitor_paths_step(s, cands, 45, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 45, all, any); }
            }
            if 46 < n {
                monitor_paths_step(s, cands, 46, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 46, all, any); }
            }
            if 47 < n {
                monitor_paths_step(s, cands, 47, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 47, all, any); }
            }
            if 48 < n {
                monitor_paths_step(s, cands, 48, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 48, all, any); }
            }
            if 49 < n {
                monitor_paths_step(s, cands, 49, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 49, all, any); }
            }
            if 50 < n {
                monitor_paths_step(s, cands, 50, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 50, all, any); }
            }
            if 51 < n {
                monitor_paths_step(s, cands, 51, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 51, all, any); }
            }
            if 52 < n {
                monitor_paths_step(s, cands, 52, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 52, all, any); }
            }
            if 53 < n {
                monitor_paths_step(s, cands, 53, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 53, all, any); }
            }
            if 54 < n {
                monitor_paths_step(s, cands, 54, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 54, all, any); }
            }
            if 55 < n {
                monitor_paths_step(s, cands, 55, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 55, all, any); }
            }
            if 56 < n {
                monitor_paths_step(s, cands, 56, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 56, all, any); }
            }
            if 57 < n {
                monitor_paths_step(s, cands, 57, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 57, all, any); }
            }
            if 58 < n {
                monitor_paths_step(s, cands, 58, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 58, all, any); }
            }
            if 59 < n {
                monitor_paths_step(s, cands, 59, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 59, all, any); }
            }
            if 60 < n {
                monitor_paths_step(s, cands, 60, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 60, all, any); }
            }
            if 61 < n {
                monitor_paths_step(s, cands, 61, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 61, all, any); }
            }
            if 62 < n {
                monitor_paths_step(s, cands, 62, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 62, all, any); }
            }
            if 63 < n {
                monitor_paths_step(s, cands, 63, &mut all, &mut any);
            } else {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_paths_skip(*s, cands, 63, all, any); }
            }
            (all, any)
        }

        /// The tags a wildcard access treats as local and as foreign, as Miri's
        /// `perform_wildcard_access` does for every tree: a tag sees a local access if every
        /// candidate of its tree is in its subtree, a foreign one if none is, and no access
        /// otherwise. `own` is the source's tree when the access goes through a tag of a
        /// wildcard tree whose root is the single bit `root` (the source tree itself is accessed
        /// normally by the caller), and zero for an access through a pointer created from an
        /// integer. Returns the code, `local` and `foreign`.
        ///
        /// Miri visits a tree's tags children first and updates its candidates when an exposed
        /// tag loses permissions during the access, so later tags can see a more precise access.
        /// The checks compute every tag's access from the candidates before the access, and
        /// report `ACCESS_UNKNOWN_WILDCARD` (see `monitor_wildcard_settle`) when that could
        /// differ.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires own == 0 || root >= 1,
            ensures r == wild_bits(s, is_write, own, root),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_wildcard_masks"))]
        pub fn monitor_wildcard_masks(
            s: &MonitorState,
            is_write: bool,
            own: u64,
            root: u64,
        ) -> (u8, u64, u64) {
            let cands = monitor_candidates(s, is_write) & !own;
            let main_cands = cands & s.main;
            let (main_all, _) = monitor_candidate_paths(s, main_cands);
            let (_, any) = monitor_candidate_paths(s, cands);
            // The tree rooted at tag 0 must have a candidate the access can come from, and no
            // tag of it can be local to an access from a younger wildcard tree.
            if main_cands == 0 {
                return (ACCESS_NOT_EXPOSED, 0, 0);
            }
            let main_local = main_all & s.main & !s.cell;
            // The tags above the root of the source tree, which cannot be its ancestors.
            let above = if own == 0 { 0 } else { !(root | (root - 1)) };
            if own != 0 && (main_cands & !above == 0 || main_local & above != 0) {
                return (ACCESS_NOT_EXPOSED, 0, 0);
            }
            let local = main_local;
            let foreign = !own & !s.cell & !main_local & (!any | above);
            (ACCESS_OK, local, foreign)
        }

        /// Whether the access just applied changed the permissions of an exposed tag outside
        /// `own` in a way that changes the candidates (see `monitor_wildcard_masks`).
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            ensures r == settle_bits(before_read, before_write, s, own),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_wildcard_settle"))]
        pub fn monitor_wildcard_settle(
            before_read: u64,
            before_write: u64,
            s: &MonitorState,
            own: u64,
        ) -> bool {
            (monitor_candidates(s, false) ^ before_read) & !own == 0
                && (monitor_candidates(s, true) ^ before_write) & !own == 0
        }

        /// An access to the tracked byte through `tag`.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(&*old(s)), tag < old(s).count,
            ensures
                monitor_wf(&*final(s)),
                access_refines(abstraction(&*old(s)), abstraction(&*final(s)), tag as nat,
                    is_write, r),
                final(s).tracked_object_id == old(s).tracked_object_id,
                final(s).tracked_offset == old(s).tracked_offset,
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_access"))]
        pub fn monitor_access(s: &mut MonitorState, tag: u8, is_write: bool) -> u8 {
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            let e = monitor_eff(s, tag);
            let ancestors = s.ancestors[e as usize];
            let local = ancestors & !s.cell;
            #[cfg(verus_keep_ghost)]
            proof! {
                lemma_test_bit(s.main, e as u64);
                lemma_access_main_masks(s0, e as u64);
                lemma_bit_live(s.count as u64, e as u64);
            }
            // Checking first that there is no wildcard tree keeps this branch concrete for
            // symbolic execution when `e` is not (it depends on the tracked byte through `cell`).
            if !s.wildcard || s.main & (1u64 << e) != 0 {
                // Every tag of a wildcard tree sees a foreign access from the main tree.
                let code = monitor_apply(s, local, !ancestors & !s.cell, is_write);
                #[cfg(verus_keep_ghost)]
                proof! { lemma_access_main_spec(s0, *s, e as u64, tag, is_write, code); }
                return code;
            }
            // A tag of a wildcard tree: its own tree is accessed normally, the others through a
            // wildcard access whose local tags cannot be younger than the tree's root.
            if s.unknown_exposed {
                return ACCESS_UNKNOWN_EXPOSED;
            }
            #[cfg(verus_keep_ghost)]
            proof! { lemma_root(s0, e as u64); lemma_bit_zero(e as u64); }
            // The lowest tag of its ancestors, the root of its tree (`ancestors` contains `e`).
            let root = ancestors & ((!ancestors) + 1);
            let own = monitor_holders(s, root);
            #[cfg(verus_keep_ghost)]
            proof! {
                lemma_shift_pos(lowest(abstraction(&s0), e as nat) as u64);
                lemma_access_wild_own(s0, e as u64, own);
            }
            let (code, wild_local, wild_foreign) = monitor_wildcard_masks(s, is_write, own, root);
            #[cfg(verus_keep_ghost)]
            proof! {
                lemma_wild_meaning(s0, is_write, own, root,
                    lowest(abstraction(&s0), e as nat) as u64);
            }
            if code != ACCESS_OK {
                return code;
            }
            let before_read = monitor_candidates(s, false);
            let before_write = monitor_candidates(s, true);
            #[cfg(verus_keep_ghost)]
            proof! { lemma_access_wild_masks(s0, e as u64, own, wild_local, wild_foreign); }
            let code = monitor_apply(
                s,
                local | wild_local,
                (own & !ancestors & !s.cell) | wild_foreign,
                is_write,
            );
            #[cfg(verus_keep_ghost)]
            proof! {
                lemma_access_wild_spec(s0, *s, e as u64, tag, is_write, own, wild_local,
                    wild_foreign, code);
            }
            if code == ACCESS_OK && !monitor_wildcard_settle(before_read, before_write, s, own) {
                return ACCESS_UNKNOWN_WILDCARD;
            }
            code
        }

        /// An access to the tracked byte through a pointer created from an integer.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                wildcard_refines(abstraction(&*old(s)), abstraction(&*final(s)), is_write, r),
                final(s).tracked_object_id == old(s).tracked_object_id,
                final(s).tracked_offset == old(s).tracked_offset,
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_wildcard_access"))]
        pub fn monitor_wildcard_access(s: &mut MonitorState, is_write: bool) -> u8 {
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            if s.unknown_exposed {
                return ACCESS_UNKNOWN_EXPOSED;
            }
            #[cfg(verus_keep_ghost)]
            proof! { lemma_wild_meaning(s0, is_write, 0, 0, 0); }
            let (code, local, foreign) = monitor_wildcard_masks(s, is_write, 0, 0);
            if code != ACCESS_OK {
                return code;
            }
            let before_read = monitor_candidates(s, false);
            let before_write = monitor_candidates(s, true);
            let code = monitor_apply(s, local, foreign, is_write);
            #[cfg(verus_keep_ghost)]
            proof! { lemma_wildcard_spec(s0, *s, is_write, local, foreign, code); }
            if code == ACCESS_OK && !monitor_wildcard_settle(before_read, before_write, s, 0) {
                return ACCESS_UNKNOWN_WILDCARD;
            }
            code
        }

        /// Create a reference from a pointer tagged `parent` and return its tag, with the
        /// permission Miri's `NewPermission` gives it at the tracked byte. `covers`: the
        /// reference's range contains the tracked byte; `in_cell`: the byte is inside an
        /// `UnsafeCell` of the referent; `freeze`: the referent's type has no `UnsafeCell`, which
        /// decides the permission outside the range; `is_protected`, `is_strong`: a function
        /// argument, a reference (not a `Box`). A reference from a pointer created from an
        /// integer starts a wildcard tree. Out of tags, the new reference gets the unknown tag.
        ///
        /// The read access that creating a reference performs is checked separately, before.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                retag_spec(abstraction(&*old(s)), abstraction(&*final(s)), parent as nat, covers,
                    in_cell, freeze, is_shared, is_protected, is_strong, r as nat,
                    old(s).exposed_values),
                final(s).exposed_values == old(s).exposed_values,
                final(s).tracked_object_id == old(s).tracked_object_id,
                final(s).tracked_offset == old(s).tracked_offset,
        ))]
        #[allow(clippy::too_many_arguments)]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_new_borrow"))]
        pub fn monitor_new_borrow(
            s: &mut MonitorState,
            parent: u8,
            covers: bool,
            in_cell: bool,
            freeze: bool,
            is_shared: bool,
            is_protected: bool,
            is_strong: bool,
        ) -> u8 {
            let wild = s.exposed_values && parent == EXPOSED_TAG;
            if parent >= s.count && !wild {
                return UNKNOWN_TAG;
            }
            if s.count == MAX_TAGS {
                return UNKNOWN_TAG;
            }
            // The byte is in the part of the referent whose permission is that of a type with
            // interior mutability.
            let cellish = if covers { in_cell } else { !freeze };
            let is_cell = is_shared && cellish;
            if is_cell && wild {
                // A `Cell` in a wildcard tree: accesses through it would be wildcard accesses
                // from that tree, which the checks do not distinguish from an integer pointer's.
                return UNKNOWN_TAG;
            }
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            let tag = s.count;
            let bit = 1u64 << tag;
            s.count = s.count + 1;
            if wild {
                s.ancestors[tag as usize] = bit;
                s.main = s.main & !bit;
                s.wildcard = true;
            } else {
                s.ancestors[tag as usize] = s.ancestors[parent as usize] | bit;
                s.main = if s.main & (1u64 << parent) != 0 { s.main | bit } else { s.main & !bit };
            }
            s.valid = s.valid | bit;
            s.active = s.active & !bit;
            s.conflicted = s.conflicted & !bit;
            s.exposed = s.exposed & !bit;
            s.stored = s.stored & !bit;
            s.alias[tag as usize] = if is_cell {
                if s.cell & (1u64 << parent) != 0 { s.alias[parent as usize] } else { parent }
            } else {
                tag
            };
            s.cell = if is_cell { s.cell | bit } else { s.cell & !bit };
            // Miri gives a protected mutable reference the Reserved of a type without interior
            // mutability, so that `noalias` holds.
            s.interior = if !is_shared && cellish && !is_protected {
                s.interior | bit
            } else {
                s.interior & !bit
            };
            s.initialized =
                if covers && !is_cell { s.initialized | bit } else { s.initialized & !bit };
            s.protected =
                if is_protected && !is_cell { s.protected | bit } else { s.protected & !bit };
            s.strong = if is_protected && is_strong && !is_cell {
                s.strong | bit
            } else {
                s.strong & !bit
            };
            s.frozen = if is_shared && !is_cell { s.frozen | bit } else { s.frozen & !bit };
            #[cfg(verus_keep_ghost)]
            proof! {
                lemma_new_borrow(s0, *s, parent, covers, in_cell, freeze, is_shared, is_protected,
                    is_strong);
            }
            tag
        }

        /// Deallocating memory that contains the tracked byte through `tag`: a write access,
        /// then undefined behavior if a strongly protected tag has an initialized permission
        /// there.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                dealloc_refines(abstraction(&*old(s)), abstraction(&*final(s)), tag as nat, r,
                    old(s).exposed_values),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_dealloc"))]
        pub fn monitor_dealloc(s: &mut MonitorState, tag: u8) -> u8 {
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            let code = if s.exposed_values && tag == EXPOSED_TAG {
                monitor_wildcard_access(s, true)
            } else if tag < s.count {
                monitor_access(s, tag, true)
            } else {
                ACCESS_UNKNOWN_TAG
            };
            #[cfg(verus_keep_ghost)]
            proof! { lemma_dealloc(s0, *s, tag, code); }
            if code != ACCESS_OK {
                return code;
            }
            if s.protected & s.strong & s.initialized & !s.cell != 0 {
                return ACCESS_PROTECTED_DEALLOC;
            }
            ACCESS_OK
        }

        /// A pointer created from an integer, tagged `EXPOSED_TAG`, now exists.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                final(s).exposed_values,
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_note_exposed"))]
        pub fn monitor_note_exposed(s: &mut MonitorState) {
            s.exposed_values = true;
        }

        /// Record that the pointer tagged `tag` was exposed.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                expose_spec(abstraction(&*old(s)), abstraction(&*final(s)), tag as nat),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_expose"))]
        pub fn monitor_expose(s: &mut MonitorState, tag: u8) {
            if tag < s.count {
                #[cfg(verus_keep_ghost)]
                proof_decl! { let ghost s0 = *s; }
                let e = monitor_eff(s, tag);
                s.exposed = s.exposed | (1u64 << e);
                #[cfg(verus_keep_ghost)]
                proof! { lemma_expose(s0, *s, tag, e); }
            } else if tag != EXPOSED_TAG {
                s.unknown_exposed = true;
            }
        }

        /// Record that a pointer tagged `tag` was stored in memory: the tag outlives the function
        /// that created it.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                store_spec(abstraction(&*old(s)), abstraction(&*final(s)), tag as nat),
                final(s).tracked_object_id == old(s).tracked_object_id,
                final(s).tracked_offset == old(s).tracked_offset,
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_store"))]
        pub fn monitor_store(s: &mut MonitorState, tag: u8) {
            if tag < s.count {
                #[cfg(verus_keep_ghost)]
                proof_decl! { let ghost s0 = *s; }
                s.stored = s.stored | (1u64 << tag);
                #[cfg(verus_keep_ghost)]
                proof! { lemma_store(s0, *s, tag); }
            }
        }

        /// The protector of `tag`, an argument of the returning function, ends: if its
        /// permission at the tracked byte is initialized, Miri performs an access through it, a
        /// write if it is Active and a read otherwise, that its descendants do not see.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(&*old(s)),
            ensures
                monitor_wf(&*final(s)),
                end_protector_refines(abstraction(&*old(s)), abstraction(&*final(s)), tag as nat,
                    r),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_end_protector"))]
        pub fn monitor_end_protector(s: &mut MonitorState, tag: u8) -> u8 {
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            if tag >= s.count {
                return ACCESS_OK;
            }
            let bit = 1u64 << tag;
            #[cfg(verus_keep_ghost)]
            proof! { lemma_end_protector_tests(s0, tag as u64); }
            if s.protected & s.initialized & s.valid & !s.cell & bit == 0 {
                return ACCESS_OK;
            }
            if s.main & bit == 0 {
                // A protected tag of a wildcard tree: the access would be a wildcard access to
                // the other trees, which the checks do not model for protector ends.
                return ACCESS_UNKNOWN_WILDCARD;
            }
            let descendants = monitor_holders(s, bit) & !bit;
            // Miri skips the wildcard trees whose root is younger than the smallest exposed tag
            // of `tag`'s subtree (`min_exposed_child`): they may hang below `tag`. Their tags
            // have no ancestor up to that exposed tag.
            let mine = (descendants | bit) & s.exposed;
            let skip = if mine != 0 {
                #[cfg(verus_keep_ghost)]
                proof! { assert(mine != 0u64 ==> !mine < u64::MAX) by (bit_vector); }
                let low = mine & ((!mine) + 1);
                #[cfg(verus_keep_ghost)]
                proof! { assert(mine != 0u64 ==> mine & ((!mine) + 1) as u64 != 0u64) by (bit_vector); }
                !s.main & !monitor_holders(s, low | (low - 1))
            } else {
                0
            };
            #[cfg(verus_keep_ghost)]
            proof! { lemma_end_protector_masks(s0, tag as u64, descendants, mine, skip); }
            let is_write = s.active & !s.frozen & bit != 0;
            let ancestors = s.ancestors[tag as usize];
            let code = monitor_apply(
                s,
                ancestors & !s.cell,
                !ancestors & !descendants & !s.cell & !skip,
                is_write,
            );
            #[cfg(verus_keep_ghost)]
            proof! { lemma_end_protector_spec(s0, *s, tag as u64, descendants, skip, is_write, code); }
            code
        }

        /// A function that started with `frame_count` tags returns the tags in `kept`: its
        /// reference arguments stop being protected (after `monitor_end_protector`), and the
        /// tags it created are dropped, except the returned ones and the tags created before
        /// them. If one of the dropped tags was exposed or stored in memory, nothing is dropped: a
        /// pointer created from an integer, or read from memory, may still use it.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(
            requires monitor_wf(&*old(s)), frame_count >= 1,
            ensures
                monitor_wf(&*final(s)),
                return_spec(abstraction(&*old(s)), abstraction(&*final(s)), frame_count as nat,
                    kept@),
                final(s).tracked_object_id == old(s).tracked_object_id,
                final(s).tracked_offset == old(s).tracked_offset,
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_return"))]
        pub fn monitor_return(s: &mut MonitorState, frame_count: u8, kept: [u8; 4]) {
            // The function created no tag that is still alive: nothing to drop or unprotect.
            // Most returns are of this kind, and the test is concrete on most paths.
            if s.count <= frame_count {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_return_nothing(*s, frame_count, kept); }
                return;
            }
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s0 = *s; }
            if frame_count < MAX_TAGS {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_shift_pos(frame_count as u64); }
                let below = (1u64 << frame_count) - 1;
                s.protected = s.protected & below;
                s.strong = s.strong & below;
                s.conflicted = s.conflicted & below;
            }
            #[cfg(verus_keep_ghost)]
            proof_decl! { let ghost s1 = *s; }
            let mut count = frame_count;
            if frame_count <= kept[0] && kept[0] < s.count && count <= kept[0] {
                count = kept[0] + 1;
            }
            if frame_count <= kept[1] && kept[1] < s.count && count <= kept[1] {
                count = kept[1] + 1;
            }
            if frame_count <= kept[2] && kept[2] < s.count && count <= kept[2] {
                count = kept[2] + 1;
            }
            if frame_count <= kept[3] && kept[3] < s.count && count <= kept[3] {
                count = kept[3] + 1;
            }
            if count < s.count {
                #[cfg(verus_keep_ghost)]
                proof! { lemma_shift_pos(count as u64); }
                let below = (1u64 << count) - 1;
                if (s.exposed | s.stored) & !below != 0 {
                    #[cfg(verus_keep_ghost)]
                    proof! { lemma_return(s0, s1, *s, frame_count, kept, count); }
                    return;
                }
                s.count = count;
                // Forget the dropped tags entirely: a stale bit above `count` would otherwise
                // make a dropped tag look valid or exposed.
                s.main = s.main & below;
                s.valid = s.valid & below;
                s.active = s.active & below;
                s.frozen = s.frozen & below;
                s.conflicted = s.conflicted & below;
                s.interior = s.interior & below;
                s.protected = s.protected & below;
                s.strong = s.strong & below;
                s.initialized = s.initialized & below;
                s.cell = s.cell & below;
                s.exposed = s.exposed & below;
                s.stored = s.stored & below;
            }
            #[cfg(verus_keep_ghost)]
            proof! { lemma_return(s0, s1, *s, frame_count, kept, count); }
        }

        /// The nondeterministic choices of `monitor_loop_head`: the new bits of each mask, and
        /// the two flags.
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        pub struct LoopHavoc {
            pub valid: u64,
            pub active: u64,
            pub frozen: u64,
            pub conflicted: u64,
            pub initialized: u64,
            pub exposed: u64,
            pub stored: u64,
            pub unknown_exposed: bool,
            pub exposed_values: bool,
        }

        /// The borrow state at the head of a loop with a contract, where CBMC starts an
        /// arbitrary iteration: any state whose tags are those of `e`, the state where the loop
        /// started, each with the same ancestors and protector and a permission that later
        /// transitions can reach from its permission in `e` (Reserved, Active, Frozen,
        /// Disabled). The tags in `common`, ancestors of every tag the loop accesses through,
        /// only see local accesses: they stay valid and keep their frozen and conflicted bits.
        /// `v` is nondeterministic; the result is whether the state they give is well
        /// formed, which the caller assumes. `loop_head_complete` proves that every state the
        /// loop can reach is one of them (up to the abstraction).
        #[cfg_attr(verus_keep_ghost, verus_verify)]
        #[cfg_attr(verus_keep_ghost, verus_spec(r =>
            requires monitor_wf(e),
            ensures
                *final(s) == loop_head_spec(*e, common, v),
                r == loop_head_ok(&*final(s), e),
                !r || monitor_wf(&*final(s)),
        ))]
        #[cfg_attr(not(verus_keep_ghost), unsafe(export_name = "__VERIFIER_kani_aliasing_monitor_loop_head"))]
        pub fn monitor_loop_head(
            s: &mut MonitorState,
            e: &MonitorState,
            common: u64,
            v: LoopHavoc,
        ) -> bool {
            s.tracked_object_id = e.tracked_object_id;
            s.tracked_offset = e.tracked_offset;
            s.count = e.count;
            s.ancestors = e.ancestors;
            s.main = e.main;
            s.interior = e.interior;
            s.protected = e.protected;
            s.strong = e.strong;
            s.cell = e.cell;
            s.alias = e.alias;
            s.wildcard = e.wildcard;
            let live = monitor_live(e);
            let local = common & live;
            let any = !local;
            s.valid = e.valid & (v.valid | local);
            s.active = (e.active | v.active) & live;
            s.frozen = (e.frozen | (v.frozen & any)) & live;
            s.conflicted = (e.conflicted | (v.conflicted & e.protected & any)) & live;
            s.initialized = (e.initialized | v.initialized) & live;
            s.exposed = (e.exposed | v.exposed) & live;
            s.stored = (e.stored | v.stored) & live;
            s.unknown_exposed = e.unknown_exposed || v.unknown_exposed;
            s.exposed_values = e.exposed_values || v.exposed_values;
            // A tag becomes Frozen only from Active, and `monitor_wf`'s conditions on each tag.
            let ok = s.frozen & !e.frozen & !s.active == 0
                && s.frozen & s.interior & !s.active == 0
                && s.exposed & s.cell == 0
                && s.conflicted & (s.active | s.frozen) == 0
                && s.active & !s.initialized == 0;
            #[cfg(verus_keep_ghost)]
            proof! { if ok { lemma_loop_head_wf(*e, *s, common, v); } }
            ok
        }
    };
}
