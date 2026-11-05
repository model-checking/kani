// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Verus proof that the transition core of Kani's aliasing checks implements the rules it
//! documents.
//!
//! This crate verifies the code Kani runs: it includes `../src/aliasing_monitor.rs` and expands
//! `kani_aliasing_monitor!`, whose functions carry `verus_spec` attributes that only exist when
//! Verus compiles them. This file adds the specification those attributes refer to: an abstract
//! Tree Borrows state at the tracked memory location (a tree of tags, each with a permission),
//! the abstraction function from the monitor's bit sets to it, and the lemmas the proofs use.
//!
//! Run: `verus library/kani_core/verus/aliasing_monitor.rs` (see README.md).

#![allow(unused_imports)]

use vstd::prelude::*;

include!("../src/aliasing_monitor.rs");
kani_aliasing_monitor!();

verus! {

// ---------------------------------------------------------------------------------------------
// Bits
// ---------------------------------------------------------------------------------------------

/// Bit `i` of `m`.
pub open spec fn bit(m: u64, i: u64) -> bool {
    (m >> i) & 1u64 == 1u64
}

pub proof fn lemma_bit_and(a: u64, b: u64, i: u64)
    requires i < 64,
    ensures bit(a & b, i) == (bit(a, i) && bit(b, i)),
{
    assert(((a & b) >> i) & 1u64 == 1u64 <==> ((a >> i) & 1u64 == 1u64 && (b >> i) & 1u64 == 1u64))
        by (bit_vector) requires i < 64;
}

pub proof fn lemma_bit_or(a: u64, b: u64, i: u64)
    requires i < 64,
    ensures bit(a | b, i) == (bit(a, i) || bit(b, i)),
{
    assert(((a | b) >> i) & 1u64 == 1u64 <==> ((a >> i) & 1u64 == 1u64 || (b >> i) & 1u64 == 1u64))
        by (bit_vector) requires i < 64;
}

pub proof fn lemma_bit_not(a: u64, i: u64)
    requires i < 64,
    ensures bit(!a, i) == !bit(a, i),
{
    assert(((!a) >> i) & 1u64 == 1u64 <==> !((a >> i) & 1u64 == 1u64)) by (bit_vector)
        requires i < 64;
}

pub proof fn lemma_bit_single(t: u64, i: u64)
    requires t < 64, i < 64,
    ensures bit(1u64 << t, i) == (i == t),
{
    assert(((1u64 << t) >> i) & 1u64 == 1u64 <==> i == t) by (bit_vector) requires t < 64, i < 64;
}

pub proof fn lemma_bit_below(c: u64, i: u64)
    requires 0 < c < 64, i < 64,
    ensures bit(((1u64 << c) - 1u64) as u64, i) == (i < c),
{
    assert(((((1u64 << c) - 1u64) as u64) >> i) & 1u64 == 1u64 <==> i < c) by (bit_vector)
        requires 0 < c < 64, i < 64;
}

pub proof fn lemma_bit_zero(i: u64)
    requires i < 64,
    ensures !bit(0u64, i),
{
    assert(!((0u64 >> i) & 1u64 == 1u64)) by (bit_vector) requires i < 64;
}

pub proof fn lemma_bit_one(i: u64)
    requires i < 64,
    ensures bit(1u64, i) == (i == 0),
{
    assert(((1u64 >> i) & 1u64 == 1u64) <==> i == 0) by (bit_vector) requires i < 64;
}

// ---------------------------------------------------------------------------------------------
// Masks as sets of tags
// ---------------------------------------------------------------------------------------------

pub proof fn lemma_all_bits_clear(m: u64)
    requires forall|i: u64| i < 64 ==> !bit(m, i),
    ensures m == 0,
{
    assert(!bit(m, 0u64));
    assert(!bit(m, 1u64));
    assert(!bit(m, 2u64));
    assert(!bit(m, 3u64));
    assert(!bit(m, 4u64));
    assert(!bit(m, 5u64));
    assert(!bit(m, 6u64));
    assert(!bit(m, 7u64));
    assert(!bit(m, 8u64));
    assert(!bit(m, 9u64));
    assert(!bit(m, 10u64));
    assert(!bit(m, 11u64));
    assert(!bit(m, 12u64));
    assert(!bit(m, 13u64));
    assert(!bit(m, 14u64));
    assert(!bit(m, 15u64));
    assert(!bit(m, 16u64));
    assert(!bit(m, 17u64));
    assert(!bit(m, 18u64));
    assert(!bit(m, 19u64));
    assert(!bit(m, 20u64));
    assert(!bit(m, 21u64));
    assert(!bit(m, 22u64));
    assert(!bit(m, 23u64));
    assert(!bit(m, 24u64));
    assert(!bit(m, 25u64));
    assert(!bit(m, 26u64));
    assert(!bit(m, 27u64));
    assert(!bit(m, 28u64));
    assert(!bit(m, 29u64));
    assert(!bit(m, 30u64));
    assert(!bit(m, 31u64));
    assert(!bit(m, 32u64));
    assert(!bit(m, 33u64));
    assert(!bit(m, 34u64));
    assert(!bit(m, 35u64));
    assert(!bit(m, 36u64));
    assert(!bit(m, 37u64));
    assert(!bit(m, 38u64));
    assert(!bit(m, 39u64));
    assert(!bit(m, 40u64));
    assert(!bit(m, 41u64));
    assert(!bit(m, 42u64));
    assert(!bit(m, 43u64));
    assert(!bit(m, 44u64));
    assert(!bit(m, 45u64));
    assert(!bit(m, 46u64));
    assert(!bit(m, 47u64));
    assert(!bit(m, 48u64));
    assert(!bit(m, 49u64));
    assert(!bit(m, 50u64));
    assert(!bit(m, 51u64));
    assert(!bit(m, 52u64));
    assert(!bit(m, 53u64));
    assert(!bit(m, 54u64));
    assert(!bit(m, 55u64));
    assert(!bit(m, 56u64));
    assert(!bit(m, 57u64));
    assert(!bit(m, 58u64));
    assert(!bit(m, 59u64));
    assert(!bit(m, 60u64));
    assert(!bit(m, 61u64));
    assert(!bit(m, 62u64));
    assert(!bit(m, 63u64));
    assert(((m >> 0u64) & 1u64 != 1u64 && (m >> 1u64) & 1u64 != 1u64 && (m >> 2u64) & 1u64 != 1u64 && (m >> 3u64) & 1u64 != 1u64 && (m >> 4u64) & 1u64 != 1u64 && (m >> 5u64) & 1u64 != 1u64 && (m >> 6u64) & 1u64 != 1u64 && (m >> 7u64) & 1u64 != 1u64 && (m >> 8u64) & 1u64 != 1u64 && (m >> 9u64) & 1u64 != 1u64 && (m >> 10u64) & 1u64 != 1u64 && (m >> 11u64) & 1u64 != 1u64 && (m >> 12u64) & 1u64 != 1u64 && (m >> 13u64) & 1u64 != 1u64 && (m >> 14u64) & 1u64 != 1u64 && (m >> 15u64) & 1u64 != 1u64 && (m >> 16u64) & 1u64 != 1u64 && (m >> 17u64) & 1u64 != 1u64 && (m >> 18u64) & 1u64 != 1u64 && (m >> 19u64) & 1u64 != 1u64 && (m >> 20u64) & 1u64 != 1u64 && (m >> 21u64) & 1u64 != 1u64 && (m >> 22u64) & 1u64 != 1u64 && (m >> 23u64) & 1u64 != 1u64 && (m >> 24u64) & 1u64 != 1u64 && (m >> 25u64) & 1u64 != 1u64 && (m >> 26u64) & 1u64 != 1u64 && (m >> 27u64) & 1u64 != 1u64 && (m >> 28u64) & 1u64 != 1u64 && (m >> 29u64) & 1u64 != 1u64 && (m >> 30u64) & 1u64 != 1u64 && (m >> 31u64) & 1u64 != 1u64 && (m >> 32u64) & 1u64 != 1u64 && (m >> 33u64) & 1u64 != 1u64 && (m >> 34u64) & 1u64 != 1u64 && (m >> 35u64) & 1u64 != 1u64 && (m >> 36u64) & 1u64 != 1u64 && (m >> 37u64) & 1u64 != 1u64 && (m >> 38u64) & 1u64 != 1u64 && (m >> 39u64) & 1u64 != 1u64 && (m >> 40u64) & 1u64 != 1u64 && (m >> 41u64) & 1u64 != 1u64 && (m >> 42u64) & 1u64 != 1u64 && (m >> 43u64) & 1u64 != 1u64 && (m >> 44u64) & 1u64 != 1u64 && (m >> 45u64) & 1u64 != 1u64 && (m >> 46u64) & 1u64 != 1u64 && (m >> 47u64) & 1u64 != 1u64 && (m >> 48u64) & 1u64 != 1u64 && (m >> 49u64) & 1u64 != 1u64 && (m >> 50u64) & 1u64 != 1u64 && (m >> 51u64) & 1u64 != 1u64 && (m >> 52u64) & 1u64 != 1u64 && (m >> 53u64) & 1u64 != 1u64 && (m >> 54u64) & 1u64 != 1u64 && (m >> 55u64) & 1u64 != 1u64 && (m >> 56u64) & 1u64 != 1u64 && (m >> 57u64) & 1u64 != 1u64 && (m >> 58u64) & 1u64 != 1u64 && (m >> 59u64) & 1u64 != 1u64 && (m >> 60u64) & 1u64 != 1u64 && (m >> 61u64) & 1u64 != 1u64 && (m >> 62u64) & 1u64 != 1u64 && (m >> 63u64) & 1u64 != 1u64) ==> m == 0u64) by (bit_vector);
}

/// A non-zero mask has a set bit.
pub proof fn lemma_nonzero_witness(m: u64)
    requires m != 0,
    ensures exists|i: u64| i < 64 && bit(m, i),
{
    if forall|i: u64| i < 64 ==> !bit(m, i) {
        lemma_all_bits_clear(m);
    }
}

/// `x & y` is non-zero exactly when some bit is set in both.
pub proof fn lemma_and_nonzero(x: u64, y: u64)
    ensures (x & y != 0) == exists|i: u64| i < 64 && bit(x, i) && bit(y, i),
{
    if x & y != 0 {
        lemma_nonzero_witness(x & y);
        let i = choose|i: u64| i < 64 && bit(x & y, i);
        lemma_bit_and(x, y, i);
    } else {
        assert forall|i: u64| i < 64 implies !(bit(x, i) && bit(y, i)) by {
            lemma_bit_and(x, y, i);
            lemma_bit_zero(i);
        }
    }
}

/// `v & l == l` exactly when every bit of `l` is set in `v`.
pub proof fn lemma_subset(v: u64, l: u64)
    ensures (v & l == l) == forall|i: u64| i < 64 && bit(l, i) ==> bit(v, i),
{
    assert((v & l == l) <==> (l & !v == 0u64)) by (bit_vector);
    lemma_and_nonzero(l, !v);
    if l & !v != 0 {
        let i = choose|i: u64| i < 64 && bit(l, i) && bit(!v, i);
        lemma_bit_not(v, i);
    } else {
        assert forall|i: u64| i < 64 && bit(l, i) implies bit(v, i) by {
            lemma_bit_not(v, i);
        }
    }
}

/// Bit `i` of the mask of tags `0..c`.
pub proof fn lemma_bit_live(c: u64, i: u64)
    requires 1 <= c <= 64, i < 64,
    ensures bit(if c >= 64 { u64::MAX } else { ((1u64 << c) - 1u64) as u64 }, i) == (i < c),
{
    if c >= 64 {
        assert((u64::MAX >> i) & 1u64 == 1u64) by (bit_vector) requires i < 64;
    } else {
        lemma_bit_below(c, i);
    }
}

/// The lowest set bit of a non-zero `x`.
pub proof fn lemma_lowest_bit(x: u64, i: u64)
    requires x != 0, i < 64,
    ensures
        bit(x & ((!x) + 1) as u64, i) == (bit(x, i) && forall|j: u64| j < i ==> !bit(x, j)),
{
    assert(bit(x & ((!x) + 1) as u64, i) == (bit(x, i) && x & (((1u64 << i) - 1u64) as u64) == 0))
        by (bit_vector) requires x != 0, i < 64;
    if x & (((1u64 << i) - 1u64) as u64) == 0 {
        assert forall|j: u64| j < i implies !bit(x, j) by {
            assert(x & (((1u64 << i) - 1u64) as u64) == 0 && j < i && i < 64 ==> !((x >> j) & 1u64 == 1u64))
                by (bit_vector);
        }
    } else {
        if i > 0 {
            lemma_and_nonzero(x, ((1u64 << i) - 1u64) as u64);
            let j = choose|j: u64| j < 64 && bit(x, j) && bit(((1u64 << i) - 1u64) as u64, j);
            lemma_bit_below(i, j);
        } else {
            assert(x & (((1u64 << 0u64) - 1u64) as u64) == 0) by (bit_vector);
        }
    }
}

/// The tags above tag `r`, from its bit.
pub proof fn lemma_bit_above(r: u64, i: u64)
    requires r < 64, i < 64,
    ensures bit(!((1u64 << r) | (((1u64 << r) - 1u64) as u64)), i) == (i > r),
{
    assert(bit(!((1u64 << r) | (((1u64 << r) - 1u64) as u64)), i) == (i > r)) by (bit_vector)
        requires r < 64, i < 64;
}

// ---------------------------------------------------------------------------------------------
// Abstract state: Tree Borrows at the tracked location
// ---------------------------------------------------------------------------------------------

/// Tree Borrows' permissions as Miri implements them (`tree_borrows/perms.rs`), with Reserved
/// split by `conflicted` (`ReservedFrz { conflicted }`) and interior mutability (`ReservedIM`).
/// Miri's `Cell` permission is the `cell` flag of a node.
pub enum Perm {
    Reserved { conflicted: bool, interior: bool },
    Active,
    Frozen,
    Disabled,
}

pub struct Node {
    /// The tag's ancestors in its tree, including itself.
    pub ancestors: ISet<nat>,
    /// The tag is in the allocation's own tree, rooted at tag 0, not a wildcard tree.
    pub main: bool,
    pub perm: Perm,
    /// The tag is protected (a reference or `Box` argument of a running function), strongly so
    /// (a reference).
    pub protected: bool,
    pub strong: bool,
    /// Miri's `accessed`: the permission at this location is initialized.
    pub initialized: bool,
    /// A shared reference to the bytes of an `UnsafeCell` (Miri's `Cell` permission): not
    /// affected by any access, and accesses through it are accesses through `alias`.
    pub cell: bool,
    pub alias: nat,
    pub exposed: bool,
    /// A pointer with this tag was stored in memory.
    pub stored: bool,
}

pub struct AbsState {
    pub count: nat,
    pub nodes: IMap<nat, Node>,
    /// A pointer whose tag is unknown was exposed.
    pub unknown_exposed: bool,
}

pub open spec fn perm_of(s: &MonitorState, t: u64) -> Perm {
    if !bit(s.valid, t) {
        Perm::Disabled
    } else if bit(s.frozen, t) {
        Perm::Frozen
    } else if bit(s.active, t) {
        Perm::Active
    } else {
        Perm::Reserved { conflicted: bit(s.conflicted, t), interior: bit(s.interior, t) }
    }
}

pub open spec fn mask_set(m: u64) -> ISet<nat> {
    ISet::new(|i: nat| i < 64 && bit(m, i as u64))
}

pub open spec fn node_of(s: &MonitorState, t: u64) -> Node {
    Node {
        ancestors: mask_set(s.ancestors[t as int]),
        main: bit(s.main, t),
        perm: perm_of(s, t),
        protected: bit(s.protected, t),
        strong: bit(s.strong, t),
        initialized: bit(s.initialized, t),
        cell: bit(s.cell, t),
        alias: s.alias[t as int] as nat,
        exposed: bit(s.exposed, t),
        stored: bit(s.stored, t),
    }
}

/// The abstraction function: the tags `0..count` and their nodes.
pub open spec fn abstraction(s: &MonitorState) -> AbsState {
    AbsState {
        count: s.count as nat,
        nodes: IMap::new(|t: nat| t < s.count as nat, |t: nat| node_of(s, t as u64)),
        unknown_exposed: s.unknown_exposed,
    }
}

pub open spec fn node_eq(x: Node, y: Node) -> bool {
    &&& x.ancestors =~= y.ancestors
    &&& x.main == y.main
    &&& x.perm == y.perm
    &&& x.protected == y.protected
    &&& x.strong == y.strong
    &&& x.initialized == y.initialized
    &&& x.cell == y.cell
    &&& x.alias == y.alias
    &&& x.exposed == y.exposed
    &&& x.stored == y.stored
}

/// `b` has the tags of `a`, unchanged.
pub open spec fn same_state(a: AbsState, b: AbsState) -> bool {
    &&& b.count == a.count
    &&& b.unknown_exposed == a.unknown_exposed
    &&& forall|t: nat| t < a.count ==> node_eq(#[trigger] b.nodes[t], a.nodes[t])
}

/// The state after `monitor_initialize`: one root tag, Active and initialized.
pub open spec fn initial_spec(a: AbsState) -> bool {
    &&& a.count == 1
    &&& forall|u: nat| a.nodes[0].ancestors.contains(u) <==> u == 0
    &&& a.nodes[0].main
    &&& a.nodes[0].perm == Perm::Active
    &&& !a.nodes[0].protected
    &&& !a.nodes[0].strong
    &&& a.nodes[0].initialized
    &&& !a.nodes[0].cell
    &&& !a.nodes[0].exposed
    &&& !a.nodes[0].stored
    &&& !a.unknown_exposed
}

// ---------------------------------------------------------------------------------------------
// Well-formedness of the monitor state
// ---------------------------------------------------------------------------------------------

/// No bit at or above `count`.
pub open spec fn clean_above(m: u64, count: u64) -> bool {
    forall|i: u64| count <= i < 64 ==> !bit(m, i)
}

pub open spec fn monitor_wf(s: &MonitorState) -> bool {
    &&& 1 <= s.count <= 64
    // The ancestors of a tag form a chain in its tree: each tag is its own ancestor, ancestors
    // are created first, the relation is transitive, and any two ancestors are related.
    &&& forall|t: u64| #![trigger s.ancestors[t as int]] t < s.count ==> {
        &&& bit(s.ancestors[t as int], t)
        &&& forall|u: u64| u < 64 && bit(s.ancestors[t as int], u) ==> u <= t
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[t as int], u)
            && bit(s.ancestors[u as int], w) ==> bit(s.ancestors[t as int], w)
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[t as int], u)
            && bit(s.ancestors[t as int], w) && u <= w ==> bit(s.ancestors[w as int], u)
        // The main tree is the one rooted at tag 0.
        &&& bit(s.main, t) == bit(s.ancestors[t as int], 0)
    }
    // A shared reference to a cell's bytes points, through `alias`, at its nearest ancestor that
    // is a node of the tree; every other tag is its own alias.
    &&& forall|t: u64| #![trigger s.alias[t as int]] t < s.count ==> if bit(s.cell, t) {
        &&& s.alias[t as int] < t
        &&& !bit(s.cell, s.alias[t as int] as u64)
        &&& bit(s.ancestors[t as int], s.alias[t as int] as u64)
    } else {
        s.alias[t as int] == t
    }
    &&& !bit(s.cell, 0)
    &&& forall|t: u64| t < 64 ==> {
        // A frozen interior-mutable reference was written before (it was frozen by a foreign
        // read of an Active).
        &&& (bit(s.frozen, t) && bit(s.interior, t) ==> bit(s.active, t))
        // Exposing records the tag accesses act through, never a cell's shared reference.
        &&& (bit(s.exposed, t) ==> !bit(s.cell, t))
        // Only a protected Reserved is conflicted.
        &&& (bit(s.conflicted, t) ==> !bit(s.active, t) && !bit(s.frozen, t)
            && bit(s.protected, t))
        // Miri: "`Unique` can only be reached on actually accessed locations".
        &&& (bit(s.active, t) ==> bit(s.initialized, t))
        // Miri never creates a protected `ReservedIM`.
        &&& (bit(s.interior, t) ==> !bit(s.protected, t))
        &&& (bit(s.strong, t) ==> bit(s.protected, t))
    }
    // Without a wildcard tree, every tag is in the main tree.
    &&& (!s.wildcard ==> s.main == live_mask(s.count as u64))
    // Nothing is recorded for tags that do not exist.
    &&& clean_above(s.main, s.count as u64)
    &&& clean_above(s.valid, s.count as u64)
    &&& clean_above(s.active, s.count as u64)
    &&& clean_above(s.frozen, s.count as u64)
    &&& clean_above(s.conflicted, s.count as u64)
    &&& clean_above(s.interior, s.count as u64)
    &&& clean_above(s.protected, s.count as u64)
    &&& clean_above(s.strong, s.count as u64)
    &&& clean_above(s.initialized, s.count as u64)
    &&& clean_above(s.cell, s.count as u64)
    &&& clean_above(s.exposed, s.count as u64)
    &&& clean_above(s.stored, s.count as u64)
}

/// A tag on the path of a tag is a tag.
pub proof fn lemma_ancestor_is_tag(s: MonitorState, e: u64, i: u64)
    requires monitor_wf(&s), e < s.count, i < 64, bit(s.ancestors[e as int], i),
    ensures i <= e, i < s.count,
{
    assert(s.ancestors[e as int] == s.ancestors[e as int]);
}

// ---------------------------------------------------------------------------------------------
// Tree Borrows' transitions at one location (Miri `tree_borrows/perms.rs` and `tree.rs`)
// ---------------------------------------------------------------------------------------------

/// The transition of a node for an access local to it (through it or a descendant):
/// `child_read` / `child_write`. `None`: undefined behavior.
pub open spec fn local_step(n: Node, is_write: bool) -> Option<Node> {
    match n.perm {
        Perm::Disabled => None,
        Perm::Frozen => if is_write { None } else { Some(Node { initialized: true, ..n }) },
        Perm::Active => Some(Node { initialized: true, ..n }),
        Perm::Reserved { conflicted, interior } => if !is_write {
            Some(Node { initialized: true, ..n })
        } else if conflicted && n.protected {
            None
        } else {
            Some(Node { perm: Perm::Active, initialized: true, ..n })
        },
    }
}

/// The transition of a node for a foreign access: `foreign_read` / `foreign_write`, then
/// `ProtectedDisabled` if a protected node with an initialized permission becomes Disabled.
pub open spec fn foreign_step(n: Node, is_write: bool) -> Option<Node> {
    let next = match n.perm {
        Perm::Disabled => n.perm,
        Perm::Frozen => if is_write { Perm::Disabled } else { Perm::Frozen },
        Perm::Active => if is_write || n.protected { Perm::Disabled } else { Perm::Frozen },
        Perm::Reserved { conflicted, interior } => if !is_write {
            Perm::Reserved { conflicted: conflicted || n.protected, interior }
        } else if interior {
            n.perm
        } else {
            Perm::Disabled
        },
    };
    if n.protected && n.initialized && n.perm != Perm::Disabled && next == Perm::Disabled {
        None
    } else {
        Some(Node { perm: next, ..n })
    }
}

/// The transition of tag `t` for an access whose local tags are `l` and foreign tags `f`.
pub open spec fn apply_step(a: AbsState, l: ISet<nat>, f: ISet<nat>, t: nat, is_write: bool)
    -> Option<Node>
{
    if l.contains(t) {
        local_step(a.nodes[t], is_write)
    } else if f.contains(t) {
        foreign_step(a.nodes[t], is_write)
    } else {
        Some(a.nodes[t])
    }
}

/// An access with local tags `l` and foreign tags `f` is undefined behavior if any tag's
/// transition is; otherwise every tag takes its transition. Failing accesses change nothing.
pub open spec fn apply_refines(a: AbsState, b: AbsState, l: ISet<nat>, f: ISet<nat>,
    is_write: bool, code: u8) -> bool
{
    &&& (code == ACCESS_OK) == forall|t: nat| t < a.count ==> (#[trigger] apply_step(a, l, f, t,
        is_write)) is Some
    &&& code == ACCESS_OK || code == ACCESS_INVALIDATED || code == ACCESS_WRITE_FROZEN
        || code == ACCESS_PROTECTOR
    &&& code != ACCESS_OK ==> same_state(a, b)
    &&& code == ACCESS_OK ==> {
        &&& b.count == a.count
        &&& b.unknown_exposed == a.unknown_exposed
        &&& forall|t: nat| t < a.count ==> node_eq(#[trigger] b.nodes[t],
            apply_step(a, l, f, t, is_write)->Some_0)
    }
}

/// The masks of a successful `monitor_apply` from `s0` (`f` already restricted to valid tags).
pub open spec fn apply_post(s0: &MonitorState, s: &MonitorState, l: u64, f: u64, is_write: bool)
    -> bool
{
    let disabled = f & !(s0.interior & !s0.active);
    &&& s.count == s0.count
    &&& s.ancestors == s0.ancestors
    &&& s.alias == s0.alias
    &&& s.main == s0.main
    &&& s.cell == s0.cell
    &&& s.interior == s0.interior
    &&& s.protected == s0.protected
    &&& s.strong == s0.strong
    &&& s.exposed == s0.exposed
    &&& s.stored == s0.stored
    &&& s.exposed_values == s0.exposed_values
    &&& s.wildcard == s0.wildcard
    &&& s.unknown_exposed == s0.unknown_exposed
    &&& s.tracked_object_id == s0.tracked_object_id
    &&& s.tracked_offset == s0.tracked_offset
    &&& s.initialized == s0.initialized | l
    &&& if is_write {
        &&& s.valid == s0.valid & !disabled
        &&& s.active == s0.active | l
        &&& s.frozen == s0.frozen
        &&& s.conflicted == s0.conflicted
    } else {
        &&& s.valid == s0.valid
        &&& s.active == s0.active
        &&& s.conflicted == s0.conflicted | (s0.protected & f & !s0.active & !s0.frozen)
        &&& s.frozen == s0.frozen | (s0.active & f)
    }
}

/// The conditions `monitor_apply` rejects are absent.
pub open spec fn apply_allowed(s0: &MonitorState, l: u64, f: u64, is_write: bool) -> bool {
    let disabled = f & !(s0.interior & !s0.active);
    &&& s0.valid & l == l
    &&& is_write ==> s0.frozen & l == 0
    &&& is_write ==> s0.protected & s0.conflicted & l == 0
    &&& is_write ==> (s0.protected & s0.initialized) & disabled == 0
    &&& !is_write ==> s0.protected & s0.active & !s0.frozen & f == 0
}

/// The bits `monitor_apply` reads and writes, at bit `t`.
proof fn lemma_apply_bits(s0: MonitorState, l: u64, f: u64, t: u64)
    requires t < 64,
    ensures ({
        let disabled = f & !(s0.interior & !s0.active);
        &&& bit(disabled, t) == (bit(f, t) && !(bit(s0.interior, t) && !bit(s0.active, t)))
        &&& bit(s0.valid & !disabled, t) == (bit(s0.valid, t) && !bit(disabled, t))
        &&& bit(s0.active | l, t) == (bit(s0.active, t) || bit(l, t))
        &&& bit(s0.initialized | l, t) == (bit(s0.initialized, t) || bit(l, t))
        &&& bit(s0.conflicted | (s0.protected & f & !s0.active & !s0.frozen), t)
            == (bit(s0.conflicted, t) || (bit(s0.protected, t) && bit(f, t) && !bit(s0.active, t)
                && !bit(s0.frozen, t)))
        &&& bit(s0.frozen | (s0.active & f), t) == (bit(s0.frozen, t) || (bit(s0.active, t)
            && bit(f, t)))
        &&& bit(s0.valid & l, t) == (bit(s0.valid, t) && bit(l, t))
        &&& bit(s0.frozen & l, t) == (bit(s0.frozen, t) && bit(l, t))
        &&& bit(s0.protected & s0.conflicted & l, t) == (bit(s0.protected, t)
            && bit(s0.conflicted, t) && bit(l, t))
        &&& bit((s0.protected & s0.initialized) & disabled, t) == (bit(s0.protected, t)
            && bit(s0.initialized, t) && bit(disabled, t))
        &&& bit(s0.protected & s0.active & !s0.frozen & f, t) == (bit(s0.protected, t)
            && bit(s0.active, t) && !bit(s0.frozen, t) && bit(f, t))
    }),
{
    let disabled = f & !(s0.interior & !s0.active);
    lemma_bit_and(f, !(s0.interior & !s0.active), t);
    lemma_bit_not(s0.interior & !s0.active, t);
    lemma_bit_and(s0.interior, !s0.active, t);
    lemma_bit_not(s0.active, t);
    lemma_bit_and(s0.valid, !disabled, t);
    lemma_bit_not(disabled, t);
    lemma_bit_or(s0.active, l, t);
    lemma_bit_or(s0.initialized, l, t);
    lemma_bit_and(s0.protected, f, t);
    lemma_bit_and(s0.protected & f, !s0.active, t);
    lemma_bit_and(s0.protected & f & !s0.active, !s0.frozen, t);
    lemma_bit_not(s0.frozen, t);
    lemma_bit_or(s0.conflicted, s0.protected & f & !s0.active & !s0.frozen, t);
    lemma_bit_and(s0.active, f, t);
    lemma_bit_or(s0.frozen, s0.active & f, t);
    lemma_bit_and(s0.valid, l, t);
    lemma_bit_and(s0.frozen, l, t);
    lemma_bit_and(s0.protected, s0.conflicted, t);
    lemma_bit_and(s0.protected & s0.conflicted, l, t);
    lemma_bit_and(s0.protected, s0.initialized, t);
    lemma_bit_and(s0.protected & s0.initialized, disabled, t);
    lemma_bit_and(s0.protected, s0.active, t);
    lemma_bit_and(s0.protected & s0.active, !s0.frozen, t);
    lemma_bit_and(s0.protected & s0.active & !s0.frozen, f, t);
}

/// One tag after a successful `monitor_apply` is its transition.
proof fn lemma_apply_node(s0: MonitorState, s: MonitorState, l: u64, f0: u64, is_write: bool,
    t: u64)
    requires
        monitor_wf(&s0), t < s0.count, l & f0 == 0, l & s0.cell == 0, f0 & s0.cell == 0,
        apply_allowed(&s0, l, f0 & s0.valid, is_write),
        apply_post(&s0, &s, l, f0 & s0.valid, is_write),
    ensures ({
        let a = abstraction(&s0);
        &&& apply_step(a, mask_set(l), mask_set(f0), t as nat, is_write) is Some
        &&& node_eq(node_of(&s, t), apply_step(a, mask_set(l), mask_set(f0), t as nat,
            is_write)->Some_0)
    }),
{
    let f = f0 & s0.valid;
    lemma_apply_bits(s0, l, f, t);
    lemma_bit_and(f0, s0.valid, t);
    lemma_and_nonzero(l, f0);
    lemma_and_nonzero(l, s0.cell);
    lemma_subset(s0.valid, l);
    if is_write {
        lemma_and_nonzero(s0.frozen, l);
        lemma_and_nonzero(s0.protected & s0.conflicted, l);
        lemma_and_nonzero(s0.protected & s0.initialized, f & !(s0.interior & !s0.active));
        lemma_bit_and(s0.protected, s0.conflicted, t);
        lemma_bit_and(s0.protected, s0.initialized, t);
    } else {
        lemma_and_nonzero(s0.protected & s0.active & !s0.frozen, f);
        lemma_bit_and(s0.protected, s0.active, t);
        lemma_bit_and(s0.protected & s0.active, !s0.frozen, t);
        lemma_bit_not(s0.frozen, t);
    }
    assert(mask_set(s.ancestors[t as int]) =~= mask_set(s0.ancestors[t as int]));
}

/// A rejected `monitor_apply` has a tag whose transition is undefined behavior.
proof fn lemma_apply_ub(s0: MonitorState, l: u64, f0: u64, is_write: bool)
    requires
        monitor_wf(&s0), l & f0 == 0, l & s0.cell == 0, f0 & s0.cell == 0,
        l & !(if s0.count >= 64 { u64::MAX } else { ((1u64 << s0.count) - 1u64) as u64 }) == 0,
        !apply_allowed(&s0, l, f0 & s0.valid, is_write),
    ensures
        exists|t: nat| t < s0.count && (#[trigger] apply_step(abstraction(&s0), mask_set(l),
            mask_set(f0), t, is_write)) is None,
{
    let f = f0 & s0.valid;
    let live = if s0.count >= 64 { u64::MAX } else { ((1u64 << s0.count) - 1u64) as u64 };
    let a = abstraction(&s0);
    let d = f & !(s0.interior & !s0.active);
    // A tag of `l` is a tag.
    assert forall|t: u64| t < 64 && bit(l, t) implies t < s0.count by {
        lemma_and_nonzero(l, !live);
        lemma_bit_not(live, t);
        lemma_bit_live(s0.count as u64, t);
    }
    lemma_and_nonzero(l, f0);
    if s0.valid & l != l {
        lemma_subset(s0.valid, l);
        let t = choose|t: u64| t < 64 && bit(l, t) && !bit(s0.valid, t);
        assert(apply_step(a, mask_set(l), mask_set(f0), t as nat, is_write) is None);
    } else if is_write && s0.frozen & l != 0 {
        lemma_and_nonzero(s0.frozen, l);
        let t = choose|t: u64| t < 64 && bit(s0.frozen, t) && bit(l, t);
        lemma_subset(s0.valid, l);
        assert(apply_step(a, mask_set(l), mask_set(f0), t as nat, is_write) is None);
    } else if is_write && s0.protected & s0.conflicted & l != 0 {
        lemma_and_nonzero(s0.protected & s0.conflicted, l);
        let t = choose|t: u64| t < 64 && bit(s0.protected & s0.conflicted, t) && bit(l, t);
        lemma_bit_and(s0.protected, s0.conflicted, t);
        lemma_subset(s0.valid, l);
        lemma_and_nonzero(s0.frozen, l);
        assert(apply_step(a, mask_set(l), mask_set(f0), t as nat, is_write) is None);
    } else if is_write {
        lemma_and_nonzero(s0.protected & s0.initialized, d);
        let t = choose|t: u64| t < 64 && bit(s0.protected & s0.initialized, t) && bit(d, t);
        lemma_apply_bits(s0, l, f, t);
        lemma_bit_and(f0, s0.valid, t);
        lemma_bit_and(s0.protected, s0.initialized, t);
        assert(t < s0.count) by { if t >= s0.count { assert(!bit(s0.valid, t)); } }
        assert(!bit(l, t)) by { lemma_and_nonzero(l, f0); }
        assert(apply_step(a, mask_set(l), mask_set(f0), t as nat, is_write) is None);
    } else {
        lemma_and_nonzero(s0.protected & s0.active & !s0.frozen, f);
        let t = choose|t: u64| t < 64 && bit(s0.protected & s0.active & !s0.frozen, t)
            && bit(f, t);
        lemma_apply_bits(s0, l, f, t);
        lemma_bit_and(f0, s0.valid, t);
        lemma_bit_and(s0.protected, s0.active, t);
        lemma_bit_and(s0.protected & s0.active, !s0.frozen, t);
        lemma_bit_not(s0.frozen, t);
        assert(t < s0.count) by { if t >= s0.count { assert(!bit(s0.valid, t)); } }
        assert(!bit(l, t)) by { lemma_and_nonzero(l, f0); }
        assert(apply_step(a, mask_set(l), mask_set(f0), t as nat, is_write) is None);
    }
}

/// A successful `monitor_apply` refines the transitions and keeps the state well-formed.
proof fn lemma_apply_ok(s0: MonitorState, s: MonitorState, l: u64, f0: u64, is_write: bool)
    requires
        monitor_wf(&s0), l & f0 == 0, l & s0.cell == 0, f0 & s0.cell == 0,
        apply_allowed(&s0, l, f0 & s0.valid, is_write),
        apply_post(&s0, &s, l, f0 & s0.valid, is_write),
    ensures
        monitor_wf(&s),
        apply_refines(abstraction(&s0), abstraction(&s), mask_set(l), mask_set(f0), is_write,
            ACCESS_OK),
{
    let a = abstraction(&s0);
    let b = abstraction(&s);
    let f = f0 & s0.valid;
    assert forall|t: nat| t < a.count implies {
        &&& (#[trigger] apply_step(a, mask_set(l), mask_set(f0), t, is_write)) is Some
        &&& node_eq(b.nodes[t], apply_step(a, mask_set(l), mask_set(f0), t, is_write)->Some_0)
    } by {
        lemma_apply_node(s0, s, l, f0, is_write, t as u64);
    }
    // Well-formedness: the tree and the aliases are unchanged; the permission invariants hold.
    assert forall|t: u64| t < 64 implies {
        &&& (bit(s.frozen, t) && bit(s.interior, t) ==> bit(s.active, t))
        &&& (bit(s.exposed, t) ==> !bit(s.cell, t))
        &&& (bit(s.conflicted, t) ==> !bit(s.active, t) && !bit(s.frozen, t)
            && bit(s.protected, t))
        &&& (bit(s.active, t) ==> bit(s.initialized, t))
        &&& (bit(s.interior, t) ==> !bit(s.protected, t))
        &&& (bit(s.strong, t) ==> bit(s.protected, t))
        &&& (s0.count <= t ==> !bit(s.valid, t) && !bit(s.active, t) && !bit(s.frozen, t)
            && !bit(s.conflicted, t) && !bit(s.initialized, t))
    } by {
        lemma_apply_bits(s0, l, f, t);
        lemma_bit_and(f0, s0.valid, t);
        if is_write {
            lemma_and_nonzero(s0.protected & s0.conflicted, l);
            lemma_bit_and(s0.protected, s0.conflicted, t);
        }
        if s0.count <= t && bit(l, t) {
            lemma_subset(s0.valid, l);
        }
    }
}

/// The mask of the tags `0..c`.
pub open spec fn live_mask(c: u64) -> u64 {
    if c >= 64 { u64::MAX } else { ((1u64 << c) - 1u64) as u64 }
}

/// Adding tag `c` to the live tags `0..c` gives `0..c + 1`.
pub proof fn lemma_live_extend(c: u64)
    requires c < 64,
    ensures (live_mask(c) | (1u64 << c)) == live_mask((c + 1) as u64),
{
    if c == 63 {
        assert((((1u64 << 63u64) - 1u64) as u64 | (1u64 << 63u64)) == u64::MAX) by (bit_vector);
    } else {
        assert((((1u64 << c) - 1u64) as u64 | (1u64 << c)) == ((1u64 << ((c + 1) as u64)) - 1u64)
            as u64) by (bit_vector) requires c < 63;
    }
}

/// Keeping the live tags `0..d` of the live tags `0..c`, for `d <= c`, gives `0..d`.
pub proof fn lemma_live_shrink(c: u64, d: u64)
    requires d <= c <= 64, d < 64,
    ensures (live_mask(c) & (((1u64 << d) - 1u64) as u64)) == live_mask(d),
{
    if c >= 64 {
        assert((u64::MAX & (((1u64 << d) - 1u64) as u64)) == ((1u64 << d) - 1u64) as u64)
            by (bit_vector) requires d < 64;
    } else {
        assert(((((1u64 << c) - 1u64) as u64) & (((1u64 << d) - 1u64) as u64))
            == ((1u64 << d) - 1u64) as u64) by (bit_vector) requires d <= c, c < 64;
    }
}

/// The exec test `m & (1 << t) != 0` is bit `t` of `m`.
pub proof fn lemma_test_bit(m: u64, t: u64)
    requires t < 64,
    ensures (m & (1u64 << t) != 0) == bit(m, t),
{
    assert((m & (1u64 << t) != 0) <==> ((m >> t) & 1u64 == 1u64)) by (bit_vector) requires t < 64;
}

/// Bit `i` of `m` with bit `t` set or cleared.
pub proof fn lemma_bit_set(m: u64, t: u64, i: u64)
    requires t < 64, i < 64,
    ensures
        bit(m | (1u64 << t), i) == (i == t || bit(m, i)),
        bit(m & !(1u64 << t), i) == (i != t && bit(m, i)),
{
    lemma_bit_or(m, 1u64 << t, i);
    lemma_bit_and(m, !(1u64 << t), i);
    lemma_bit_not(1u64 << t, i);
    lemma_bit_single(t, i);
}

/// The tag that accesses through `t` act through.
pub open spec fn eff(a: AbsState, t: nat) -> nat {
    if a.nodes[t].cell { a.nodes[t].alias } else { t }
}

pub proof fn lemma_initialize(s: MonitorState)
    requires
        s.count == 1,
        s.ancestors[0] == 1u64,
        s.main == 1u64,
        s.valid == 1u64,
        s.active == 1u64,
        s.frozen == 0u64,
        s.conflicted == 0u64,
        s.interior == 0u64,
        s.protected == 0u64,
        s.strong == 0u64,
        s.initialized == 1u64,
        s.cell == 0u64,
        s.alias[0] == 0u8,
        s.exposed == 0u64,
        s.stored == 0u64,
        !s.unknown_exposed,
        !s.wildcard,
    ensures monitor_wf(&s), initial_spec(abstraction(&s)),
{
    assert(((1u64 << 1u64) - 1u64) as u64 == 1u64) by (bit_vector);
    assert forall|i: u64| i < 64 implies bit(1u64, i) == (i == 0) && !bit(0u64, i) by {
        lemma_bit_one(i);
        lemma_bit_zero(i);
    }
    let a = abstraction(&s);
    assert forall|u: nat| a.nodes[0].ancestors.contains(u) <==> u == 0 by {
        if u < 64 { lemma_bit_one(u as u64); }
    }
}

pub proof fn lemma_and_max(x: u64)
    ensures x & u64::MAX == x,
{
    assert(x & u64::MAX == x) by (bit_vector);
}

pub proof fn lemma_or_zero(x: u64)
    ensures x | 0u64 == x,
{
    assert(x | 0u64 == x) by (bit_vector);
}

/// `apply_refines` only looks at the tags `0..count` of its sets.
proof fn lemma_apply_refines_ext(a: AbsState, b: AbsState, l1: ISet<nat>, f1: ISet<nat>,
    l2: ISet<nat>, f2: ISet<nat>, is_write: bool, code: u8)
    requires
        forall|t: nat| t < a.count ==> (l1.contains(t) == l2.contains(t) && f1.contains(t)
            == f2.contains(t)),
        apply_refines(a, b, l1, f1, is_write, code),
    ensures apply_refines(a, b, l2, f2, is_write, code),
{
    assert forall|t: nat| t < a.count implies apply_step(a, l1, f1, t, is_write)
        == #[trigger] apply_step(a, l2, f2, t, is_write) by {}
    if code == ACCESS_OK {
        assert forall|t: nat| t < a.count implies node_eq(#[trigger] b.nodes[t],
            apply_step(a, l2, f2, t, is_write)->Some_0) by {
            assert(apply_step(a, l1, f1, t, is_write) == apply_step(a, l2, f2, t, is_write));
        }
    }
    if forall|t: nat| t < a.count ==> (#[trigger] apply_step(a, l1, f1, t, is_write)) is Some {
        assert forall|t: nat| t < a.count implies (#[trigger] apply_step(a, l2, f2, t, is_write))
            is Some by {
            assert(apply_step(a, l1, f1, t, is_write) == apply_step(a, l2, f2, t, is_write));
        }
    } else {
        let t = choose|t: nat| t < a.count && !((#[trigger] apply_step(a, l1, f1, t, is_write))
            is Some);
        assert(apply_step(a, l1, f1, t, is_write) == apply_step(a, l2, f2, t, is_write));
    }
}

pub proof fn lemma_shift_pos(f: u64)
    requires f < 64,
    ensures (1u64 << f) >= 1,
{
    assert((1u64 << f) >= 1u64) by (bit_vector) requires f < 64;
}

// ---------------------------------------------------------------------------------------------
// The written-out loops
// ---------------------------------------------------------------------------------------------

/// `monitor_holders` after the tags `0..k`.
pub open spec fn holders_upto(s: &MonitorState, b: u64, k: nat) -> u64
    decreases k,
{
    if k == 0 { 0u64 } else {
        let v = (k - 1) as nat;
        holders_upto(s, b, v) | if v < s.count && s.ancestors[v as int] & b != 0 {
            1u64 << (v as u64) } else { 0u64 }
    }
}

proof fn lemma_holders_upto(s: MonitorState, b: u64, k: nat)
    requires k <= 64,
    ensures forall|i: u64| i < 64 ==> bit(#[trigger] holders_upto(&s, b, k), i)
        == (i < k && i < s.count && s.ancestors[i as int] & b != 0),
    decreases k,
{
    if k == 0 {
        assert forall|i: u64| i < 64 implies !bit(holders_upto(&s, b, 0), i) by { lemma_bit_zero(i); }
    } else {
        let v = (k - 1) as nat;
        lemma_holders_upto(s, b, v);
        let add = if v < s.count && s.ancestors[v as int] & b != 0 { 1u64 << (v as u64) } else {
            0u64 };
        assert forall|i: u64| i < 64 implies bit(holders_upto(&s, b, k), i)
            == (i < k && i < s.count && s.ancestors[i as int] & b != 0) by {
            lemma_bit_or(holders_upto(&s, b, v), add, i);
            lemma_bit_single(v as u64, i);
            lemma_bit_zero(i);
        }
    }
}

/// `monitor_candidate_paths` after the candidates `0..k`.
pub open spec fn all_upto(s: &MonitorState, cands: u64, k: nat) -> u64
    decreases k,
{
    if k == 0 { u64::MAX } else {
        let c = (k - 1) as nat;
        all_upto(s, cands, c) & if c < s.count && cands & (1u64 << (c as u64)) != 0 {
            s.ancestors[c as int] } else { u64::MAX }
    }
}

pub open spec fn any_upto(s: &MonitorState, cands: u64, k: nat) -> u64
    decreases k,
{
    if k == 0 { 0u64 } else {
        let c = (k - 1) as nat;
        any_upto(s, cands, c) | if c < s.count && cands & (1u64 << (c as u64)) != 0 {
            s.ancestors[c as int] } else { 0u64 }
    }
}

proof fn lemma_paths_upto(s: MonitorState, cands: u64, k: nat)
    requires k <= 64,
    ensures
        forall|i: u64| i < 64 ==> bit(#[trigger] all_upto(&s, cands, k), i)
            == forall|c: u64| c < k && c < s.count && bit(cands, c) ==> bit(s.ancestors[c as int], i),
        forall|i: u64| i < 64 ==> bit(#[trigger] any_upto(&s, cands, k), i)
            == exists|c: u64| c < k && c < s.count && bit(cands, c) && bit(s.ancestors[c as int], i),
    decreases k,
{
    if k == 0 {
        assert forall|i: u64| i < 64 implies bit(all_upto(&s, cands, 0), i)
            && !bit(any_upto(&s, cands, 0), i) by {
            assert((u64::MAX >> i) & 1u64 == 1u64) by (bit_vector) requires i < 64;
            lemma_bit_zero(i);
        }
    } else {
        let c = (k - 1) as nat;
        lemma_paths_upto(s, cands, c);
        lemma_test_bit(cands, c as u64);
        let in_c = c < s.count && cands & (1u64 << (c as u64)) != 0;
        let pa = if in_c { s.ancestors[c as int] } else { u64::MAX };
        let po = if in_c { s.ancestors[c as int] } else { 0u64 };
        assert forall|i: u64| i < 64 implies {
            &&& bit(all_upto(&s, cands, k), i)
                == forall|d: u64| d < k && d < s.count && bit(cands, d) ==> bit(s.ancestors[d as int], i)
            &&& bit(any_upto(&s, cands, k), i)
                == exists|d: u64| d < k && d < s.count && bit(cands, d) && bit(s.ancestors[d as int], i)
        } by {
            lemma_bit_and(all_upto(&s, cands, c), pa, i);
            lemma_bit_or(any_upto(&s, cands, c), po, i);
            assert((u64::MAX >> i) & 1u64 == 1u64) by (bit_vector) requires i < 64;
            lemma_bit_zero(i);
            if !bit(all_upto(&s, cands, k), i) {
                if bit(all_upto(&s, cands, c), i) {
                    assert(c < s.count && bit(cands, c as u64) && !bit(s.ancestors[c as int], i));
                } else {
                    let d = choose|d: u64| d < c && d < s.count && bit(cands, d)
                        && !bit(s.ancestors[d as int], i);
                    assert(d < k);
                }
            }
            if bit(any_upto(&s, cands, k), i) {
                if !bit(any_upto(&s, cands, c), i) {
                    assert(c < s.count && bit(cands, c as u64) && bit(s.ancestors[c as int], i));
                }
            } else {
                assert forall|d: u64| !(d < k && d < s.count && bit(cands, d)
                    && bit(s.ancestors[d as int], i)) by {
                    if d == c { } else if d < c { }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Stage 11: accesses through pointers created from integers (Miri `tree_borrows/wildcard.rs`)
// ---------------------------------------------------------------------------------------------

/// The exposed tags whose own permission allows the access (`strongest_allowed_local_access`):
/// any but Disabled for a read; neither Frozen nor a protected conflicted Reserved for a write.
pub open spec fn is_cand(a: AbsState, c: nat, is_write: bool) -> bool {
    let n = a.nodes[c];
    &&& c < a.count
    &&& n.exposed
    &&& !n.cell
    &&& n.perm != Perm::Disabled
    &&& is_write ==> n.perm != Perm::Frozen
    &&& is_write ==> !(n.protected && n.perm matches Perm::Reserved { conflicted: true, .. })
}

pub open spec fn cand_mask(s: &MonitorState, is_write: bool) -> u64 {
    let readable = s.exposed & s.valid & !s.cell;
    if is_write { readable & !s.frozen & !(s.protected & s.conflicted) } else { readable }
}

proof fn lemma_cand(s: MonitorState, is_write: bool, c: u64)
    requires monitor_wf(&s), c < 64,
    ensures bit(cand_mask(&s, is_write), c) == is_cand(abstraction(&s), c as nat, is_write),
{
    lemma_bit_and(s.exposed, s.valid, c);
    lemma_bit_and(s.exposed & s.valid, !s.cell, c);
    lemma_bit_not(s.cell, c);
    let r = s.exposed & s.valid & !s.cell;
    lemma_bit_and(r, !s.frozen, c);
    lemma_bit_not(s.frozen, c);
    lemma_bit_and(s.protected, s.conflicted, c);
    lemma_bit_not(s.protected & s.conflicted, c);
    lemma_bit_and(r & !s.frozen, !(s.protected & s.conflicted), c);
}

/// A wildcard access, as Miri's `perform_wildcard_access` applies it to every tree other than
/// the source's (`own`): a tag sees a local access if every candidate of its tree is in its
/// subtree (`access_relatedness`: `total_num == local_num`), a foreign one if none is, and no
/// access otherwise ("Either"). Only the main tree can lack candidates (wildcard trees count one
/// more possible foreign access, `is_wildcard_tree`), and then the access is undefined behavior;
/// for an access from a wildcard tree whose root is `r` (`max_local_tag`), a tag younger than
/// `r` can only see a foreign access, a local one is undefined behavior, and the main tree must
/// have a candidate no younger than `r` (`has_valid_exposed`).
pub open spec fn wcand(a: AbsState, own: ISet<nat>, c: nat, is_write: bool) -> bool {
    is_cand(a, c, is_write) && !own.contains(c)
}

pub open spec fn wmain_cand(a: AbsState, own: ISet<nat>, c: nat, is_write: bool) -> bool {
    wcand(a, own, c, is_write) && a.nodes[c].main
}

pub open spec fn wmain_local(a: AbsState, own: ISet<nat>, t: nat, is_write: bool) -> bool {
    &&& t < a.count
    &&& a.nodes[t].main
    &&& !a.nodes[t].cell
    &&& forall|c: nat| #[trigger] wmain_cand(a, own, c, is_write) ==> a.nodes[c].ancestors.contains(t)
}

pub open spec fn wany(a: AbsState, own: ISet<nat>, t: nat, is_write: bool) -> bool {
    exists|c: nat| #[trigger] wcand(a, own, c, is_write) && a.nodes[c].ancestors.contains(t)
}

pub open spec fn wild_ub(a: AbsState, own: ISet<nat>, root: Option<nat>, is_write: bool) -> bool {
    ||| forall|c: nat| !#[trigger] wmain_cand(a, own, c, is_write)
    ||| root is Some && forall|c: nat| #[trigger] wmain_cand(a, own, c, is_write) ==> c > root->Some_0
    ||| root is Some && exists|t: nat| #[trigger] wmain_local(a, own, t, is_write) && t > root->Some_0
}

pub open spec fn wild_access(a: AbsState, own: ISet<nat>, root: Option<nat>, is_write: bool)
    -> (u8, ISet<nat>, ISet<nat>)
{
    let main_local = ISet::new(|t: nat| wmain_local(a, own, t, is_write));
    let foreign = ISet::new(|t: nat| t < a.count && !own.contains(t) && !a.nodes[t].cell
        && !wmain_local(a, own, t, is_write)
        && (!wany(a, own, t, is_write) || (root is Some && t > root->Some_0)));
    if wild_ub(a, own, root, is_write) { (ACCESS_NOT_EXPOSED, ISet::empty(), ISet::empty()) }
    else { (ACCESS_OK, main_local, foreign) }
}

/// The masks `monitor_wildcard_masks` computes.
pub open spec fn wild_bits(s: &MonitorState, is_write: bool, own: u64, root: u64) -> (u8, u64, u64)
{
    let cands = cand_mask(s, is_write) & !own;
    let main_cands = cands & s.main;
    let main_all = all_upto(s, main_cands, 64);
    let any = any_upto(s, cands, 64);
    if main_cands == 0 {
        (ACCESS_NOT_EXPOSED, 0u64, 0u64)
    } else {
        let main_local = main_all & s.main & !s.cell;
        let above = if own == 0 { 0u64 } else { !(root | ((root - 1u64) as u64)) };
        if own != 0 && (main_cands & !above == 0 || main_local & above != 0) {
            (ACCESS_NOT_EXPOSED, 0u64, 0u64)
        } else {
            (ACCESS_OK, main_local, !own & !s.cell & !main_local & (!any | above))
        }
    }
}

/// The tree whose root is `r`, as a mask (`monitor_holders` of its bit).
pub open spec fn tree_set(a: AbsState, r: nat) -> ISet<nat> {
    ISet::new(|v: nat| v < a.count && #[trigger] a.nodes[v].ancestors.contains(r))
}

/// The source tree of a wildcard access, from its mask: none for an integer pointer.
pub open spec fn wild_own(a: AbsState, own: u64, r: u64) -> ISet<nat> {
    if own == 0 { ISet::empty() } else { tree_set(a, r as nat) }
}

pub open spec fn wild_root(own: u64, r: u64) -> Option<nat> {
    if own == 0 { None } else { Some(r as nat) }
}

/// `wild_bits` computes `wild_access`: for an access through an integer pointer (`own == 0`), or
/// from the wildcard tree rooted at `r` (`own` its tree, `root` its bit).
#[verifier::rlimit(100)]
proof fn lemma_wild_meaning(s: MonitorState, is_write: bool, own: u64, root: u64, r: u64)
    requires
        monitor_wf(&s),
        own == 0 ==> root == 0,
        own != 0 ==> r < s.count && root == 1u64 << r && !bit(s.main, r)
            && forall|v: u64| v < 64 ==> bit(own, v) == (v < s.count
                && bit(s.ancestors[v as int], r)),
    ensures
        wild_bits(&s, is_write, own, root).0 == wild_access(abstraction(&s), wild_own(abstraction(&s),
            own, r), wild_root(own, r), is_write).0,
        wild_bits(&s, is_write, own, root).0 == ACCESS_OK ==> forall|t: nat| t < s.count ==> (
            #[trigger] mask_set(wild_bits(&s, is_write, own, root).1).contains(t)
            == wild_access(abstraction(&s), wild_own(abstraction(&s), own, r), wild_root(own, r),
                is_write).1.contains(t)),
        wild_bits(&s, is_write, own, root).0 == ACCESS_OK ==> forall|t: nat| t < s.count ==> (
            #[trigger] mask_set(wild_bits(&s, is_write, own, root).2).contains(t)
            == wild_access(abstraction(&s), wild_own(abstraction(&s), own, r), wild_root(own, r),
                is_write).2.contains(t)),
        wild_bits(&s, is_write, own, root).0 == ACCESS_OK ==> ({
            let l1 = wild_bits(&s, is_write, own, root).1;
            let f1 = wild_bits(&s, is_write, own, root).2;
            l1 & f1 == 0 && l1 & s.cell == 0 && f1 & s.cell == 0
                && l1 & !live_mask(s.count as u64) == 0 && l1 & own == 0 && f1 & own == 0
        }),
{
    let a = abstraction(&s);
    let own_set = if own == 0 { ISet::<nat>::empty() } else { tree_set(a, r as nat) };
    let rt = if own == 0 { None } else { Some(r as nat) };
    let cands = cand_mask(&s, is_write) & !own;
    let main_cands = cands & s.main;
    lemma_paths_upto(s, main_cands, 64);
    lemma_paths_upto(s, cands, 64);
    let main_all = all_upto(&s, main_cands, 64);
    let any = any_upto(&s, cands, 64);
    let main_local = main_all & s.main & !s.cell;
    let above = if own == 0 { 0u64 } else { !(root | ((root - 1u64) as u64)) };
    // Candidates.
    assert forall|c: u64| c < 64 implies {
        &&& bit(cands, c) == (is_cand(a, c as nat, is_write) && !own_set.contains(c as nat))
        &&& bit(main_cands, c) == (bit(cands, c) && bit(s.main, c))
    } by {
        lemma_cand(s, is_write, c);
        lemma_bit_and(cand_mask(&s, is_write), !own, c);
        lemma_bit_not(own, c);
        lemma_bit_and(cands, s.main, c);
        if own == 0 {
            lemma_bit_zero(c);
        } else if c < s.count {
            assert(a.nodes[c as nat].ancestors.contains(r as nat) == bit(s.ancestors[c as int], r));
        }
    }
    assert forall|t: u64| t < 64 implies {
        &&& bit(main_local, t) == (bit(main_all, t) && bit(s.main, t) && !bit(s.cell, t))
        &&& bit(above, t) == (own != 0 && t > r)
    } by {
        lemma_bit_and(main_all, s.main, t);
        lemma_bit_and(main_all & s.main, !s.cell, t);
        lemma_bit_not(s.cell, t);
        if own != 0 { lemma_bit_above(r, t); } else { lemma_bit_zero(t); }
    }
    // Tags of `cands` are tags, and their ancestors are tags.
    assert forall|c: u64, t: u64| c < 64 && t < 64 && bit(cands, c) && bit(s.ancestors[c as int], t)
        implies t < s.count && c < s.count by {
        lemma_cand(s, is_write, c);
        lemma_bit_and(cand_mask(&s, is_write), !own, c);
        lemma_ancestor_is_tag(s, c, t);
    }
    // The own tree has no tag of the main tree.
    assert forall|v: u64| v < 64 && bit(own, v) implies !bit(s.main, v) by {
        if own != 0 && bit(s.main, v) {
            assert(s.ancestors[v as int] == s.ancestors[v as int]);
            assert(bit(s.ancestors[v as int], 0));
            assert(bit(s.ancestors[r as int], 0));
        }
        if own == 0 { lemma_bit_zero(v); }
    }
    // `main_all` and `any` at the tags.
    assert forall|t: u64| t < 64 implies {
        &&& bit(main_all, t) == forall|c: nat| #[trigger] wmain_cand(a, own_set, c, is_write)
            ==> a.nodes[c].ancestors.contains(t as nat)
        &&& bit(any, t) == wany(a, own_set, t as nat, is_write)
    } by {
        if bit(main_all, t) {
            assert forall|c: nat| #[trigger] wmain_cand(a, own_set, c, is_write) implies
                a.nodes[c].ancestors.contains(t as nat) by {
                assert(c < 64);
                assert(bit(main_cands, c as u64));
            }
        } else {
            let c = choose|c: u64| c < 64 && bit(main_cands, c) && !bit(s.ancestors[c as int], t);
            assert(wmain_cand(a, own_set, c as nat, is_write));
        }
        if bit(any, t) {
            let c = choose|c: u64| c < 64 && bit(cands, c) && bit(s.ancestors[c as int], t);
            assert(wcand(a, own_set, c as nat, is_write));
            assert(a.nodes[c as nat].ancestors.contains(t as nat));
        } else if wany(a, own_set, t as nat, is_write) {
            let c = choose|c: nat| #[trigger] wcand(a, own_set, c, is_write)
                && a.nodes[c].ancestors.contains(t as nat);
            assert(c < 64);
            assert(bit(cands, c as u64) && bit(s.ancestors[c as int], t));
        }
    }
    assert forall|t: u64| t < 64 implies bit(main_local, t) == wmain_local(a, own_set, t as nat,
        is_write) by {
        if bit(s.main, t) && t >= s.count { assert(!bit(s.main, t)); }
    }
    // Whether the main tree has a candidate.
    assert((main_cands == 0) == forall|c: nat| !#[trigger] wmain_cand(a, own_set, c, is_write)) by {
        if main_cands == 0 {
            assert forall|c: nat| !#[trigger] wmain_cand(a, own_set, c, is_write) by {
                if wmain_cand(a, own_set, c, is_write) {
                    assert(c < 64);
                    assert(bit(main_cands, c as u64));
                    lemma_bit_zero(c as u64);
                }
            }
        } else {
            lemma_nonzero_witness(main_cands);
            let c = choose|c: u64| c < 64 && bit(main_cands, c);
            assert(wmain_cand(a, own_set, c as nat, is_write));
        }
    }
    if main_cands != 0 && own != 0 {
        assert((main_cands & !above == 0) == forall|c: nat| #[trigger] wmain_cand(a, own_set, c,
            is_write) ==> c > r) by {
            lemma_and_nonzero(main_cands, !above);
            if main_cands & !above != 0 {
                let c = choose|c: u64| c < 64 && bit(main_cands, c) && bit(!above, c);
                lemma_bit_not(above, c);
                assert(wmain_cand(a, own_set, c as nat, is_write));
            } else {
                assert forall|c: nat| #[trigger] wmain_cand(a, own_set, c, is_write) implies c > r by {
                    assert(c < 64);
                    lemma_bit_not(above, c as u64);
                }
            }
        }
        assert((main_local & above != 0) == exists|t: nat| #[trigger] wmain_local(a, own_set, t,
            is_write) && t > r) by {
            lemma_and_nonzero(main_local, above);
            if main_local & above != 0 {
                let t = choose|t: u64| t < 64 && bit(main_local, t) && bit(above, t);
                assert(wmain_local(a, own_set, t as nat, is_write));
            } else if exists|t: nat| #[trigger] wmain_local(a, own_set, t, is_write) && t > r {
                let t = choose|t: nat| #[trigger] wmain_local(a, own_set, t, is_write) && t > r;
                assert(t < 64);
                assert(bit(main_local, t as u64) && bit(above, t as u64));
            }
        }
    }
    let f1 = !own & !s.cell & !main_local & (!any | above);
    assert forall|t: u64| t < 64 implies {
        &&& bit(f1, t) == (!bit(own, t) && !bit(s.cell, t) && !bit(main_local, t)
            && (!bit(any, t) || bit(above, t)))
        &&& !(bit(main_local, t) && bit(f1, t))
        &&& !(bit(main_local, t) && bit(s.cell, t))
        &&& !(bit(f1, t) && bit(s.cell, t))
        &&& (bit(main_local, t) ==> t < s.count)
        &&& !(bit(main_local, t) && bit(own, t))
        &&& !(bit(f1, t) && bit(own, t))
        &&& (t < s.count ==> bit(own, t) == own_set.contains(t as nat))
    } by {
        lemma_bit_and(!own, !s.cell, t);
        lemma_bit_and(!own & !s.cell, !main_local, t);
        lemma_bit_and(!own & !s.cell & !main_local, !any | above, t);
        lemma_bit_or(!any, above, t);
        lemma_bit_not(own, t);
        lemma_bit_not(s.cell, t);
        lemma_bit_not(main_local, t);
        lemma_bit_not(any, t);
        if own == 0 { lemma_bit_zero(t); }
        else if t < s.count {
            assert(a.nodes[t as nat].ancestors.contains(r as nat) == bit(s.ancestors[t as int], r));
        }
    }
    lemma_and_nonzero(main_local, f1);
    lemma_and_nonzero(main_local, s.cell);
    lemma_and_nonzero(f1, s.cell);
    lemma_and_nonzero(main_local, own);
    lemma_and_nonzero(f1, own);
    lemma_and_nonzero(main_local, !live_mask(s.count as u64));
    assert forall|t: u64| t < 64 && bit(main_local, t) implies !bit(!live_mask(s.count as u64), t) by {
        lemma_bit_not(live_mask(s.count as u64), t);
        lemma_bit_live(s.count as u64, t);
    }
}

/// A failed operation changes nothing.
pub open spec fn failed_unchanged(code: u8, s0: &MonitorState, s: &MonitorState) -> bool {
    code != ACCESS_OK ==> *s == *s0
}

/// Masks with the same bits are equal.
pub proof fn lemma_bits_ext(x: u64, y: u64)
    requires forall|i: u64| i < 64 ==> bit(x, i) == bit(y, i),
    ensures x == y,
{
    assert forall|i: u64| i < 64 implies !bit(x ^ y, i) by {
        assert(bit(x ^ y, i) == (bit(x, i) != bit(y, i))) by (bit_vector) requires i < 64;
    }
    lemma_all_bits_clear(x ^ y);
    assert(x ^ y == 0u64 ==> x == y) by (bit_vector);
}

// ---------------------------------------------------------------------------------------------
// Accesses (Miri `Tree::perform_access`)
// ---------------------------------------------------------------------------------------------

/// The lowest of the ancestors of `e`: the root of its tree.
pub open spec fn lowest(a: AbsState, e: nat) -> nat {
    choose|r: nat| a.nodes[e].ancestors.contains(r) && forall|u: nat|
        #[trigger] a.nodes[e].ancestors.contains(u) ==> r <= u
}

/// The tags an access through `e` treats as local in its own tree: `e` and its ancestors.
pub open spec fn local_set(a: AbsState, e: nat) -> ISet<nat> {
    ISet::new(|t: nat| t < a.count && a.nodes[e].ancestors.contains(t) && !a.nodes[t].cell)
}

/// The tags an access through a tag `e` of the main tree treats as foreign: every other tag,
/// those of the wildcard trees included (Miri: `max_local_tag` is the main root, so they only
/// see foreign accesses).
pub open spec fn foreign_set(a: AbsState, e: nat) -> ISet<nat> {
    ISet::new(|t: nat| t < a.count && !a.nodes[e].ancestors.contains(t) && !a.nodes[t].cell)
}

/// An access through `tag`: a normal access to its own tree, and, from a wildcard tree, a
/// wildcard access to the others (`Tree::perform_access` then `perform_wildcard_access` for every
/// other root). `ACCESS_UNKNOWN_WILDCARD`: the checks give up (see `monitor_wildcard_masks`).
pub open spec fn access_refines(a: AbsState, b: AbsState, tag: nat, is_write: bool, code: u8)
    -> bool
{
    let e = eff(a, tag);
    if a.nodes[e].main {
        apply_refines(a, b, local_set(a, e), foreign_set(a, e), is_write, code)
    } else if a.unknown_exposed {
        code == ACCESS_UNKNOWN_EXPOSED && same_state(a, b)
    } else {
        let r = lowest(a, e);
        let own = tree_set(a, r);
        let (wc, wl, wf) = wild_access(a, own, Some(r), is_write);
        if wc != ACCESS_OK {
            code == wc && same_state(a, b)
        } else {
            code == ACCESS_UNKNOWN_WILDCARD || apply_refines(a, b, local_set(a, e).union(wl),
                ISet::new(|t: nat| own.contains(t) && !a.nodes[e].ancestors.contains(t)
                    && !a.nodes[t].cell).union(wf), is_write, code)
        }
    }
}

/// An access through a pointer created from an integer: a wildcard access to every tree.
pub open spec fn wildcard_refines(a: AbsState, b: AbsState, is_write: bool, code: u8) -> bool {
    if a.unknown_exposed {
        code == ACCESS_UNKNOWN_EXPOSED && same_state(a, b)
    } else {
        let (wc, wl, wf) = wild_access(a, ISet::empty(), None, is_write);
        if wc != ACCESS_OK {
            code == wc && same_state(a, b)
        } else {
            code == ACCESS_UNKNOWN_WILDCARD || apply_refines(a, b, wl, wf, is_write, code)
        }
    }
}

pub open spec fn settle_bits(br: u64, bw: u64, s: &MonitorState, own: u64) -> bool {
    (cand_mask(s, false) ^ br) & !own == 0 && (cand_mask(s, true) ^ bw) & !own == 0
}

/// A non-zero mask has a lowest set bit.
proof fn lemma_lowest_exists(x: u64, k: u64)
    requires
        k <= 64,
        forall|j: u64| j < k ==> !bit(x, j),
        exists|i: u64| i < 64 && bit(x, i),
    ensures exists|i: u64| i < 64 && bit(x, i) && forall|j: u64| j < i ==> !bit(x, j),
    decreases 64 - k,
{
    if k < 64 {
        if bit(x, k) {
            assert(forall|j: u64| j < k ==> !bit(x, j));
        } else {
            lemma_lowest_exists(x, (k + 1) as u64);
        }
    } else {
        let i = choose|i: u64| i < 64 && bit(x, i);
        assert(i < k);
    }
}

/// The root of the tree of a tag of a wildcard tree, as a bit, is the bit of `lowest`.
proof fn lemma_root(s: MonitorState, e: u64)
    requires monitor_wf(&s), e < s.count, !bit(s.main, e),
    ensures ({
        let a = abstraction(&s);
        let anc = s.ancestors[e as int];
        let r = lowest(a, e as nat);
        &&& r < s.count
        &&& anc & ((!anc) + 1) as u64 == 1u64 << (r as u64)
        &&& !bit(s.main, r as u64)
        &&& bit(anc, r as u64)
        &&& forall|u: u64| u < 64 && bit(anc, u) ==> bit(s.ancestors[u as int], r as u64)
        &&& !anc < u64::MAX
    }),
{
    let a = abstraction(&s);
    let anc = s.ancestors[e as int];
    assert(bit(anc, e));
    assert(anc != 0) by { lemma_bit_zero(e); }
    lemma_lowest_exists(anc, 0);
    let lo = choose|i: u64| i < 64 && bit(anc, i) && forall|j: u64| j < i ==> !bit(anc, j);
    // `lowest` is `lo`.
    assert(a.nodes[e as nat].ancestors.contains(lo as nat) && forall|u: nat|
        #[trigger] a.nodes[e as nat].ancestors.contains(u) ==> lo as nat <= u) by {
        assert forall|u: nat| #[trigger] a.nodes[e as nat].ancestors.contains(u) implies lo as nat <= u by {
            if u < lo as nat { assert(!bit(anc, u as u64)); }
        }
    }
    let r = lowest(a, e as nat);
    assert(r == lo as nat) by {
        assert(a.nodes[e as nat].ancestors.contains(r));
        assert(r < 64);
        if r < lo as nat { assert(!bit(anc, r as u64)); }
        assert(lo as nat <= r);
    }
    // Its bit.
    assert forall|i: u64| i < 64 implies bit(anc & ((!anc) + 1) as u64, i) == bit(1u64 << lo, i) by {
        lemma_lowest_bit(anc, i);
        lemma_bit_single(lo, i);
        if bit(anc, i) && forall|j: u64| j < i ==> !bit(anc, j) {
            if i < lo { } else if i > lo { assert(!bit(anc, lo)); }
        }
    }
    lemma_bits_ext(anc & ((!anc) + 1) as u64, 1u64 << lo);
    lemma_ancestor_is_tag(s, e, lo);
    assert(s.ancestors[e as int] == s.ancestors[e as int]);
    if bit(s.main, lo) {
        assert(s.ancestors[lo as int] == s.ancestors[lo as int]);
        assert(bit(s.ancestors[lo as int], 0));
        assert(bit(anc, 0));
    }
    assert forall|u: u64| u < 64 && bit(anc, u) implies bit(s.ancestors[u as int], lo) by {
        if u < lo { assert(!bit(anc, u)); }
    }
    assert(anc != 0u64 ==> !anc < u64::MAX) by (bit_vector);
}

/// The masks of an access through a tag `e` of the main tree meet `monitor_apply`'s
/// requirements.
proof fn lemma_access_main_masks(s: MonitorState, e: u64)
    requires monitor_wf(&s), e < s.count, !bit(s.cell, e),
    ensures ({
        let anc = s.ancestors[e as int];
        let l = anc & !s.cell;
        let f = !anc & !s.cell;
        &&& l & f == 0 && l & s.cell == 0 && f & s.cell == 0
        &&& l & !live_mask(s.count as u64) == 0
        &&& forall|t: u64| t < 64 ==> bit(l, t) == (bit(anc, t) && !bit(s.cell, t))
            && bit(f, t) == (!bit(anc, t) && !bit(s.cell, t))
    }),
{
    let anc = s.ancestors[e as int];
    let l = anc & !s.cell;
    let f = !anc & !s.cell;
    let c = s.cell;
    assert(l & f == 0 && l & c == 0 && f & c == 0) by (bit_vector)
        requires l == anc & !c, f == !anc & !c;
    assert forall|t: u64| t < 64 implies bit(l, t) == (bit(anc, t) && !bit(s.cell, t))
        && bit(f, t) == (!bit(anc, t) && !bit(s.cell, t)) by {
        lemma_bit_and(anc, !s.cell, t);
        lemma_bit_and(!anc, !s.cell, t);
        lemma_bit_not(anc, t);
        lemma_bit_not(s.cell, t);
    }
    lemma_and_nonzero(l, !live_mask(s.count as u64));
    assert forall|t: u64| t < 64 && bit(l, t) implies !bit(!live_mask(s.count as u64), t) by {
        lemma_ancestor_is_tag(s, e, t);
        lemma_bit_not(live_mask(s.count as u64), t);
        lemma_bit_live(s.count as u64, t);
    }
}

proof fn lemma_access_main_spec(s0: MonitorState, s: MonitorState, e: u64, tag: u8,
    is_write: bool, code: u8)
    requires
        monitor_wf(&s0), tag < s0.count, e < s0.count, !bit(s0.cell, e), bit(s0.main, e),
        e as nat == eff(abstraction(&s0), tag as nat),
        apply_refines(abstraction(&s0), abstraction(&s), mask_set(s0.ancestors[e as int] & !s0.cell),
            mask_set(!s0.ancestors[e as int] & !s0.cell), is_write, code),
    ensures access_refines(abstraction(&s0), abstraction(&s), tag as nat, is_write, code),
{
    let a = abstraction(&s0);
    lemma_access_main_masks(s0, e);
    lemma_apply_refines_ext(a, abstraction(&s), mask_set(s0.ancestors[e as int] & !s0.cell),
        mask_set(!s0.ancestors[e as int] & !s0.cell), local_set(a, e as nat),
        foreign_set(a, e as nat), is_write, code);
}

/// The tree of a tag of a wildcard tree, from `monitor_holders`.
proof fn lemma_access_wild_own(s: MonitorState, e: u64, own: u64)
    requires
        monitor_wf(&s), e < s.count, !bit(s.main, e),
        forall|i: u64| i < 64 ==> bit(own, i) == (i < s.count && s.ancestors[i as int]
            & (1u64 << (lowest(abstraction(&s), e as nat) as u64)) != 0),
    ensures ({
        let r = lowest(abstraction(&s), e as nat) as u64;
        &&& forall|v: u64| v < 64 ==> bit(own, v) == (v < s.count && bit(s.ancestors[v as int], r))
        &&& own != 0
    }),
{
    lemma_root(s, e);
    let r = lowest(abstraction(&s), e as nat) as u64;
    assert forall|v: u64| v < 64 implies bit(own, v) == (v < s.count && bit(s.ancestors[v as int], r))
    by {
        lemma_test_bit(s.ancestors[v as int], r);
    }
    assert(bit(s.ancestors[r as int], r));
    assert(bit(own, r));
    lemma_bit_zero(r);
}

/// The masks of an access through a tag `e` of a wildcard tree meet `monitor_apply`'s
/// requirements.
proof fn lemma_access_wild_masks(s: MonitorState, e: u64, own: u64, wl: u64, wf: u64)
    requires
        monitor_wf(&s), e < s.count, !bit(s.main, e), !bit(s.cell, e),
        forall|v: u64| v < 64 ==> bit(own, v) == (v < s.count && bit(s.ancestors[v as int],
            lowest(abstraction(&s), e as nat) as u64)),
        own != 0,
        wl & wf == 0, wl & s.cell == 0, wf & s.cell == 0, wl & !live_mask(s.count as u64) == 0,
        wl & own == 0, wf & own == 0,
    ensures ({
        let anc = s.ancestors[e as int];
        let l = (anc & !s.cell) | wl;
        let f = (own & !anc & !s.cell) | wf;
        &&& l & f == 0 && l & s.cell == 0 && f & s.cell == 0
        &&& l & !live_mask(s.count as u64) == 0
    }),
{
    let anc = s.ancestors[e as int];
    lemma_root(s, e);
    lemma_access_main_masks(s, e);
    // The ancestors of `e` are in its tree.
    assert forall|t: u64| t < 64 && bit(anc, t) implies bit(own, t) by {
        lemma_ancestor_is_tag(s, e, t);
    }
    let l = (anc & !s.cell) | wl;
    let f = (own & !anc & !s.cell) | wf;
    assert forall|t: u64| t < 64 implies !(bit(l, t) && bit(f, t)) && !(bit(l, t) && bit(s.cell, t))
        && !(bit(f, t) && bit(s.cell, t)) && !(bit(l, t) && bit(!live_mask(s.count as u64), t)) by {
        lemma_bit_or(anc & !s.cell, wl, t);
        lemma_bit_or(own & !anc & !s.cell, wf, t);
        lemma_bit_and(own, !anc, t);
        lemma_bit_and(own & !anc, !s.cell, t);
        lemma_bit_and(anc, !s.cell, t);
        lemma_bit_not(anc, t);
        lemma_bit_not(s.cell, t);
        lemma_and_nonzero(wl, wf);
        lemma_and_nonzero(wl, s.cell);
        lemma_and_nonzero(wf, s.cell);
        lemma_and_nonzero(wl, own);
        lemma_and_nonzero(wf, own);
        lemma_and_nonzero(wl, !live_mask(s.count as u64));
        lemma_bit_not(live_mask(s.count as u64), t);
        lemma_bit_live(s.count as u64, t);
        if bit(anc, t) { lemma_ancestor_is_tag(s, e, t); }
    }
    lemma_and_nonzero(l, f);
    lemma_and_nonzero(l, s.cell);
    lemma_and_nonzero(f, s.cell);
    lemma_and_nonzero(l, !live_mask(s.count as u64));
}

proof fn lemma_access_wild_spec(s0: MonitorState, s: MonitorState, e: u64, tag: u8,
    is_write: bool, own: u64, wl: u64, wf: u64, code: u8)
    requires
        monitor_wf(&s0), tag < s0.count, e < s0.count, !bit(s0.cell, e), !bit(s0.main, e),
        !s0.unknown_exposed,
        e as nat == eff(abstraction(&s0), tag as nat),
        forall|v: u64| v < 64 ==> bit(own, v) == (v < s0.count && bit(s0.ancestors[v as int],
            lowest(abstraction(&s0), e as nat) as u64)),
        own != 0,
        wild_bits(&s0, is_write, own, 1u64 << (lowest(abstraction(&s0), e as nat) as u64))
            == (ACCESS_OK, wl, wf),
        apply_refines(abstraction(&s0), abstraction(&s),
            mask_set((s0.ancestors[e as int] & !s0.cell) | wl),
            mask_set((own & !s0.ancestors[e as int] & !s0.cell) | wf), is_write, code),
    ensures access_refines(abstraction(&s0), abstraction(&s), tag as nat, is_write, code),
{
    let a = abstraction(&s0);
    let r = lowest(a, e as nat);
    lemma_root(s0, e);
    assert((r as u64) as nat == r);
    lemma_wild_meaning(s0, is_write, own, 1u64 << (r as u64), r as u64);
    assert(wild_own(a, own, r as u64) == tree_set(a, r));
    assert(wild_root(own, r as u64) == Some(r));
    let own_set = tree_set(a, r);
    let (wc, l2, f2) = wild_access(a, own_set, Some(r), is_write);
    let anc = s0.ancestors[e as int];
    let lspec = local_set(a, e as nat).union(l2);
    let fspec = ISet::new(|t: nat| own_set.contains(t) && !a.nodes[e as nat].ancestors.contains(t)
        && !a.nodes[t].cell).union(f2);
    assert forall|t: nat| t < a.count implies mask_set((anc & !s0.cell) | wl).contains(t)
        == lspec.contains(t) && mask_set((own & !anc & !s0.cell) | wf).contains(t)
        == fspec.contains(t) by {
        let u = t as u64;
        assert(mask_set(wl).contains(t) == l2.contains(t));
        assert(mask_set(wf).contains(t) == f2.contains(t));
        lemma_bit_or(anc & !s0.cell, wl, u);
        lemma_bit_or(own & !anc & !s0.cell, wf, u);
        lemma_bit_and(own, !anc, u);
        lemma_bit_and(own & !anc, !s0.cell, u);
        lemma_bit_and(anc, !s0.cell, u);
        lemma_bit_not(anc, u);
        lemma_bit_not(s0.cell, u);
        assert(own_set.contains(t) == (t < a.count && a.nodes[t].ancestors.contains(r)));
    }
    lemma_apply_refines_ext(a, abstraction(&s), mask_set((anc & !s0.cell) | wl),
        mask_set((own & !anc & !s0.cell) | wf), lspec, fspec, is_write, code);
}

/// `r` is the mask of the tags whose ancestors meet `b`.
pub open spec fn holders_spec(s: &MonitorState, b: u64, r: u64) -> bool {
    forall|i: u64| i < 64 ==> bit(r, i) == (i < s.count && s.ancestors[i as int] & b != 0)
}

proof fn lemma_wildcard_spec(s0: MonitorState, s: MonitorState, is_write: bool, l: u64, f: u64,
    code: u8)
    requires
        monitor_wf(&s0), !s0.unknown_exposed,
        wild_bits(&s0, is_write, 0, 0) == (ACCESS_OK, l, f),
        apply_refines(abstraction(&s0), abstraction(&s), mask_set(l), mask_set(f), is_write, code),
    ensures
        code == ACCESS_UNKNOWN_WILDCARD || wildcard_refines(abstraction(&s0), abstraction(&s),
            is_write, code),
        wildcard_refines(abstraction(&s0), abstraction(&s), is_write, ACCESS_UNKNOWN_WILDCARD),
{
    let a = abstraction(&s0);
    lemma_wild_meaning(s0, is_write, 0, 0, 0);
    assert(wild_own(a, 0, 0) == ISet::<nat>::empty());
    assert(wild_root(0, 0) == None::<nat>);
    let w = wild_access(a, ISet::empty(), None, is_write);
    lemma_apply_refines_ext(a, abstraction(&s), mask_set(l), mask_set(f), w.1, w.2, is_write, code);
}

// ---------------------------------------------------------------------------------------------
// Retags (Miri `NewPermission` and `tb_reborrow`)
// ---------------------------------------------------------------------------------------------

/// The node a retag creates as tag `n` at the tracked location, from a pointer tagged `parent`
/// (or created from an integer: `EXPOSED_TAG`, which starts a wildcard tree). In the range
/// (`covers`) the permission depends on whether the byte is in an `UnsafeCell` (`in_cell`),
/// outside it on whether the whole type is `Freeze` (`freeze`): a shared reference gets Frozen,
/// or `Cell`; a mutable reference or `Box` gets Reserved, interior-mutable unless protected or
/// frozen. Only the in-range part of a non-`Cell` permission is accessed (initialized).
pub open spec fn retag_node(a: AbsState, parent: nat, n: nat, covers: bool, in_cell: bool,
    freeze: bool, is_shared: bool, is_protected: bool, is_strong: bool) -> Node
{
    let cellish = if covers { in_cell } else { !freeze };
    let is_cell = is_shared && cellish;
    let wild = parent == EXPOSED_TAG as nat;
    Node {
        ancestors: if wild { ISet::empty().insert(n) } else { a.nodes[parent].ancestors.insert(n) },
        main: !wild && a.nodes[parent].main,
        perm: if is_shared && !is_cell {
            Perm::Frozen
        } else {
            Perm::Reserved { conflicted: false, interior: !is_shared && cellish && !is_protected }
        },
        protected: is_protected && !is_cell,
        strong: is_protected && is_strong && !is_cell,
        initialized: covers && !is_cell,
        cell: is_cell,
        alias: if is_cell { eff(a, parent) } else { n },
        exposed: false,
        stored: false,
    }
}

/// `exposed_values`: a pointer tagged `EXPOSED_TAG` may exist; otherwise `parent` cannot be
/// that tag, and the reference gets the unknown tag if it is.
pub open spec fn retag_spec(a: AbsState, b: AbsState, parent: nat, covers: bool, in_cell: bool,
    freeze: bool, is_shared: bool, is_protected: bool, is_strong: bool, r: nat,
    exposed_values: bool) -> bool
{
    let cellish = if covers { in_cell } else { !freeze };
    let is_cell = is_shared && cellish;
    let wild = exposed_values && parent == EXPOSED_TAG as nat;
    if parent >= a.count && !wild {
        r == UNKNOWN_TAG as nat && same_state(a, b)
    } else if a.count == 64 {
        r == UNKNOWN_TAG as nat && same_state(a, b)
    } else if is_cell && wild {
        r == UNKNOWN_TAG as nat && same_state(a, b)
    } else {
        &&& r == a.count
        &&& b.count == a.count + 1
        &&& b.unknown_exposed == a.unknown_exposed
        &&& forall|t: nat| t < a.count ==> node_eq(#[trigger] b.nodes[t], a.nodes[t])
        &&& node_eq(b.nodes[r], retag_node(a, parent, r, covers, in_cell, freeze, is_shared,
            is_protected, is_strong))
    }
}

#[verifier::rlimit(100)]
proof fn lemma_new_borrow(s0: MonitorState, s: MonitorState, parent: u8, covers: bool,
    in_cell: bool, freeze: bool, is_shared: bool, is_protected: bool, is_strong: bool)
    requires
        monitor_wf(&s0),
        parent < s0.count || (parent == EXPOSED_TAG && s0.exposed_values),
        s0.count < 64,
        ({
            let t = s0.count as u64;
            let b = 1u64 << t;
            let cellish = if covers { in_cell } else { !freeze };
            let is_cell = is_shared && cellish;
            let wild = parent == EXPOSED_TAG;
            &&& !(is_cell && wild)
            &&& s.count == s0.count + 1
            &&& s.tracked_object_id == s0.tracked_object_id
            &&& s.tracked_offset == s0.tracked_offset
            &&& s.unknown_exposed == s0.unknown_exposed
            &&& s.wildcard == (s0.wildcard || wild)
            &&& s.exposed_values == s0.exposed_values
            &&& s.ancestors@ == s0.ancestors@.update(t as int, if wild { b } else {
                    s0.ancestors[parent as int] | b })
            &&& s.main == if !wild && s0.main & (1u64 << parent) != 0 { s0.main | b } else {
                    s0.main & !b }
            &&& s.valid == s0.valid | b
            &&& s.active == s0.active & !b
            &&& s.conflicted == s0.conflicted & !b
            &&& s.exposed == s0.exposed & !b
            &&& s.stored == s0.stored & !b
            &&& s.alias@ == s0.alias@.update(t as int, if is_cell {
                    if s0.cell & (1u64 << parent) != 0 { s0.alias[parent as int] } else { parent }
                } else {
                    t as u8
                })
            &&& s.cell == if is_cell { s0.cell | b } else { s0.cell & !b }
            &&& s.interior == if !is_shared && cellish && !is_protected { s0.interior | b } else {
                    s0.interior & !b }
            &&& s.initialized == if covers && !is_cell { s0.initialized | b } else {
                    s0.initialized & !b }
            &&& s.protected == if is_protected && !is_cell { s0.protected | b } else {
                    s0.protected & !b }
            &&& s.strong == if is_protected && is_strong && !is_cell { s0.strong | b } else {
                    s0.strong & !b }
            &&& s.frozen == if is_shared && !is_cell { s0.frozen | b } else { s0.frozen & !b }
        }),
    ensures
        monitor_wf(&s),
        retag_spec(abstraction(&s0), abstraction(&s), parent as nat, covers, in_cell, freeze,
            is_shared, is_protected, is_strong, s0.count as nat, s0.exposed_values),
{
    if !s.wildcard {
        let c = s0.count as u64;
        lemma_bit_live(c, parent as u64);
        lemma_test_bit(s0.main, parent as u64);
        lemma_live_extend(c);
        assert(s.main == live_mask(s.count as u64));
    }
    let t = s0.count as u64;
    let p = parent as u64;
    let wild = parent == EXPOSED_TAG;
    let cellish = if covers { in_cell } else { !freeze };
    let is_cell = is_shared && cellish;
    let is_interior = !is_shared && cellish && !is_protected;
    let is_initialized = covers && !is_cell;
    let is_prot = is_protected && !is_cell;
    let is_strong_p = is_protected && is_strong && !is_cell;
    let is_frozen = is_shared && !is_cell;
    let in_main = !wild && bit(s0.main, p);
    if !wild { lemma_test_bit(s0.main, p); lemma_test_bit(s0.cell, p); }
    assert forall|i: u64| i < 64 implies {
        &&& bit(s.valid, i) == (i == t || bit(s0.valid, i))
        &&& bit(s.active, i) == (i != t && bit(s0.active, i))
        &&& bit(s.conflicted, i) == (i != t && bit(s0.conflicted, i))
        &&& bit(s.exposed, i) == (i != t && bit(s0.exposed, i))
        &&& bit(s.stored, i) == (i != t && bit(s0.stored, i))
        &&& bit(s.main, i) == if i == t { in_main } else { bit(s0.main, i) }
        &&& bit(s.cell, i) == if i == t { is_cell } else { bit(s0.cell, i) }
        &&& bit(s.interior, i) == if i == t { is_interior } else { bit(s0.interior, i) }
        &&& bit(s.initialized, i) == if i == t { is_initialized } else { bit(s0.initialized, i) }
        &&& bit(s.protected, i) == if i == t { is_prot } else { bit(s0.protected, i) }
        &&& bit(s.strong, i) == if i == t { is_strong_p } else { bit(s0.strong, i) }
        &&& bit(s.frozen, i) == if i == t { is_frozen } else { bit(s0.frozen, i) }
    } by {
        lemma_bit_set(s0.valid, t, i);
        lemma_bit_set(s0.active, t, i);
        lemma_bit_set(s0.conflicted, t, i);
        lemma_bit_set(s0.exposed, t, i);
        lemma_bit_set(s0.stored, t, i);
        lemma_bit_set(s0.main, t, i);
        lemma_bit_set(s0.cell, t, i);
        lemma_bit_set(s0.interior, t, i);
        lemma_bit_set(s0.initialized, t, i);
        lemma_bit_set(s0.protected, t, i);
        lemma_bit_set(s0.strong, t, i);
        lemma_bit_set(s0.frozen, t, i);
    }
    let ap = if wild { 0u64 } else { s0.ancestors[p as int] };
    assert(s.ancestors[t as int] == ap | (1u64 << t)) by {
        if wild { lemma_or_zero(1u64 << t); assert((0u64 | (1u64 << t)) == (1u64 << t)) by (bit_vector); }
    }
    assert forall|i: u64| i < 64 implies bit(s.ancestors[t as int], i) == (i == t || bit(ap, i)) by {
        lemma_bit_set(ap, t, i);
        if wild { lemma_bit_zero(i); }
    }
    if !wild {
        assert(bit(ap, p));
        assert forall|u: u64| u < 64 && bit(ap, u) implies u <= p by {}
    }
    assert forall|u: u64| u < t implies s.ancestors[u as int] == s0.ancestors[u as int]
        && s.alias[u as int] == s0.alias[u as int] by {}
    // wf: the trees.
    assert forall|x: u64| #![trigger s.ancestors[x as int]] x < s.count implies {
        &&& bit(s.ancestors[x as int], x)
        &&& forall|u: u64| u < 64 && bit(s.ancestors[x as int], u) ==> u <= x
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
            && bit(s.ancestors[u as int], w) ==> bit(s.ancestors[x as int], w)
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
            && bit(s.ancestors[x as int], w) && u <= w ==> bit(s.ancestors[w as int], u)
        &&& bit(s.main, x) == bit(s.ancestors[x as int], 0)
    } by {
        if x < t {
            assert(s.ancestors[x as int] == s0.ancestors[x as int]);
            assert(s0.ancestors[x as int] == s0.ancestors[x as int]);
            lemma_bit_set(s0.main, t, x);
            if !wild && bit(s0.main, p) { assert(bit(s.main, x) == bit(s0.main, x)); }
            assert forall|u: u64| u < 64 && bit(s.ancestors[x as int], u) implies u <= x by {}
            assert forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
                && bit(s.ancestors[u as int], w) implies bit(s.ancestors[x as int], w) by {
                assert(u <= x);
                assert(s.ancestors[u as int] == s0.ancestors[u as int]);
            }
            assert forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
                && bit(s.ancestors[x as int], w) && u <= w implies bit(s.ancestors[w as int], u) by {
                assert(w <= x);
                assert(s.ancestors[w as int] == s0.ancestors[w as int]);
            }
        } else {
            assert(x == t);
            lemma_bit_set(ap, t, t);
            assert forall|u: u64| u < 64 && bit(s.ancestors[x as int], u) implies u <= x by {
                if u != t {
                    assert(bit(ap, u));
                    if wild {
                        lemma_bit_zero(u);
                    } else {
                        assert(s0.ancestors[p as int] == s0.ancestors[p as int]);
                        assert(u <= p);
                    }
                }
            }
            assert forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
                && bit(s.ancestors[u as int], w) implies bit(s.ancestors[x as int], w) by {
                if u != t {
                    assert(bit(ap, u));
                    if wild { lemma_bit_zero(u); }
                    assert(s0.ancestors[p as int] == s0.ancestors[p as int]);
                    assert(u <= p);
                    assert(s.ancestors[u as int] == s0.ancestors[u as int]);
                    assert(s0.ancestors[p as int] == s0.ancestors[p as int]);
                }
            }
            assert forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
                && bit(s.ancestors[x as int], w) && u <= w implies bit(s.ancestors[w as int], u) by {
                if w == t {
                } else {
                    assert(bit(ap, w));
                    if wild { lemma_bit_zero(w); }
                    assert(s0.ancestors[p as int] == s0.ancestors[p as int]);
                    assert(w <= p);
                    if u == t { assert(false); }
                    assert(bit(ap, u));
                    assert(s.ancestors[w as int] == s0.ancestors[w as int]);
                    assert(s0.ancestors[p as int] == s0.ancestors[p as int]);
                }
            }
            lemma_bit_set(ap, t, 0);
            if !wild {
                assert(s0.ancestors[p as int] == s0.ancestors[p as int]);
                assert(bit(s0.main, p) == bit(s0.ancestors[p as int], 0));
                assert(t != 0);
                assert(bit(s.ancestors[x as int], 0) == bit(ap, 0));
                assert(bit(s.main, x) == in_main);
            } else {
                lemma_bit_zero(0);
                assert(t != 0);
                assert(!bit(s.ancestors[x as int], 0));
                assert(!bit(s.main, x));
            }
        }
            assert(bit(s.ancestors[x as int], x));
        assert(forall|u: u64| u < 64 && bit(s.ancestors[x as int], u) ==> u <= x);
        assert(forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
            && bit(s.ancestors[u as int], w) ==> bit(s.ancestors[x as int], w));
        assert(forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[x as int], u)
            && bit(s.ancestors[x as int], w) && u <= w ==> bit(s.ancestors[w as int], u));
        assert(bit(s.main, x) == bit(s.ancestors[x as int], 0));
    }
    // wf: aliases.
    assert forall|x: u64| #![trigger s.alias[x as int]] x < s.count implies if bit(s.cell, x) {
        &&& s.alias[x as int] < x
        &&& !bit(s.cell, s.alias[x as int] as u64)
        &&& bit(s.ancestors[x as int], s.alias[x as int] as u64)
    } else {
        s.alias[x as int] == x
    } by {
        if x < t {
            assert(s.alias[x as int] == s0.alias[x as int]);
            if bit(s0.cell, x) {
                assert(s0.alias[x as int] < x);
            }
        } else if is_cell && bit(s0.cell, p) {
            let q = s0.alias[p as int] as u64;
            assert(s0.alias[p as int] == s0.alias[p as int]);
            assert(q < p && !bit(s0.cell, q) && bit(ap, q));
        }
    }
    assert forall|i: u64| t + 1 <= i < 64 implies !bit(s.valid, i) && !bit(s.active, i)
        && !bit(s.frozen, i) && !bit(s.conflicted, i) && !bit(s.interior, i)
        && !bit(s.protected, i) && !bit(s.strong, i) && !bit(s.initialized, i)
        && !bit(s.cell, i) && !bit(s.exposed, i) && !bit(s.main, i) && !bit(s.stored, i) by {}
    // The refinement.
    let a = abstraction(&s0);
    let b = abstraction(&s);
    assert forall|x: nat| x < a.count implies node_eq(#[trigger] b.nodes[x], a.nodes[x]) by {
        assert(s.ancestors[x as int] == s0.ancestors[x as int]);
        assert(mask_set(s.ancestors[x as int]) =~= mask_set(s0.ancestors[x as int]));
    }
    if wild {
        assert forall|i: nat| b.nodes[t as nat].ancestors.contains(i) == (i == t as nat) by {
            if i < 64 { lemma_bit_set(ap, t, i as u64); lemma_bit_zero(i as u64); }
        }
        assert(b.nodes[t as nat].ancestors =~= ISet::<nat>::empty().insert(t as nat));
    } else {
        assert(b.nodes[t as nat].ancestors =~= a.nodes[parent as nat].ancestors.insert(t as nat));
    }
}

// ---------------------------------------------------------------------------------------------
// Exposure (Miri `expose_tag`)
// ---------------------------------------------------------------------------------------------

/// `b` is `a` with the nodes of the tags in `changed` replaced; the others are unchanged.
pub open spec fn same_nodes_except(a: AbsState, b: AbsState, changed: ISet<nat>) -> bool {
    forall|t: nat| t < a.count && !changed.contains(t) ==> node_eq(#[trigger] b.nodes[t], a.nodes[t])
}

/// Exposing the pointer tagged `tag` exposes the tag accesses through it act through; exposing a
/// pointer with an unknown tag is recorded as such.
pub open spec fn expose_spec(a: AbsState, b: AbsState, tag: nat) -> bool {
    &&& b.count == a.count
    &&& if tag < a.count {
        &&& b.unknown_exposed == a.unknown_exposed
        &&& same_nodes_except(a, b, iset![eff(a, tag)])
        &&& node_eq(b.nodes[eff(a, tag)], Node { exposed: true, ..a.nodes[eff(a, tag)] })
    } else {
        &&& b.unknown_exposed == (a.unknown_exposed || tag != EXPOSED_TAG)
        &&& same_nodes_except(a, b, ISet::empty())
    }
}

proof fn lemma_expose(s0: MonitorState, s: MonitorState, tag: u8, e: u8)
    requires
        monitor_wf(&s0), tag < s0.count, e < s0.count, !bit(s0.cell, e as u64),
        e as nat == eff(abstraction(&s0), tag as nat),
        s == (MonitorState { exposed: s0.exposed | (1u64 << e), ..s0 }),
    ensures monitor_wf(&s), expose_spec(abstraction(&s0), abstraction(&s), tag as nat),
{
    assert forall|i: u64| i < 64 implies bit(s.exposed, i) == (i == e || bit(s0.exposed, i)) by {
        lemma_bit_set(s0.exposed, e as u64, i);
    }
    let a = abstraction(&s0);
    let b = abstraction(&s);
    assert forall|t: nat| t < a.count && !iset![eff(a, tag as nat)].contains(t) implies
        node_eq(#[trigger] b.nodes[t], a.nodes[t]) by {}
}

// ---------------------------------------------------------------------------------------------
// Stage 10: deallocation (Miri `Tree::dealloc`)
// ---------------------------------------------------------------------------------------------

/// A strongly protected tag has an initialized permission (`ProtectedDealloc`; `Cell`s excepted).
pub open spec fn strong_ub(m: AbsState) -> bool {
    exists|t: nat| t < m.count && {
        let n = #[trigger] m.nodes[t];
        n.protected && n.strong && n.initialized && !n.cell
    }
}

/// Deallocating through `tag`: a write access through it (a wildcard one for a pointer created
/// from an integer), then undefined behavior if a strongly protected tag has an initialized
/// permission.
pub open spec fn dealloc_refines(a: AbsState, b: AbsState, tag: nat, code: u8,
    exposed_values: bool) -> bool {
    exists|m: AbsState, c1: u8| #[trigger] dealloc_witness(a, b, tag, code, m, c1, exposed_values)
}

/// The state `m` after the write access, whose result is `c1`.
pub open spec fn dealloc_witness(a: AbsState, b: AbsState, tag: nat, code: u8, m: AbsState, c1: u8,
    exposed_values: bool) -> bool
{
    {
        &&& if exposed_values && tag == EXPOSED_TAG as nat {
            wildcard_refines(a, m, true, c1)
        } else if tag < a.count {
            access_refines(a, m, tag, true, c1)
        } else {
            c1 == ACCESS_UNKNOWN_TAG && same_state(a, m)
        }
        &&& same_state(m, b)
        &&& code == if c1 != ACCESS_OK { c1 } else if strong_ub(m) { ACCESS_PROTECTED_DEALLOC }
            else { ACCESS_OK }
    }
}

proof fn lemma_same_state_refl(a: AbsState)
    ensures same_state(a, a),
{
    assert forall|t: nat| t < a.count implies node_eq(#[trigger] a.nodes[t], a.nodes[t]) by {
        assert(a.nodes[t].ancestors =~= a.nodes[t].ancestors);
    }
}

proof fn lemma_dealloc(s0: MonitorState, s: MonitorState, tag: u8, code: u8)
    requires
        monitor_wf(&s0), monitor_wf(&s),
        s0.exposed_values && tag == EXPOSED_TAG ==> wildcard_refines(abstraction(&s0),
            abstraction(&s), true, code),
        !(s0.exposed_values && tag == EXPOSED_TAG) && tag < s0.count ==> access_refines(
            abstraction(&s0), abstraction(&s), tag as nat, true, code),
        !(s0.exposed_values && tag == EXPOSED_TAG) && tag >= s0.count ==> code
            == ACCESS_UNKNOWN_TAG && s == s0,
    ensures
        code != ACCESS_OK ==> dealloc_refines(abstraction(&s0), abstraction(&s), tag as nat, code,
            s0.exposed_values),
        code == ACCESS_OK ==> dealloc_refines(abstraction(&s0), abstraction(&s), tag as nat,
            if s.protected & s.strong & s.initialized & !s.cell != 0 { ACCESS_PROTECTED_DEALLOC }
            else { ACCESS_OK }, s0.exposed_values),
{
    let a = abstraction(&s0);
    let m = abstraction(&s);
    lemma_same_state_refl(m);
    lemma_same_state_refl(a);
    let x = s.protected & s.strong & s.initialized & !s.cell;
    assert forall|t: u64| t < 64 implies bit(x, t) == (bit(s.protected, t) && bit(s.strong, t)
        && bit(s.initialized, t) && !bit(s.cell, t)) by {
        lemma_bit_and(s.protected, s.strong, t);
        lemma_bit_and(s.protected & s.strong, s.initialized, t);
        lemma_bit_and(s.protected & s.strong & s.initialized, !s.cell, t);
        lemma_bit_not(s.cell, t);
    }
    assert((x != 0) == strong_ub(m)) by {
        if x != 0 {
            lemma_nonzero_witness(x);
            let t = choose|t: u64| t < 64 && bit(x, t);
            assert(t < s.count) by { if t >= s.count { assert(!bit(s.protected, t)); } }
            assert(m.nodes[t as nat].protected);
        } else {
            assert forall|t: nat| t < m.count implies !({
                let n = #[trigger] m.nodes[t];
                n.protected && n.strong && n.initialized && !n.cell
            }) by {
                lemma_bit_zero(t as u64);
            }
        }
    }
    if code != ACCESS_OK {
        assert(dealloc_witness(a, m, tag as nat, code, m, code, s0.exposed_values));
    } else {
        assert(dealloc_witness(a, m, tag as nat, if x != 0 { ACCESS_PROTECTED_DEALLOC }
            else { ACCESS_OK }, m, code, s0.exposed_values));
    }
}

// ---------------------------------------------------------------------------------------------
// Stage 9: the end of a protector (Miri `perform_protector_end_access`)
// ---------------------------------------------------------------------------------------------

/// The descendants of `p`, other than `p`.
pub open spec fn desc_of(a: AbsState, p: nat, v: nat) -> bool {
    v < a.count && v != p && a.nodes[v].ancestors.contains(p)
}

/// The tags of the subtree of `p`, `p` included, that are exposed.
pub open spec fn exposed_mine(a: AbsState, p: nat, v: nat) -> bool {
    (v == p || desc_of(a, p, v)) && a.nodes[v].exposed
}

/// The smallest of them (Miri's `min_exposed_child`).
pub open spec fn min_exposed(a: AbsState, p: nat) -> nat {
    choose|m: nat| exposed_mine(a, p, m) && forall|v: nat| #[trigger] exposed_mine(a, p, v) ==> m <= v
}

/// Whether `t` is in a wildcard tree that Miri skips on the protector end access of `p`: a tree
/// rooted above the smallest exposed tag of `p`'s subtree, which may hang below `p`.
pub open spec fn end_skipped(a: AbsState, p: nat, t: nat) -> bool {
    &&& exists|v: nat| #[trigger] exposed_mine(a, p, v)
    &&& !a.nodes[t].main
    &&& forall|u: nat| #[trigger] a.nodes[t].ancestors.contains(u) ==> u > min_exposed(a, p)
}

/// When the protector of `p` ends and its permission is initialized, an access through it that
/// its descendants do not see: a write if it is Active, a read otherwise. The wildcard trees
/// that may hang below it are skipped too (`end_skipped`).
pub open spec fn end_protector_refines(a: AbsState, b: AbsState, p: nat, code: u8) -> bool {
    let n = a.nodes[p];
    if !(p < a.count && n.protected && n.initialized && n.perm != Perm::Disabled && !n.cell) {
        code == ACCESS_OK && same_state(a, b)
    } else if !n.main {
        code == ACCESS_UNKNOWN_WILDCARD && same_state(a, b)
    } else {
        apply_refines(a, b, local_set(a, p), ISet::new(|t: nat| t < a.count
            && !a.nodes[p].ancestors.contains(t) && !desc_of(a, p, t) && !a.nodes[t].cell
            && !end_skipped(a, p, t)), n.perm == Perm::Active, code)
    }
}

proof fn lemma_end_protector_tests(s: MonitorState, p: u64)
    requires monitor_wf(&s), p < s.count,
    ensures
        (s.protected & s.initialized & s.valid & !s.cell & (1u64 << p) == 0) == !(bit(s.protected, p)
            && bit(s.initialized, p) && bit(s.valid, p) && !bit(s.cell, p)),
        (s.main & (1u64 << p) == 0) == !bit(s.main, p),
        (s.active & !s.frozen & (1u64 << p) != 0) == (bit(s.active, p) && !bit(s.frozen, p)),
{
    let x = s.protected & s.initialized & s.valid & !s.cell;
    lemma_test_bit(x, p);
    lemma_bit_and(s.protected, s.initialized, p);
    lemma_bit_and(s.protected & s.initialized, s.valid, p);
    lemma_bit_and(s.protected & s.initialized & s.valid, !s.cell, p);
    lemma_bit_not(s.cell, p);
    lemma_test_bit(s.main, p);
    lemma_test_bit(s.active & !s.frozen, p);
    lemma_bit_and(s.active, !s.frozen, p);
    lemma_bit_not(s.frozen, p);
}

/// Bit `i` of the tags up to `m`, from the bit of `m`.
pub proof fn lemma_bit_upto(m: u64, i: u64)
    requires m < 64, i < 64,
    ensures bit((1u64 << m) | (((1u64 << m) - 1u64) as u64), i) == (i <= m),
{
    assert(bit((1u64 << m) | (((1u64 << m) - 1u64) as u64), i) == (i <= m)) by (bit_vector)
        requires m < 64, i < 64;
}

proof fn lemma_end_protector_masks(s: MonitorState, p: u64, d: u64, mine: u64, skip: u64)
    requires
        monitor_wf(&s), p < s.count, !bit(s.cell, p),
        exists|h: u64| #[trigger] holders_spec(&s, 1u64 << p, h) && d == h & !(1u64 << p),
        mine == (d | (1u64 << p)) & s.exposed,
        mine == 0 ==> skip == 0,
        mine != 0 ==> exists|h2: u64| holders_spec(&s, (mine & ((!mine) + 1) as u64)
            | (((mine & ((!mine) + 1) as u64) - 1u64) as u64), h2) && skip == !s.main & !h2,
    ensures ({
        let a = abstraction(&s);
        let anc = s.ancestors[p as int];
        let l = anc & !s.cell;
        let f = !anc & !d & !s.cell & !skip;
        &&& forall|v: u64| v < 64 ==> bit(d, v) == desc_of(a, p as nat, v as nat)
        &&& l & f == 0 && l & s.cell == 0 && f & s.cell == 0
        &&& l & !live_mask(s.count as u64) == 0
        &&& forall|t: u64| t < s.count ==> #[trigger] bit(f, t) == (!bit(anc, t) && !bit(d, t)
            && !bit(s.cell, t) && !end_skipped(a, p as nat, t as nat))
    }),
{
    let a = abstraction(&s);
    let h = choose|h: u64| #[trigger] holders_spec(&s, 1u64 << p, h) && d == h & !(1u64 << p);
    let anc = s.ancestors[p as int];
    assert forall|v: u64| v < 64 implies bit(d, v) == desc_of(a, p as nat, v as nat) by {
        lemma_bit_and(h, !(1u64 << p), v);
        lemma_bit_not(1u64 << p, v);
        lemma_bit_single(p, v);
        lemma_test_bit(s.ancestors[v as int], p);
    }
    // The exposed tags of the subtree.
    assert forall|v: u64| v < 64 implies bit(mine, v) == exposed_mine(a, p as nat, v as nat) by {
        lemma_bit_and(d | (1u64 << p), s.exposed, v);
        lemma_bit_or(d, 1u64 << p, v);
        lemma_bit_single(p, v);
        if v >= s.count { assert(!bit(s.exposed, v)); }
    }
    assert forall|v: nat| #[trigger] exposed_mine(a, p as nat, v) implies v < 64 && bit(mine, v as u64) by {}
    // The skipped tags.
    assert forall|t: u64| t < s.count implies bit(skip, t) == end_skipped(a, p as nat, t as nat) by {
        if mine == 0 {
            lemma_bit_zero(t);
            if exists|v: nat| #[trigger] exposed_mine(a, p as nat, v) {
                let v = choose|v: nat| #[trigger] exposed_mine(a, p as nat, v);
                lemma_bit_zero(v as u64);
            }
        } else {
            lemma_nonzero_witness(mine);
            lemma_lowest_exists(mine, 0);
            let m = choose|i: u64| i < 64 && bit(mine, i) && forall|j: u64| j < i ==> !bit(mine, j);
            let low = mine & ((!mine) + 1) as u64;
            assert(!mine < u64::MAX) by {
                assert(mine != 0u64 ==> !mine < u64::MAX) by (bit_vector);
            }
            assert forall|i: u64| i < 64 implies bit(low, i) == bit(1u64 << m, i) by {
                lemma_lowest_bit(mine, i);
                lemma_bit_single(m, i);
                if bit(mine, i) && forall|j: u64| j < i ==> !bit(mine, j) {
                    if i < m { } else if i > m { assert(!bit(mine, m)); }
                }
            }
            lemma_bits_ext(low, 1u64 << m);
            // `m` is the smallest exposed tag of the subtree.
            assert(exposed_mine(a, p as nat, m as nat) && forall|v: nat|
                #[trigger] exposed_mine(a, p as nat, v) ==> m as nat <= v) by {
                assert forall|v: nat| #[trigger] exposed_mine(a, p as nat, v) implies m as nat <= v by {
                    if v < m as nat { assert(!bit(mine, v as u64)); }
                }
            }
            let mn = min_exposed(a, p as nat);
            assert(mn == m as nat) by {
                assert(exposed_mine(a, p as nat, mn));
                assert(mn < 64 && bit(mine, mn as u64));
                if mn < m as nat { assert(!bit(mine, mn as u64)); }
            }
            let upto = low | ((low - 1u64) as u64);
            let h2 = choose|h2: u64| holders_spec(&s, upto, h2) && skip == !s.main & !h2;
            lemma_bit_and(!s.main, !h2, t);
            lemma_bit_not(s.main, t);
            lemma_bit_not(h2, t);
            lemma_and_nonzero(s.ancestors[t as int], upto);
            assert forall|i: u64| i < 64 implies bit(upto, i) == (i <= m) by { lemma_bit_upto(m, i); }
            if bit(h2, t) {
                let u = choose|u: u64| u < 64 && bit(s.ancestors[t as int], u) && bit(upto, u);
                assert(a.nodes[t as nat].ancestors.contains(u as nat));
            } else {
                assert forall|u: nat| #[trigger] a.nodes[t as nat].ancestors.contains(u) implies
                    u > m as nat by {
                    if u <= m as nat { assert(bit(upto, u as u64)); }
                }
            }
            assert(exposed_mine(a, p as nat, m as nat));
        }
    }
    lemma_access_main_masks(s, p);
    let l = anc & !s.cell;
    let f = !anc & !d & !s.cell & !skip;
    assert forall|t: u64| t < 64 implies bit(f, t) == (!bit(anc, t) && !bit(d, t) && !bit(s.cell, t)
        && !bit(skip, t)) && !(bit(l, t) && bit(f, t)) && !(bit(f, t) && bit(s.cell, t)) by {
        lemma_bit_and(!anc, !d, t);
        lemma_bit_and(!anc & !d, !s.cell, t);
        lemma_bit_and(!anc & !d & !s.cell, !skip, t);
        lemma_bit_not(anc, t);
        lemma_bit_not(d, t);
        lemma_bit_not(s.cell, t);
        lemma_bit_not(skip, t);
    }
    lemma_and_nonzero(l, f);
    lemma_and_nonzero(f, s.cell);
}

proof fn lemma_end_protector_spec(s0: MonitorState, s: MonitorState, p: u64, d: u64, skip: u64,
    is_write: bool, code: u8)
    requires
        monitor_wf(&s0), p < s0.count, !bit(s0.cell, p), bit(s0.main, p), bit(s0.valid, p),
        bit(s0.protected, p), bit(s0.initialized, p),
        is_write == (bit(s0.active, p) && !bit(s0.frozen, p)),
        forall|t: u64| t < s0.count ==> #[trigger] bit(!s0.ancestors[p as int] & !d & !s0.cell & !skip, t)
            == (!bit(s0.ancestors[p as int], t) && !bit(d, t) && !bit(s0.cell, t)
                && !end_skipped(abstraction(&s0), p as nat, t as nat)),
        forall|v: u64| v < 64 ==> bit(d, v) == desc_of(abstraction(&s0), p as nat, v as nat),
        apply_refines(abstraction(&s0), abstraction(&s), mask_set(s0.ancestors[p as int] & !s0.cell),
            mask_set(!s0.ancestors[p as int] & !d & !s0.cell & !skip), is_write, code),
    ensures
        end_protector_refines(abstraction(&s0), abstraction(&s), p as nat, code),
{
    let a = abstraction(&s0);
    let anc = s0.ancestors[p as int];
    let fs = ISet::new(|t: nat| t < a.count && !a.nodes[p as nat].ancestors.contains(t)
        && !desc_of(a, p as nat, t) && !a.nodes[t].cell && !end_skipped(a, p as nat, t));
    lemma_access_main_masks(s0, p);
    assert forall|t: nat| t < a.count implies mask_set(!anc & !d & !s0.cell & !skip).contains(t)
        == fs.contains(t) by {
        assert((t as u64) as nat == t);
        assert((t as u64) < s0.count);
        assert(bit(!anc & !d & !s0.cell & !skip, t as u64) == (!bit(anc, t as u64) && !bit(d, t as u64)
            && !bit(s0.cell, t as u64) && !end_skipped(a, p as nat, (t as u64) as nat)));
    }
    lemma_apply_refines_ext(a, abstraction(&s), mask_set(anc & !s0.cell), mask_set(!anc & !d
        & !s0.cell & !skip), local_set(a, p as nat), fs, is_write, code);
}

// ---------------------------------------------------------------------------------------------
// Returns: protectors end, unreachable tags are dropped
// ---------------------------------------------------------------------------------------------

/// The number of tags kept when a function that started with `frame` tags returns `kept`:
/// the tags created before it, and the ones up to the last returned tag it created.
pub open spec fn kept_count(count: nat, frame: nat, kept: Seq<u8>) -> nat {
    let step = |c: nat, k: u8| if frame <= k && (k as nat) < count && c <= k { k as nat + 1 } else { c };
    let c = step(step(step(step(frame, kept[0]), kept[1]), kept[2]), kept[3]);
    if c < count { c } else { count }
}

/// A tag created by the returning function stops being protected (and so conflicted).
pub open spec fn unprotect(n: Node) -> Node {
    Node {
        protected: false,
        strong: false,
        perm: match n.perm {
            Perm::Reserved { conflicted, interior } => Perm::Reserved { conflicted: false, interior },
            p => p,
        },
        ..n
    }
}

/// Unless one of them was exposed, the tags the returning function created and does not return
/// are dropped.
pub open spec fn return_spec(a: AbsState, b: AbsState, frame: nat, kept: Seq<u8>) -> bool {
    let c = kept_count(a.count, frame, kept);
    let keep_all = exists|t: nat| c <= t < a.count
        && ((#[trigger] a.nodes[t]).exposed || a.nodes[t].stored);
    &&& b.count == if keep_all { a.count } else { c }
    &&& b.unknown_exposed == a.unknown_exposed
    &&& forall|t: nat| t < b.count ==> node_eq(#[trigger] b.nodes[t],
            if frame < 64 && t >= frame { unprotect(a.nodes[t]) } else { a.nodes[t] })
}

#[verifier::rlimit(100)]
proof fn lemma_return(s0: MonitorState, s1: MonitorState, s: MonitorState, frame: u8, kept: [u8; 4],
    count: u8)
    requires
        monitor_wf(&s0),
        frame >= 1,
        // Protection ends.
        s1 == if frame < 64 {
            MonitorState {
                protected: s0.protected & (((1u64 << frame) - 1u64) as u64),
                strong: s0.strong & (((1u64 << frame) - 1u64) as u64),
                conflicted: s0.conflicted & (((1u64 << frame) - 1u64) as u64),
                ..s0
            }
        } else {
            s0
        },
        count as nat == {
            let step = |c: nat, k: u8| if frame <= k && k < s0.count && c <= k { k as nat + 1 } else { c };
            step(step(step(step(frame as nat, kept[0]), kept[1]), kept[2]), kept[3])
        },
        // Unreachable tags are dropped, unless one was exposed.
        s == if count < s1.count && (s1.exposed | s1.stored) & !(((1u64 << count) - 1u64) as u64) == 0 {
            let below = ((1u64 << count) - 1u64) as u64;
            MonitorState {
                count: count,
                main: s1.main & below,
                valid: s1.valid & below,
                active: s1.active & below,
                frozen: s1.frozen & below,
                conflicted: s1.conflicted & below,
                interior: s1.interior & below,
                protected: s1.protected & below,
                strong: s1.strong & below,
                initialized: s1.initialized & below,
                cell: s1.cell & below,
                exposed: s1.exposed & below,
                stored: s1.stored & below,
                ..s1
            }
        } else {
            s1
        },
    ensures
        monitor_wf(&s),
        return_spec(abstraction(&s0), abstraction(&s), frame as nat, kept@),
{
    if !s.wildcard && s.count < s1.count {
        lemma_live_shrink(s0.count as u64, count as u64);
        assert(s.main == live_mask(s.count as u64));
    }
    // Bits below the frame.
    assert forall|i: u64| i < 64 implies {
        &&& bit(s1.protected, i) == (bit(s0.protected, i) && (frame >= 64 || i < frame))
        &&& bit(s1.strong, i) == (bit(s0.strong, i) && (frame >= 64 || i < frame))
        &&& bit(s1.conflicted, i) == (bit(s0.conflicted, i) && (frame >= 64 || i < frame))
    } by {
        if frame < 64 {
            lemma_bit_below(frame as u64, i);
            lemma_bit_and(s0.protected, ((1u64 << frame) - 1u64) as u64, i);
            lemma_bit_and(s0.strong, ((1u64 << frame) - 1u64) as u64, i);
            lemma_bit_and(s0.conflicted, ((1u64 << frame) - 1u64) as u64, i);
        }
    }
    let a = abstraction(&s0);
    let c = kept_count(a.count, frame as nat, kept@);
    let drop = count < s1.count && (s1.exposed | s1.stored) & !(((1u64 << count) - 1u64) as u64) == 0;
    // Whether a dropped tag was exposed.
    if count < s1.count {
        let below = ((1u64 << count) - 1u64) as u64;
        assert(count >= 1);
        let es = s1.exposed | s1.stored;
        lemma_and_nonzero(es, !below);
        assert((es & !below != 0) == exists|t: nat| c <= t < a.count
            && ((#[trigger] a.nodes[t]).exposed || a.nodes[t].stored)) by {
            assert(c == count as nat);
            if es & !below != 0 {
                let t = choose|t: u64| t < 64 && bit(es, t) && bit(!below, t);
                lemma_bit_or(s1.exposed, s1.stored, t);
                lemma_bit_not(below, t);
                lemma_bit_below(count as u64, t);
                assert(t < s0.count) by {
                    if t >= s0.count { assert(!bit(s0.exposed, t) && !bit(s0.stored, t)); }
                }
                assert(a.nodes[t as nat].exposed || a.nodes[t as nat].stored);
            } else if exists|t: nat| c <= t < a.count
                && ((#[trigger] a.nodes[t]).exposed || a.nodes[t].stored) {
                let t = choose|t: nat| c <= t < a.count
                    && ((#[trigger] a.nodes[t]).exposed || a.nodes[t].stored);
                lemma_bit_or(s1.exposed, s1.stored, t as u64);
                lemma_bit_not(below, t as u64);
                lemma_bit_below(count as u64, t as u64);
            }
        }
    } else {
        assert(c == a.count);
    }
    let n = s.count;
    assert forall|i: u64| i < 64 implies {
        &&& bit(s.main, i) == (bit(s1.main, i) && i < n)
        &&& bit(s.valid, i) == (bit(s1.valid, i) && i < n)
        &&& bit(s.active, i) == (bit(s1.active, i) && i < n)
        &&& bit(s.frozen, i) == (bit(s1.frozen, i) && i < n)
        &&& bit(s.conflicted, i) == (bit(s1.conflicted, i) && i < n)
        &&& bit(s.interior, i) == (bit(s1.interior, i) && i < n)
        &&& bit(s.protected, i) == (bit(s1.protected, i) && i < n)
        &&& bit(s.strong, i) == (bit(s1.strong, i) && i < n)
        &&& bit(s.initialized, i) == (bit(s1.initialized, i) && i < n)
        &&& bit(s.cell, i) == (bit(s1.cell, i) && i < n)
        &&& bit(s.exposed, i) == (bit(s1.exposed, i) && i < n)
        &&& bit(s.stored, i) == (bit(s1.stored, i) && i < n)
    } by {
        if drop {
            let below = ((1u64 << count) - 1u64) as u64;
            lemma_bit_below(count as u64, i);
            lemma_bit_and(s1.main, below, i);
            lemma_bit_and(s1.valid, below, i);
            lemma_bit_and(s1.active, below, i);
            lemma_bit_and(s1.frozen, below, i);
            lemma_bit_and(s1.conflicted, below, i);
            lemma_bit_and(s1.interior, below, i);
            lemma_bit_and(s1.protected, below, i);
            lemma_bit_and(s1.strong, below, i);
            lemma_bit_and(s1.initialized, below, i);
            lemma_bit_and(s1.cell, below, i);
            lemma_bit_and(s1.exposed, below, i);
            lemma_bit_and(s1.stored, below, i);
        } else {
            if i >= n {
                assert(!bit(s0.main, i) && !bit(s0.valid, i) && !bit(s0.active, i)
                    && !bit(s0.frozen, i) && !bit(s0.conflicted, i) && !bit(s0.interior, i)
                    && !bit(s0.protected, i) && !bit(s0.strong, i) && !bit(s0.initialized, i)
                    && !bit(s0.cell, i) && !bit(s0.exposed, i) && !bit(s0.stored, i));
            }
        }
    }
    // Well-formedness: the remaining tags keep their ancestors and aliases.
    assert forall|t: u64| #![trigger s.ancestors[t as int]] t < s.count implies {
        &&& bit(s.ancestors[t as int], t)
        &&& forall|u: u64| u < 64 && bit(s.ancestors[t as int], u) ==> u <= t
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[t as int], u)
            && bit(s.ancestors[u as int], w) ==> bit(s.ancestors[t as int], w)
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(s.ancestors[t as int], u)
            && bit(s.ancestors[t as int], w) && u <= w ==> bit(s.ancestors[w as int], u)
        &&& bit(s.main, t) == bit(s.ancestors[t as int], 0)
    } by {
        assert(s0.ancestors[t as int] == s0.ancestors[t as int]);
    }
    assert forall|t: u64| #![trigger s.alias[t as int]] t < s.count implies if bit(s.cell, t) {
        &&& s.alias[t as int] < t
        &&& !bit(s.cell, s.alias[t as int] as u64)
        &&& bit(s.ancestors[t as int], s.alias[t as int] as u64)
    } else {
        s.alias[t as int] == t
    } by {
        assert(s0.alias[t as int] == s0.alias[t as int]);
    }
    let b = abstraction(&s);
    assert forall|t: nat| t < b.count implies node_eq(#[trigger] b.nodes[t],
        if frame < 64 && t >= frame { unprotect(a.nodes[t]) } else { a.nodes[t] }) by {
        assert(mask_set(s.ancestors[t as int]) =~= mask_set(s0.ancestors[t as int]));
    }
}

/// A skipped step of `monitor_holders`: a free slot adds nothing.
proof fn lemma_holders_skip(s: MonitorState, b: u64, v: nat, h: u64)
    requires v < 64, v >= s.count, h == holders_upto(&s, b, v),
    ensures h == holders_upto(&s, b, v + 1),
{
    lemma_or_zero(h);
}

/// A skipped step of `monitor_candidate_paths`: a free slot adds nothing.
proof fn lemma_paths_skip(s: MonitorState, cands: u64, c: nat, all: u64, any: u64)
    requires c < 64, c >= s.count, all == all_upto(&s, cands, c), any == any_upto(&s, cands, c),
    ensures all == all_upto(&s, cands, c + 1), any == any_upto(&s, cands, c + 1),
{
    lemma_and_max(all);
    lemma_or_zero(any);
}


// ---------------------------------------------------------------------------------------------
// Pointers stored in memory
// ---------------------------------------------------------------------------------------------

/// Storing a pointer tagged `tag` in memory marks the tag stored; nothing else changes.
pub open spec fn store_spec(a: AbsState, b: AbsState, tag: nat) -> bool {
    &&& b.count == a.count
    &&& b.unknown_exposed == a.unknown_exposed
    &&& if tag < a.count {
        &&& same_nodes_except(a, b, iset![tag])
        &&& node_eq(b.nodes[tag], Node { stored: true, ..a.nodes[tag] })
    } else {
        same_nodes_except(a, b, ISet::empty())
    }
}

proof fn lemma_store(s0: MonitorState, s: MonitorState, tag: u8)
    requires
        monitor_wf(&s0), tag < s0.count,
        s == (MonitorState { stored: s0.stored | (1u64 << tag), ..s0 }),
    ensures monitor_wf(&s), store_spec(abstraction(&s0), abstraction(&s), tag as nat),
{
    assert forall|i: u64| i < 64 implies bit(s.stored, i) == (i == tag || bit(s0.stored, i)) by {
        lemma_bit_set(s0.stored, tag as u64, i);
    }
    let a = abstraction(&s0);
    let b = abstraction(&s);
    assert forall|t: nat| t < a.count && !iset![tag as nat].contains(t) implies
        node_eq(#[trigger] b.nodes[t], a.nodes[t]) by {}
}
fn main() {}
// --- Loop and function contracts: permissions only move down ---------------------------------
//
// At the head of a loop with a contract, the checks let the state be any one whose tags, those
// alive where the loop started, are at or below their state there (`aliasing_loop_head` in
// `../src/aliasing.rs`). That is sound because no transition moves a node up: the lemmas below.
// Tags that only see local accesses in the loop keep their permission except Reserved to Active,
// which the second lemma gives.

/// `new` is reachable from `old` by Tree Borrows transitions: Reserved, Active, Frozen,
/// Disabled, in this order, with a Reserved that may become conflicted but keeps its interior
/// mutability.
pub open spec fn perm_le(new: Perm, old: Perm) -> bool {
    match old {
        Perm::Disabled => new == Perm::Disabled,
        Perm::Frozen => new == Perm::Frozen || new == Perm::Disabled,
        Perm::Active => new == Perm::Active || new == Perm::Frozen || new == Perm::Disabled,
        Perm::Reserved { conflicted, interior } => match new {
            Perm::Reserved { conflicted: c2, interior: i2 } => i2 == interior && (conflicted ==> c2),
            _ => true,
        },
    }
}

/// `new` is `old` after transitions: its permission is at or below, it is initialized if `old`
/// is, and nothing else changed.
pub open spec fn node_le(new: Node, old: Node) -> bool {
    &&& perm_le(new.perm, old.perm)
    &&& (old.initialized ==> new.initialized)
    &&& new.ancestors == old.ancestors
    &&& new.main == old.main
    &&& new.protected == old.protected
    &&& new.strong == old.strong
    &&& new.cell == old.cell
    &&& new.alias == old.alias
    &&& new.exposed == old.exposed
    &&& new.stored == old.stored
}

pub proof fn lemma_perm_le_refl(p: Perm)
    ensures perm_le(p, p),
{
}

pub proof fn lemma_perm_le_trans(a: Perm, b: Perm, c: Perm)
    requires perm_le(a, b), perm_le(b, c),
    ensures perm_le(a, c),
{
}

pub proof fn lemma_node_le_trans(a: Node, b: Node, c: Node)
    requires node_le(a, b), node_le(b, c),
    ensures node_le(a, c),
{
    lemma_perm_le_trans(a.perm, b.perm, c.perm);
}

/// Every transition, local or foreign, moves a node down.
pub proof fn lemma_steps_monotone(n: Node, is_write: bool)
    ensures
        local_step(n, is_write) matches Some(m) ==> node_le(m, n),
        foreign_step(n, is_write) matches Some(m) ==> node_le(m, n),
{
}

/// A local access never disables, freezes or conflicts a node: it keeps its permission, or a
/// Reserved becomes Active.
pub proof fn lemma_local_step_keeps(n: Node, is_write: bool)
    ensures
        local_step(n, is_write) matches Some(m) ==> (m.perm == n.perm || (
            n.perm matches Perm::Reserved { .. } && m.perm == Perm::Active)),
{
}

// --- The loop head: the havoc is sound and complete ------------------------------------------
//
// `monitor_loop_head` replaces the borrow state at the head of a loop with a contract. The
// argument that this is sound has three parts:
// 1. Every monitor operation keeps `loop_rel(entry, _, roots)` (`lemma_loop_*` below): the tags
//    alive at loop entry keep their ancestors and protector, their permissions only move down,
//    and the roots only see local accesses. So the state at the head after any number of
//    iterations is in that relation with the state at entry.
// 2. Dropping the tags created by earlier iterations (`project`) keeps the state well formed
//    and the other tags unchanged (`lemma_project`).
// 3. For every well-formed state in the relation, some choice of the nondeterministic values
//    makes `monitor_loop_head` produce it, up to the abstraction, and pass its assumption
//    (`lemma_loop_head_complete`). When the assumption holds, the state is well formed
//    (`lemma_loop_head_wf`), so the other theorems apply to it.

pub open spec fn loop_head_spec(e: MonitorState, common: u64, v: LoopHavoc) -> MonitorState {
    let live = live_mask(e.count as u64);
    let local = common & live;
    let any = !local;
    MonitorState {
        valid: e.valid & (v.valid | local),
        active: (e.active | v.active) & live,
        frozen: (e.frozen | (v.frozen & any)) & live,
        conflicted: (e.conflicted | (v.conflicted & e.protected & any)) & live,
        initialized: (e.initialized | v.initialized) & live,
        exposed: (e.exposed | v.exposed) & live,
        stored: (e.stored | v.stored) & live,
        unknown_exposed: e.unknown_exposed || v.unknown_exposed,
        exposed_values: e.exposed_values || v.exposed_values,
        ..e
    }
}

pub open spec fn loop_head_ok(h: &MonitorState, e: &MonitorState) -> bool {
    &&& h.frozen & !e.frozen & !h.active == 0
    &&& h.frozen & h.interior & !h.active == 0
    &&& h.exposed & h.cell == 0
    &&& h.conflicted & (h.active | h.frozen) == 0
    &&& h.active & !h.initialized == 0
}

proof fn lemma_bits_and_not_not(x: u64, y: u64, z: u64, i: u64)
    requires i < 64,
    ensures bit(x & !y & !z, i) == (bit(x, i) && !bit(y, i) && !bit(z, i)),
{
    assert((((x & !y & !z) >> i) & 1u64 == 1u64) == (((x >> i) & 1u64 == 1u64)
        && !((y >> i) & 1u64 == 1u64) && !((z >> i) & 1u64 == 1u64))) by (bit_vector)
        requires i < 64;
}

proof fn lemma_bits_and_and_not(x: u64, y: u64, z: u64, i: u64)
    requires i < 64,
    ensures bit(x & y & !z, i) == (bit(x, i) && bit(y, i) && !bit(z, i)),
{
    assert((((x & y & !z) >> i) & 1u64 == 1u64) == (((x >> i) & 1u64 == 1u64)
        && ((y >> i) & 1u64 == 1u64) && !((z >> i) & 1u64 == 1u64))) by (bit_vector)
        requires i < 64;
}

proof fn lemma_bits_and_or(x: u64, y: u64, z: u64, i: u64)
    requires i < 64,
    ensures bit(x & (y | z), i) == (bit(x, i) && (bit(y, i) || bit(z, i))),
{
    assert((((x & (y | z)) >> i) & 1u64 == 1u64) == (((x >> i) & 1u64 == 1u64)
        && (((y >> i) & 1u64 == 1u64) || ((z >> i) & 1u64 == 1u64)))) by (bit_vector)
        requires i < 64;
}

proof fn lemma_bits_and_not(x: u64, y: u64, i: u64)
    requires i < 64,
    ensures bit(x & !y, i) == (bit(x, i) && !bit(y, i)),
{
    assert((((x & !y) >> i) & 1u64 == 1u64) == (((x >> i) & 1u64 == 1u64)
        && !((y >> i) & 1u64 == 1u64))) by (bit_vector) requires i < 64;
}

/// A mask is zero if and only if none of its bits is set.
proof fn lemma_zero_iff(m: u64)
    ensures (m == 0) == forall|i: u64| i < 64 ==> !bit(m, i),
{
    if m == 0 {
        assert forall|i: u64| i < 64 implies !bit(m, i) by { lemma_bit_zero(i); }
    }
    if forall|i: u64| i < 64 ==> !bit(m, i) {
        lemma_all_bits_clear(m);
    }
}

/// The bits of `loop_head_spec`.
proof fn lemma_loop_head_bits(e: MonitorState, common: u64, v: LoopHavoc, i: u64)
    requires 1 <= e.count <= 64, i < 64,
    ensures ({
        let h = loop_head_spec(e, common, v);
        let live = i < e.count;
        let local = bit(common, i) && live;
        &&& bit(h.valid, i) == (bit(e.valid, i) && (bit(v.valid, i) || local))
        &&& bit(h.active, i) == ((bit(e.active, i) || bit(v.active, i)) && live)
        &&& bit(h.frozen, i) == ((bit(e.frozen, i) || (bit(v.frozen, i) && !local)) && live)
        &&& bit(h.conflicted, i) == ((bit(e.conflicted, i) || (bit(v.conflicted, i)
            && bit(e.protected, i) && !local)) && live)
        &&& bit(h.initialized, i) == ((bit(e.initialized, i) || bit(v.initialized, i)) && live)
        &&& bit(h.exposed, i) == ((bit(e.exposed, i) || bit(v.exposed, i)) && live)
        &&& bit(h.stored, i) == ((bit(e.stored, i) || bit(v.stored, i)) && live)
    }),
{
    let lm = live_mask(e.count as u64);
    lemma_bit_live(e.count as u64, i);
    let local = common & lm;
    lemma_bit_and(common, lm, i);
    lemma_bit_not(local, i);
    lemma_bit_or(v.valid, local, i);
    lemma_bit_and(e.valid, v.valid | local, i);
    lemma_bit_or(e.active, v.active, i);
    lemma_bit_and(e.active | v.active, lm, i);
    lemma_bit_and(v.frozen, !local, i);
    lemma_bit_or(e.frozen, v.frozen & !local, i);
    lemma_bit_and(e.frozen | (v.frozen & !local), lm, i);
    lemma_bit_and(v.conflicted, e.protected, i);
    lemma_bit_and(v.conflicted & e.protected, !local, i);
    lemma_bit_or(e.conflicted, v.conflicted & e.protected & !local, i);
    lemma_bit_and(e.conflicted | (v.conflicted & e.protected & !local), lm, i);
    lemma_bit_or(e.initialized, v.initialized, i);
    lemma_bit_and(e.initialized | v.initialized, lm, i);
    lemma_bit_or(e.exposed, v.exposed, i);
    lemma_bit_and(e.exposed | v.exposed, lm, i);
    lemma_bit_or(e.stored, v.stored, i);
    lemma_bit_and(e.stored | v.stored, lm, i);
}

/// The per-bit conditions of `loop_head_ok`.
pub open spec fn loop_head_ok_at(h: MonitorState, e: MonitorState, i: u64) -> bool {
    &&& (bit(h.frozen, i) && !bit(e.frozen, i) ==> bit(h.active, i))
    &&& (bit(h.frozen, i) && bit(h.interior, i) ==> bit(h.active, i))
    &&& !(bit(h.exposed, i) && bit(h.cell, i))
    &&& (bit(h.conflicted, i) ==> !bit(h.active, i) && !bit(h.frozen, i))
    &&& (bit(h.active, i) ==> bit(h.initialized, i))
}

/// The per-bit meaning of `loop_head_ok`.
proof fn lemma_loop_head_ok_bits(h: MonitorState, e: MonitorState)
    ensures loop_head_ok(&h, &e) == forall|i: u64| i < 64 ==> #[trigger] loop_head_ok_at(h, e, i),
{
    let w1 = h.frozen & !e.frozen & !h.active;
    let w2 = h.frozen & h.interior & !h.active;
    let w3 = h.exposed & h.cell;
    let w4 = h.conflicted & (h.active | h.frozen);
    let w5 = h.active & !h.initialized;
    assert forall|i: u64| i < 64 implies loop_head_ok_at(h, e, i) == (!bit(w1, i) && !bit(w2, i)
        && !bit(w3, i) && !bit(w4, i) && !bit(w5, i)) by {
        lemma_bits_and_not_not(h.frozen, e.frozen, h.active, i);
        lemma_bits_and_and_not(h.frozen, h.interior, h.active, i);
        lemma_bit_and(h.exposed, h.cell, i);
        lemma_bits_and_or(h.conflicted, h.active, h.frozen, i);
        lemma_bits_and_not(h.active, h.initialized, i);
    }
    lemma_zero_iff(w1);
    lemma_zero_iff(w2);
    lemma_zero_iff(w3);
    lemma_zero_iff(w4);
    lemma_zero_iff(w5);
    if forall|i: u64| i < 64 ==> #[trigger] loop_head_ok_at(h, e, i) {
        assert forall|i: u64| i < 64 implies !bit(w1, i) && !bit(w2, i) && !bit(w3, i)
            && !bit(w4, i) && !bit(w5, i) by {
            assert(loop_head_ok_at(h, e, i));
        }
    }
    if loop_head_ok(&h, &e) {
        assert forall|i: u64| i < 64 implies #[trigger] loop_head_ok_at(h, e, i) by {
            assert(!bit(w1, i) && !bit(w2, i) && !bit(w3, i) && !bit(w4, i) && !bit(w5, i));
        }
    }
}

/// Soundness of the assumption: a state `monitor_loop_head` accepts is well formed.
pub proof fn lemma_loop_head_wf(e: MonitorState, h: MonitorState, common: u64, v: LoopHavoc)
    requires monitor_wf(&e), h == loop_head_spec(e, common, v), loop_head_ok(&h, &e),
    ensures monitor_wf(&h),
{
    lemma_loop_head_ok_bits(h, e);
    assert forall|i: u64| i < 64 implies {
        &&& (bit(h.frozen, i) && bit(h.interior, i) ==> bit(h.active, i))
        &&& (bit(h.exposed, i) ==> !bit(h.cell, i))
        &&& (bit(h.conflicted, i) ==> !bit(h.active, i) && !bit(h.frozen, i)
            && bit(h.protected, i))
        &&& (bit(h.active, i) ==> bit(h.initialized, i))
        &&& (bit(h.interior, i) ==> !bit(h.protected, i))
        &&& (bit(h.strong, i) ==> bit(h.protected, i))
    } by {
        assert(loop_head_ok_at(h, e, i));
        lemma_loop_head_bits(e, common, v, i);
    }
    assert forall|i: u64| h.count <= i < 64 implies {
        &&& !bit(h.valid, i) && !bit(h.active, i) && !bit(h.frozen, i)
        &&& !bit(h.conflicted, i) && !bit(h.initialized, i) && !bit(h.exposed, i)
        &&& !bit(h.stored, i)
    } by {
        lemma_loop_head_bits(e, common, v, i);
    }
}

/// The nondeterministic choices that make `monitor_loop_head` produce `s`.
pub open spec fn loop_head_witness(e: MonitorState, s: MonitorState) -> LoopHavoc {
    LoopHavoc {
        valid: s.valid,
        active: (s.active | (s.frozen & !e.frozen)) & s.valid,
        frozen: s.frozen & s.valid,
        conflicted: s.conflicted & s.valid,
        initialized: s.initialized,
        exposed: s.exposed,
        stored: s.stored,
        unknown_exposed: s.unknown_exposed,
        exposed_values: s.exposed_values,
    }
}

/// Facts about a node that `monitor_wf` gives.
pub open spec fn node_wf(n: Node) -> bool {
    &&& n.perm == Perm::Active ==> n.initialized
    &&& (n.perm matches Perm::Reserved { conflicted: true, .. }) ==> n.protected
}

/// `new` is `old` after any transitions during a loop: the permission moved down, and only the
/// initialized, exposed and stored flags may have been set. A conflicted Reserved can only be
/// disabled (it is protected, so a local write is undefined behavior), and a tag that became
/// Frozen was Active, so it is initialized.
pub open spec fn loop_le(new: Node, old: Node) -> bool {
    &&& perm_le(new.perm, old.perm)
    &&& (old.perm matches Perm::Reserved { conflicted: true, .. }) ==> (new.perm == old.perm
        || new.perm == Perm::Disabled)
    &&& (new.perm == Perm::Frozen && old.perm != Perm::Frozen) ==> new.initialized
    &&& (old.initialized ==> new.initialized)
    &&& (old.exposed ==> new.exposed)
    &&& (old.stored ==> new.stored)
    &&& new.ancestors == old.ancestors
    &&& new.main == old.main
    &&& new.protected == old.protected
    &&& new.strong == old.strong
    &&& new.cell == old.cell
    &&& new.alias == old.alias
}

/// The permission of a tag that only sees local accesses: unchanged, or a Reserved that was
/// written.
pub open spec fn root_keep(new: Perm, old: Perm) -> bool {
    new == old || ((old matches Perm::Reserved { conflicted: false, .. }) && new == Perm::Active)
}

/// The state `a` at the head of a loop that started in state `e`, whose roots are `roots`.
pub open spec fn loop_rel(e: AbsState, a: AbsState, roots: ISet<nat>) -> bool {
    &&& e.count <= a.count
    &&& (e.unknown_exposed ==> a.unknown_exposed)
    &&& forall|t: nat| t < e.count ==> loop_le(#[trigger] a.nodes[t], e.nodes[t])
    &&& forall|t: nat| t < e.count && roots.contains(t) ==> root_keep((#[trigger] a.nodes[t]).perm,
        e.nodes[t].perm)
}

/// Completeness: every well-formed state in the relation with the entry state is, up to the
/// abstraction, a state `monitor_loop_head` produces and accepts.
pub proof fn lemma_loop_head_complete(e: MonitorState, s: MonitorState, common: u64)
    requires
        monitor_wf(&e), monitor_wf(&s), s.count == e.count,
        s.tracked_object_id == e.tracked_object_id, s.tracked_offset == e.tracked_offset,
        e.exposed_values ==> s.exposed_values,
        loop_rel(abstraction(&e), abstraction(&s), mask_set(common & live_mask(e.count as u64))),
    ensures ({
        let h = loop_head_spec(e, common, loop_head_witness(e, s));
        &&& loop_head_ok(&h, &e)
        &&& monitor_wf(&h)
        &&& same_state(abstraction(&h), abstraction(&s))
        &&& h.exposed_values == s.exposed_values
        &&& h.tracked_object_id == s.tracked_object_id
        &&& h.tracked_offset == s.tracked_offset
    }),
{
    let v = loop_head_witness(e, s);
    let h = loop_head_spec(e, common, v);
    let lm = live_mask(e.count as u64);
    let ae = abstraction(&e);
    let as_ = abstraction(&s);
    assert forall|i: u64| i < 64 implies {
        &&& (i < e.count ==> bit(common & lm, i) == bit(common, i))
        &&& bit(v.active, i) == ((bit(s.active, i) || (bit(s.frozen, i) && !bit(e.frozen, i)))
            && bit(s.valid, i))
        &&& bit(v.frozen, i) == (bit(s.frozen, i) && bit(s.valid, i))
        &&& bit(v.conflicted, i) == (bit(s.conflicted, i) && bit(s.valid, i))
    } by {
        lemma_bit_live(e.count as u64, i);
        lemma_bit_and(common, lm, i);
        lemma_bit_and(s.frozen, !e.frozen, i);
        lemma_bit_not(e.frozen, i);
        lemma_bit_or(s.active, s.frozen & !e.frozen, i);
        lemma_bit_and(s.active | (s.frozen & !e.frozen), s.valid, i);
        lemma_bit_and(s.frozen, s.valid, i);
        lemma_bit_and(s.conflicted, s.valid, i);
    }
    // Per tag: the bits of `h` against those of `s`.
    assert forall|i: u64| i < 64 implies {
        &&& #[trigger] loop_head_ok_at(h, e, i)
        &&& (i < e.count ==> perm_of(&h, i) == perm_of(&s, i))
        &&& bit(h.initialized, i) == bit(s.initialized, i)
        &&& bit(h.exposed, i) == bit(s.exposed, i)
        &&& bit(h.stored, i) == bit(s.stored, i)
    } by {
        lemma_loop_head_bits(e, common, v, i);
        if i < e.count {
            let t = i as nat;
            assert(loop_le(as_.nodes[t], ae.nodes[t]));
            if bit(common, i) {
                assert(mask_set(common & lm).contains(t));
                assert(root_keep(as_.nodes[t].perm, ae.nodes[t].perm));
            }
        }
    }
    lemma_loop_head_ok_bits(h, e);
    lemma_loop_head_wf(e, h, common, v);
    let ah = abstraction(&h);
    assert forall|t: nat| t < ah.count implies node_eq(#[trigger] as_.nodes[t], ah.nodes[t]) by {
        assert(loop_head_ok_at(h, e, t as u64));
        assert(loop_le(as_.nodes[t], ae.nodes[t]));
        assert(mask_set(h.ancestors[t as int]) =~= mask_set(s.ancestors[t as int]));
    }
}

/// The state without the tags `n..`: those created by earlier iterations of a loop.
pub open spec fn project(s: MonitorState, n: u8) -> MonitorState {
    let live = live_mask(n as u64);
    MonitorState {
        count: n,
        main: s.main & live,
        valid: s.valid & live,
        active: s.active & live,
        frozen: s.frozen & live,
        conflicted: s.conflicted & live,
        interior: s.interior & live,
        protected: s.protected & live,
        strong: s.strong & live,
        initialized: s.initialized & live,
        cell: s.cell & live,
        exposed: s.exposed & live,
        stored: s.stored & live,
        ..s
    }
}

pub proof fn lemma_project(s: MonitorState, n: u8)
    requires monitor_wf(&s), 1 <= n <= s.count,
    ensures ({
        let p = project(s, n);
        &&& monitor_wf(&p)
        &&& p.unknown_exposed == s.unknown_exposed
        &&& p.exposed_values == s.exposed_values
        &&& p.tracked_object_id == s.tracked_object_id
        &&& p.tracked_offset == s.tracked_offset
        &&& forall|t: nat| t < n ==> node_eq(#[trigger] abstraction(&p).nodes[t],
            abstraction(&s).nodes[t])
    }),
{
    let p = project(s, n);
    let lm = live_mask(n as u64);
    assert forall|i: u64| i < 64 implies {
        &&& bit(p.main, i) == (bit(s.main, i) && i < n)
        &&& bit(p.valid, i) == (bit(s.valid, i) && i < n)
        &&& bit(p.active, i) == (bit(s.active, i) && i < n)
        &&& bit(p.frozen, i) == (bit(s.frozen, i) && i < n)
        &&& bit(p.conflicted, i) == (bit(s.conflicted, i) && i < n)
        &&& bit(p.interior, i) == (bit(s.interior, i) && i < n)
        &&& bit(p.protected, i) == (bit(s.protected, i) && i < n)
        &&& bit(p.strong, i) == (bit(s.strong, i) && i < n)
        &&& bit(p.initialized, i) == (bit(s.initialized, i) && i < n)
        &&& bit(p.cell, i) == (bit(s.cell, i) && i < n)
        &&& bit(p.exposed, i) == (bit(s.exposed, i) && i < n)
        &&& bit(p.stored, i) == (bit(s.stored, i) && i < n)
        &&& bit(live_mask(s.count as u64), i) == (i < s.count)
        &&& bit(lm, i) == (i < n)
    } by {
        lemma_bit_live(n as u64, i);
        lemma_bit_live(s.count as u64, i);
        lemma_bit_and(s.main, lm, i);
        lemma_bit_and(s.valid, lm, i);
        lemma_bit_and(s.active, lm, i);
        lemma_bit_and(s.frozen, lm, i);
        lemma_bit_and(s.conflicted, lm, i);
        lemma_bit_and(s.interior, lm, i);
        lemma_bit_and(s.protected, lm, i);
        lemma_bit_and(s.strong, lm, i);
        lemma_bit_and(s.initialized, lm, i);
        lemma_bit_and(s.cell, lm, i);
        lemma_bit_and(s.exposed, lm, i);
        lemma_bit_and(s.stored, lm, i);
    }
    if !s.wildcard {
        assert forall|i: u64| i < 64 implies bit(p.main, i) == bit(lm, i) by {}
        lemma_bits_ext(p.main, lm);
    }
    assert forall|t: u64| #![trigger p.ancestors[t as int]] t < p.count implies
        bit(p.main, t) == bit(p.ancestors[t as int], 0) by {
        assert(s.ancestors[t as int] == s.ancestors[t as int]);
        lemma_bit_live(n as u64, t);
        lemma_bit_and(s.main, lm, t);
    }
    assert forall|t: u64| #![trigger p.alias[t as int]] t < p.count implies (if bit(p.cell, t) {
        &&& p.alias[t as int] < t
        &&& !bit(p.cell, p.alias[t as int] as u64)
        &&& bit(p.ancestors[t as int], p.alias[t as int] as u64)
    } else {
        p.alias[t as int] == t
    }) by {
        assert(s.alias[t as int] == s.alias[t as int]);
        lemma_bit_live(n as u64, t);
        lemma_bit_and(s.cell, lm, t);
        if bit(s.cell, t) {
            let u = s.alias[t as int] as u64;
            lemma_bit_live(n as u64, u);
            lemma_bit_and(s.cell, lm, u);
        }
    }
    assert forall|t: u64| #![trigger p.ancestors[t as int]] t < p.count implies {
        &&& bit(p.ancestors[t as int], t)
        &&& forall|u: u64| u < 64 && bit(p.ancestors[t as int], u) ==> u <= t
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(p.ancestors[t as int], u)
            && bit(p.ancestors[u as int], w) ==> bit(p.ancestors[t as int], w)
        &&& forall|u: u64, w: u64| u < 64 && w < 64 && bit(p.ancestors[t as int], u)
            && bit(p.ancestors[t as int], w) && u <= w ==> bit(p.ancestors[w as int], u)
    } by {
        assert(s.ancestors[t as int] == s.ancestors[t as int]);
    }
    assert forall|i: u64| i < 64 implies {
        &&& (p.count <= i ==> !bit(p.main, i) && !bit(p.valid, i) && !bit(p.stored, i))
        &&& (bit(p.frozen, i) && bit(p.interior, i) ==> bit(p.active, i))
        &&& (bit(p.exposed, i) ==> !bit(p.cell, i))
        &&& (bit(p.conflicted, i) ==> !bit(p.active, i) && !bit(p.frozen, i)
            && bit(p.protected, i))
        &&& (bit(p.active, i) ==> bit(p.initialized, i))
        &&& (bit(p.interior, i) ==> !bit(p.protected, i))
        &&& (bit(p.strong, i) ==> bit(p.protected, i))
    } by {
        lemma_bit_live(n as u64, i);
        lemma_bit_and(s.main, lm, i);
        lemma_bit_and(s.valid, lm, i);
        lemma_bit_and(s.active, lm, i);
        lemma_bit_and(s.frozen, lm, i);
        lemma_bit_and(s.conflicted, lm, i);
        lemma_bit_and(s.interior, lm, i);
        lemma_bit_and(s.protected, lm, i);
        lemma_bit_and(s.strong, lm, i);
        lemma_bit_and(s.initialized, lm, i);
        lemma_bit_and(s.cell, lm, i);
        lemma_bit_and(s.exposed, lm, i);
        lemma_bit_and(s.stored, lm, i);
    }
    assert(clean_above(p.main, p.count as u64));
    assert(clean_above(p.valid, p.count as u64));
    assert(clean_above(p.active, p.count as u64));
    assert(clean_above(p.frozen, p.count as u64));
    assert(clean_above(p.conflicted, p.count as u64));
    assert(clean_above(p.interior, p.count as u64));
    assert(clean_above(p.protected, p.count as u64));
    assert(clean_above(p.strong, p.count as u64));
    assert(clean_above(p.initialized, p.count as u64));
    assert(clean_above(p.cell, p.count as u64));
    assert(clean_above(p.exposed, p.count as u64));
    assert(clean_above(p.stored, p.count as u64));
    assert(!bit(p.cell, 0)) by { lemma_bit_and(s.cell, lm, 0); }
    assert(monitor_wf(&p));
    let ap = abstraction(&p);
    let as_ = abstraction(&s);
    assert forall|t: nat| t < n implies node_eq(#[trigger] ap.nodes[t], as_.nodes[t]) by {
        let i = t as u64;
        lemma_bit_live(n as u64, i);
        lemma_bit_and(s.main, lm, i);
        lemma_bit_and(s.valid, lm, i);
        lemma_bit_and(s.active, lm, i);
        lemma_bit_and(s.frozen, lm, i);
        lemma_bit_and(s.conflicted, lm, i);
        lemma_bit_and(s.interior, lm, i);
        lemma_bit_and(s.protected, lm, i);
        lemma_bit_and(s.strong, lm, i);
        lemma_bit_and(s.initialized, lm, i);
        lemma_bit_and(s.cell, lm, i);
        lemma_bit_and(s.exposed, lm, i);
        lemma_bit_and(s.stored, lm, i);
        assert(mask_set(p.ancestors[t as int]) =~= mask_set(s.ancestors[t as int]));
    }
}

/// The theorem for the loop head: given the state `s` that the loop reached at its head, in the
/// relation with the entry state `e` (which the `lemma_loop_*` lemmas give), some choice of the
/// nondeterministic values makes `monitor_loop_head` produce, up to the abstraction, `s`
/// without the tags created by earlier iterations, and accept it.
pub proof fn lemma_loop_head_sound(e: MonitorState, s: MonitorState, common: u64)
    requires
        monitor_wf(&e), monitor_wf(&s), e.count <= s.count,
        s.tracked_object_id == e.tracked_object_id, s.tracked_offset == e.tracked_offset,
        e.exposed_values ==> s.exposed_values,
        loop_rel(abstraction(&e), abstraction(&s), mask_set(common & live_mask(e.count as u64))),
    ensures exists|v: LoopHavoc| {
        let h = #[trigger] loop_head_spec(e, common, v);
        &&& loop_head_ok(&h, &e)
        &&& monitor_wf(&h)
        &&& same_state(abstraction(&h), abstraction(&project(s, e.count)))
        &&& forall|t: nat| t < e.count ==> node_eq(#[trigger] abstraction(&h).nodes[t],
            abstraction(&s).nodes[t])
    },
{
    let p = project(s, e.count);
    lemma_project(s, e.count);
    let roots = mask_set(common & live_mask(e.count as u64));
    let (ae, ap, as_) = (abstraction(&e), abstraction(&p), abstraction(&s));
    assert forall|t: nat| t < ae.count implies loop_le(#[trigger] ap.nodes[t], ae.nodes[t]) by {
        assert(node_eq(ap.nodes[t], as_.nodes[t]));
        assert(loop_le(as_.nodes[t], ae.nodes[t]));
        assert(ap.nodes[t].ancestors =~= as_.nodes[t].ancestors);
    }
    assert forall|t: nat| t < ae.count && roots.contains(t) implies root_keep(
        (#[trigger] ap.nodes[t]).perm, ae.nodes[t].perm) by {
        assert(node_eq(ap.nodes[t], as_.nodes[t]));
        assert(root_keep(as_.nodes[t].perm, ae.nodes[t].perm));
    }
    lemma_loop_head_complete(e, p, common);
    let v = loop_head_witness(e, p);
    let h = loop_head_spec(e, common, v);
    let ah = abstraction(&h);
    assert forall|t: nat| t < e.count implies node_eq(#[trigger] ah.nodes[t], as_.nodes[t]) by {
        assert(node_eq(ap.nodes[t], ah.nodes[t]));
        assert(node_eq(ap.nodes[t], as_.nodes[t]));
        assert(ah.nodes[t].ancestors =~= as_.nodes[t].ancestors);
    }
}

// --- Every monitor operation keeps the loop relation -----------------------------------------

proof fn lemma_monitor_node_wf(s: MonitorState)
    requires monitor_wf(&s),
    ensures forall|t: nat| t < s.count ==> node_wf(#[trigger] abstraction(&s).nodes[t]),
{
    assert forall|t: nat| t < s.count implies node_wf(#[trigger] abstraction(&s).nodes[t]) by {
        assert(t < 64);
    }
}

/// Each transition keeps `loop_le`; a local one also `root_keep`.
proof fn lemma_steps_loop(n: Node, is_write: bool)
    requires node_wf(n),
    ensures
        local_step(n, is_write) matches Some(m) ==> loop_le(m, n) && root_keep(m.perm, n.perm),
        foreign_step(n, is_write) matches Some(m) ==> loop_le(m, n),
{
}

proof fn lemma_loop_le_trans(a: Node, b: Node, c: Node)
    requires loop_le(a, b), loop_le(b, c),
    ensures loop_le(a, c),
{
    lemma_perm_le_trans(a.perm, b.perm, c.perm);
}

proof fn lemma_node_eq_loop_le(a: Node, b: Node, c: Node)
    requires node_eq(a, b), loop_le(b, c),
    ensures loop_le(a, c),
{
    assert(a.ancestors =~= b.ancestors);
}

/// The relation survives a step whose nodes, for the tags of `e`, are each `loop_le` the
/// previous ones, and `root_keep` for the roots.
proof fn lemma_loop_rel_step(e: AbsState, a: AbsState, b: AbsState, roots: ISet<nat>)
    requires
        loop_rel(e, a, roots), e.count <= b.count,
        e.unknown_exposed ==> b.unknown_exposed,
        forall|t: nat| t < e.count ==> loop_le(#[trigger] b.nodes[t], a.nodes[t]),
        forall|t: nat| t < e.count && roots.contains(t) ==> root_keep(
            (#[trigger] b.nodes[t]).perm, a.nodes[t].perm),
    ensures loop_rel(e, b, roots),
{
    assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], e.nodes[t]) by {
        lemma_loop_le_trans(b.nodes[t], a.nodes[t], e.nodes[t]);
    }
}

/// An unchanged state is still in the relation.
proof fn lemma_loop_same(e: AbsState, a: AbsState, b: AbsState, roots: ISet<nat>)
    requires loop_rel(e, a, roots), same_state(a, b),
    ensures loop_rel(e, b, roots),
{
    assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], e.nodes[t]) by {
        lemma_node_eq_loop_le(b.nodes[t], a.nodes[t], e.nodes[t]);
    }
}

/// An access, given its local tags `l` and foreign tags `f`, keeps the relation if every root it
/// treats as foreign is also local (a tag in both takes the local transition).
pub proof fn lemma_loop_apply(e: AbsState, a: AbsState, b: AbsState, l: ISet<nat>,
    f: ISet<nat>, is_write: bool, code: u8, roots: ISet<nat>)
    requires
        loop_rel(e, a, roots),
        forall|t: nat| t < a.count ==> node_wf(#[trigger] a.nodes[t]),
        apply_refines(a, b, l, f, is_write, code),
        forall|t: nat| t < e.count && roots.contains(t) && f.contains(t) ==> l.contains(t),
    ensures loop_rel(e, b, roots),
{
    if code != ACCESS_OK {
        lemma_loop_same(e, a, b, roots);
    } else {
        assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], a.nodes[t])
            && (roots.contains(t) ==> root_keep(b.nodes[t].perm, a.nodes[t].perm)) by {
            let m = apply_step(a, l, f, t, is_write);
            assert(m is Some);
            lemma_steps_loop(a.nodes[t], is_write);
            assert(node_eq(b.nodes[t], m->Some_0));
            assert(loop_le(m->Some_0, a.nodes[t]));
            lemma_node_eq_loop_le(b.nodes[t], m->Some_0, a.nodes[t]);
        }
        lemma_loop_rel_step(e, a, b, roots);
    }
}

/// An access through `tag` keeps the relation, unless the checks give up, if the roots are
/// ancestors of the tag the access acts through, or cells (which accesses do not change).
pub proof fn lemma_loop_access(e: AbsState, a: AbsState, b: AbsState, tag: nat, is_write: bool,
    code: u8, roots: ISet<nat>)
    requires
        loop_rel(e, a, roots),
        forall|t: nat| t < a.count ==> node_wf(#[trigger] a.nodes[t]),
        tag < a.count,
        access_refines(a, b, tag, is_write, code),
        code != ACCESS_UNKNOWN_WILDCARD,
        forall|r: nat| r < e.count && #[trigger] roots.contains(r) ==> a.nodes[r].cell
            || a.nodes[eff(a, tag)].ancestors.contains(r),
    ensures loop_rel(e, b, roots),
{
    let x = eff(a, tag);
    if a.nodes[x].main {
        lemma_loop_apply(e, a, b, local_set(a, x), foreign_set(a, x), is_write, code, roots);
    } else if a.unknown_exposed {
        lemma_loop_same(e, a, b, roots);
    } else {
        let r = lowest(a, x);
        let own = tree_set(a, r);
        let (wc, wl, wf) = wild_access(a, own, Some(r), is_write);
        if wc != ACCESS_OK {
            lemma_loop_same(e, a, b, roots);
        } else {
            let f = ISet::new(|t: nat| own.contains(t) && !a.nodes[x].ancestors.contains(t)
                && !a.nodes[t].cell).union(wf);
            lemma_loop_apply(e, a, b, local_set(a, x).union(wl), f, is_write, code, roots);
        }
    }
}

/// An access through a pointer created from an integer keeps the relation when the loop has
/// no roots (the compiler gives it none when it accesses memory through such a pointer).
pub proof fn lemma_loop_wildcard(e: AbsState, a: AbsState, b: AbsState, is_write: bool, code: u8,
    roots: ISet<nat>)
    requires
        loop_rel(e, a, roots),
        forall|t: nat| t < a.count ==> node_wf(#[trigger] a.nodes[t]),
        wildcard_refines(a, b, is_write, code),
        code != ACCESS_UNKNOWN_WILDCARD,
        forall|r: nat| !roots.contains(r),
    ensures loop_rel(e, b, roots),
{
    if a.unknown_exposed {
        lemma_loop_same(e, a, b, roots);
    } else {
        let (wc, wl, wf) = wild_access(a, ISet::empty(), None, is_write);
        if wc != ACCESS_OK {
            lemma_loop_same(e, a, b, roots);
        } else {
            lemma_loop_apply(e, a, b, wl, wf, is_write, code, roots);
        }
    }
}

/// Deallocation: a write access, then a check that changes nothing.
pub proof fn lemma_loop_dealloc(e: AbsState, a: AbsState, b: AbsState, tag: nat, code: u8,
    exposed_values: bool, roots: ISet<nat>)
    requires
        loop_rel(e, a, roots),
        forall|t: nat| t < a.count ==> node_wf(#[trigger] a.nodes[t]),
        dealloc_refines(a, b, tag, code, exposed_values),
        code != ACCESS_UNKNOWN_WILDCARD,
        exposed_values && tag == EXPOSED_TAG as nat ==> forall|r: nat| !roots.contains(r),
        tag < a.count ==> forall|r: nat| r < e.count && #[trigger] roots.contains(r) ==>
            a.nodes[r].cell || a.nodes[eff(a, tag)].ancestors.contains(r),
    ensures loop_rel(e, b, roots),
{
    let (m, c1) = choose|m: AbsState, c1: u8| #[trigger] dealloc_witness(a, b, tag, code, m, c1,
        exposed_values);
    if exposed_values && tag == EXPOSED_TAG as nat {
        lemma_loop_wildcard(e, a, m, true, c1, roots);
    } else if tag < a.count {
        lemma_loop_access(e, a, m, tag, true, c1, roots);
    } else {
        lemma_loop_same(e, a, m, roots);
    }
    lemma_loop_same(e, m, b, roots);
}

/// The end of a protector: an access through `p` that its descendants do not see.
pub proof fn lemma_loop_end_protector(e: AbsState, a: AbsState, b: AbsState, p: nat, code: u8,
    roots: ISet<nat>)
    requires
        loop_rel(e, a, roots),
        forall|t: nat| t < a.count ==> node_wf(#[trigger] a.nodes[t]),
        end_protector_refines(a, b, p, code),
        code != ACCESS_UNKNOWN_WILDCARD,
        forall|r: nat| r < e.count && #[trigger] roots.contains(r) ==> a.nodes[r].cell
            || a.nodes[p].ancestors.contains(r),
    ensures loop_rel(e, b, roots),
{
    let n = a.nodes[p];
    if !(p < a.count && n.protected && n.initialized && n.perm != Perm::Disabled && !n.cell) {
        lemma_loop_same(e, a, b, roots);
    } else {
        let f = ISet::new(|t: nat| t < a.count && !a.nodes[p].ancestors.contains(t)
            && !desc_of(a, p, t) && !a.nodes[t].cell && !end_skipped(a, p, t));
        lemma_loop_apply(e, a, b, local_set(a, p), f, n.perm == Perm::Active, code, roots);
    }
}

/// A new reference adds a tag and changes no other.
pub proof fn lemma_loop_new_borrow(e: AbsState, a: AbsState, b: AbsState, parent: nat,
    covers: bool, in_cell: bool, freeze: bool, is_shared: bool, is_protected: bool,
    is_strong: bool, r: nat, exposed_values: bool, roots: ISet<nat>)
    requires
        loop_rel(e, a, roots),
        retag_spec(a, b, parent, covers, in_cell, freeze, is_shared, is_protected, is_strong, r,
            exposed_values),
    ensures loop_rel(e, b, roots),
{
    if b.count == a.count + 1 && r == a.count {
        assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], e.nodes[t]) by {
            lemma_node_eq_loop_le(b.nodes[t], a.nodes[t], e.nodes[t]);
        }
    } else {
        lemma_loop_same(e, a, b, roots);
    }
}

/// Exposing a tag, or recording that a pointer with it was stored, only sets a flag.
pub proof fn lemma_loop_expose(e: AbsState, a: AbsState, b: AbsState, tag: nat,
    roots: ISet<nat>)
    requires loop_rel(e, a, roots), expose_spec(a, b, tag),
    ensures loop_rel(e, b, roots),
{
    assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], a.nodes[t])
        && (roots.contains(t) ==> root_keep(b.nodes[t].perm, a.nodes[t].perm)) by {
        if tag < a.count && t == eff(a, tag) {
            assert(b.nodes[t].ancestors =~= a.nodes[t].ancestors);
        } else {
            assert(node_eq(b.nodes[t], a.nodes[t]));
            assert(b.nodes[t].ancestors =~= a.nodes[t].ancestors);
        }
    }
    lemma_loop_rel_step(e, a, b, roots);
}

pub proof fn lemma_loop_store(e: AbsState, a: AbsState, b: AbsState, tag: nat, roots: ISet<nat>)
    requires loop_rel(e, a, roots), store_spec(a, b, tag),
    ensures loop_rel(e, b, roots),
{
    assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], a.nodes[t])
        && (roots.contains(t) ==> root_keep(b.nodes[t].perm, a.nodes[t].perm)) by {
        if tag < a.count && t == tag {
            assert(b.nodes[t].ancestors =~= a.nodes[t].ancestors);
        } else {
            assert(node_eq(b.nodes[t], a.nodes[t]));
            assert(b.nodes[t].ancestors =~= a.nodes[t].ancestors);
        }
    }
    lemma_loop_rel_step(e, a, b, roots);
}

/// The return of a function called in the loop (its frame starts at or after the loop's first
/// tag) only drops or unprotects tags created in the loop.
pub proof fn lemma_loop_return(e: AbsState, a: AbsState, b: AbsState, frame: nat, kept: Seq<u8>,
    roots: ISet<nat>)
    requires
        loop_rel(e, a, roots), return_spec(a, b, frame, kept), e.count <= frame <= a.count,
        frame < 64, kept.len() == 4,
    ensures loop_rel(e, b, roots),
{
    assert(kept_count(a.count, frame, kept) >= frame);
    assert forall|t: nat| t < e.count implies loop_le(#[trigger] b.nodes[t], e.nodes[t]) by {
        lemma_node_eq_loop_le(b.nodes[t], a.nodes[t], e.nodes[t]);
    }
}

/// A return when no tag at or above the frame exists changes nothing.
proof fn lemma_return_nothing(s: MonitorState, frame: u8, kept: [u8; 4])
    requires monitor_wf(&s), frame >= 1, s.count <= frame,
    ensures return_spec(abstraction(&s), abstraction(&s), frame as nat, kept@),
{
    let a = abstraction(&s);
    let k = kept@;
    assert(k.len() == 4);
    let step = |c: nat, x: u8| if frame as nat <= x && (x as nat) < a.count && c <= x { x as nat + 1 } else { c };
    assert(step(frame as nat, k[0]) == frame as nat);
    assert(step(step(frame as nat, k[0]), k[1]) == frame as nat);
    assert(step(step(step(frame as nat, k[0]), k[1]), k[2]) == frame as nat);
    assert(step(step(step(step(frame as nat, k[0]), k[1]), k[2]), k[3]) == frame as nat);
    assert(kept_count(a.count, frame as nat, k) == a.count);
    assert forall|t: nat| t < a.count implies node_eq(#[trigger] a.nodes[t],
        if frame < 64 && t >= frame { unprotect(a.nodes[t]) } else { a.nodes[t] }) by {
        assert(a.nodes[t].ancestors =~= a.nodes[t].ancestors);
    }
}

} // verus!
