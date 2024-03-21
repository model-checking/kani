// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

use std::alloc::Layout;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use std::ptr::{self, NonNull};

/// This replaces the value behind the `v` unique reference by calling the
/// relevant function, and returns a result obtained along the way.
///
/// If a panic occurs in the `change` closure, the entire process will be aborted.
#[inline]
fn replace<T, R>(v: &mut T, change: impl FnOnce(T) -> (T, R)) -> R {
    let value = unsafe { ptr::read(v) };
    let (new_value, ret) = change(value);
    unsafe {
        ptr::write(v, new_value);
    }
    ret
}

const B: usize = 6;
const CAPACITY: usize = 2 * B - 1;

/// The underlying representation of leaf nodes and part of the representation of internal nodes.
struct LeafNode {
    /// We want to be covariant in `K` and `V`.
    parent: Option<NonNull<InternalNode>>,

    /// This node's index into the parent node's `edges` array.
    /// `*node.parent.edges[node.parent_idx]` should be the same thing as `node`.
    /// This is only guaranteed to be initialized when `parent` is non-null.
    parent_idx: u16,

    /// The number of keys and values this node stores.
    len: u16,
}

impl LeafNode {
    /// Creates a new boxed `LeafNode`.
    fn new() -> Box<Self> {
        Box::new(LeafNode { parent: None, parent_idx: 0, len: 0 })
    }
}

/// The underlying representation of internal nodes. As with `LeafNode`s, these should be hidden
/// behind `BoxedNode`s to prevent dropping uninitialized keys and values. Any pointer to an
/// `InternalNode` can be directly cast to a pointer to the underlying `LeafNode` portion of the
/// node, allowing code to act on leaf and internal nodes generically without having to even check
/// which of the two a pointer is pointing at. This property is enabled by the use of `repr(C)`.
// gdb_providers.py uses this type name for introspection.
struct InternalNode {}

// N.B. `NodeRef` is always covariant in `K` and `V`, even when the `BorrowType`
// is `Mut`. This is technically wrong, but cannot result in any unsafety due to
// internal use of `NodeRef` because we stay completely generic over `K` and `V`.
// However, whenever a public type wraps `NodeRef`, make sure that it has the
// correct variance.
///
/// A reference to a node.
///
/// This type has a number of parameters that controls how it acts:
/// - `BorrowType`: A dummy type that describes the kind of borrow and carries a lifetime.
///    - When this is `Immut<'a>`, the `NodeRef` acts roughly like `&'a Node`.
///    - When this is `ValMut<'a>`, the `NodeRef` acts roughly like `&'a Node`
///      with respect to keys and tree structure, but also allows many
///      mutable references to values throughout the tree to coexist.
///    - When this is `Mut<'a>`, the `NodeRef` acts roughly like `&'a mut Node`,
///      although insert methods allow a mutable pointer to a value to coexist.
///    - When this is `Owned`, the `NodeRef` acts roughly like `Box<Node>`,
///      but does not have a destructor, and must be cleaned up manually.
///    - When this is `Dying`, the `NodeRef` still acts roughly like `Box<Node>`,
///      but has methods to destroy the tree bit by bit, and ordinary methods,
///      while not marked as unsafe to call, can invoke UB if called incorrectly.
///   Since any `NodeRef` allows navigating through the tree, `BorrowType`
///   effectively applies to the entire tree, not just to the node itself.
/// - `K` and `V`: These are the types of keys and values stored in the nodes.
/// - `Type`: This can be `Leaf`, `Internal`, or `LeafOrInternal`. When this is
///   `Leaf`, the `NodeRef` points to a leaf node, when this is `Internal` the
///   `NodeRef` points to an internal node, and when this is `LeafOrInternal` the
///   `NodeRef` could be pointing to either type of node.
///   `Type` is named `NodeType` when used outside `NodeRef`.
///
/// Both `BorrowType` and `NodeType` restrict what methods we implement, to
/// exploit static type safety. There are limitations in the way we can apply
/// such restrictions:
/// - For each type parameter, we can only define a method either generically
///   or for one particular type. For example, we cannot define a method like
///   `into_kv` generically for all `BorrowType`, or once for all types that
///   carry a lifetime, because we want it to return `&'a` references.
///   Therefore, we define it only for the least powerful type `Immut<'a>`.
/// - We cannot get implicit coercion from say `Mut<'a>` to `Immut<'a>`.
///   Therefore, we have to explicitly call `reborrow` on a more powerful
///   `NodeRef` in order to reach a method like `into_kv`.
///
/// All methods on `NodeRef` that return some kind of reference, either:
/// - Take `self` by value, and return the lifetime carried by `BorrowType`.
///   Sometimes, to invoke such a method, we need to call `reborrow_mut`.
/// - Take `self` by reference, and (implicitly) return that reference's
///   lifetime, instead of the lifetime carried by `BorrowType`. That way,
///   the borrow checker guarantees that the `NodeRef` remains borrowed as long
///   as the returned reference is used.
///   The methods supporting insert bend this rule by returning a raw pointer,
///   i.e., a reference without any lifetime.
struct NodeRef<BorrowType, Type> {
    /// The number of levels that the node and the level of leaves are apart, a
    /// constant of the node that cannot be entirely described by `Type`, and that
    /// the node itself does not store. We only need to store the height of the root
    /// node, and derive every other node's height from it.
    /// Must be zero if `Type` is `Leaf` and non-zero if `Type` is `Internal`.
    height: usize,
    /// The pointer to the leaf or internal node. The definition of `InternalNode`
    /// ensures that the pointer is valid either way.
    node: NonNull<LeafNode>,
    _marker: PhantomData<(BorrowType, Type)>,
}

/// The root node of an owned tree.
///
/// Note that this does not have a destructor, and must be cleaned up manually.
type Root = NodeRef<marker::Owned, marker::Leaf>;

impl NodeRef<marker::Owned, marker::Leaf> {
    fn new_leaf() -> Self {
        Self::from_new_leaf(LeafNode::new())
    }

    fn from_new_leaf(leaf: Box<LeafNode>) -> Self {
        NodeRef { height: 0, node: NonNull::from(Box::leak(leaf)), _marker: PhantomData }
    }
}

impl<BorrowType> NodeRef<BorrowType, marker::Internal> {
    /// Unpack a node reference that was packed as `NodeRef::parent`.
    fn from_internal(node: NonNull<InternalNode>, height: usize) -> Self {
        debug_assert!(height > 0);
        NodeRef { height, node: node.cast(), _marker: PhantomData }
    }
}

impl<BorrowType, Type> NodeRef<BorrowType, Type> {
    /// Finds the length of the node. This is the number of keys or values.
    /// The number of edges is `len() + 1`.
    /// Note that, despite being safe, calling this function can have the side effect
    /// of invalidating mutable references that unsafe code has created.
    fn len(&self) -> usize {
        // Crucially, we only access the `len` field here. If BorrowType is marker::ValMut,
        // there might be outstanding mutable references to values that we must not invalidate.
        unsafe { usize::from((*Self::as_leaf_ptr(self)).len) }
    }

    /// Exposes the leaf portion of any leaf or internal node.
    ///
    /// Returns a raw ptr to avoid invalidating other references to this node.
    fn as_leaf_ptr(this: &Self) -> *mut LeafNode {
        // The node must be valid for at least the LeafNode portion.
        // This is not a reference in the NodeRef type because we don't know if
        // it should be unique or shared.
        this.node.as_ptr()
    }
}

impl<BorrowType: marker::BorrowType, Type> NodeRef<BorrowType, Type> {
    /// Finds the parent of the current node. Returns `Ok(handle)` if the current
    /// node actually has a parent, where `handle` points to the edge of the parent
    /// that points to the current node. Returns `Err(self)` if the current node has
    /// no parent, giving back the original `NodeRef`.
    ///
    /// The method name assumes you picture trees with the root node on top.
    ///
    /// `edge.descend().ascend().unwrap()` and `node.ascend().unwrap().descend()` should
    /// both, upon success, do nothing.
    fn ascend(self) -> Result<Handle<NodeRef<BorrowType, marker::Internal>, marker::Edge>, Self> {
        // We need to use raw pointers to nodes because, if BorrowType is marker::ValMut,
        // there might be outstanding mutable references to values that we must not invalidate.
        let leaf_ptr: *const _ = Self::as_leaf_ptr(&self);
        unsafe { (*leaf_ptr).parent }
            .as_ref()
            .map(|parent| Handle {
                node: NodeRef::from_internal(*parent, self.height + 1),
                idx: unsafe { usize::from((*leaf_ptr).parent_idx) },
                _marker: PhantomData,
            })
            .ok_or(self)
    }

    fn first_edge(self) -> Handle<Self, marker::Edge> {
        unsafe { Handle::new_edge(self, 0) }
    }
}

impl NodeRef<marker::Dying, marker::Leaf> {
    /// Similar to `ascend`, gets a reference to a node's parent node, but also
    /// deallocates the current node in the process. This is unsafe because the
    /// current node will still be accessible despite being deallocated.
    unsafe fn deallocate_and_ascend(
        self,
    ) -> Option<Handle<NodeRef<marker::Dying, marker::Internal>, marker::Edge>> {
        let height = self.height;
        let node = self.node;
        let ret = self.ascend().ok();
        unsafe {
            std::alloc::dealloc(
                node.as_ptr() as *mut u8,
                if height > 0 { Layout::new::<InternalNode>() } else { Layout::new::<LeafNode>() },
            );
        }
        ret
    }
}

impl<'a, Type> NodeRef<marker::Mut<'a>, Type> {
    /// Borrows exclusive access to the leaf portion of a leaf or internal node.
    fn as_leaf_mut(&mut self) -> &mut LeafNode {
        let ptr = Self::as_leaf_ptr(self);
        // SAFETY: we have exclusive access to the entire node.
        unsafe { &mut *ptr }
    }
}

impl<'a, Type> NodeRef<marker::Mut<'a>, Type> {
    /// Borrows exclusive access to the length of the node.
    fn len_mut(&mut self) -> &mut u16 {
        &mut self.as_leaf_mut().len
    }
}

impl<Type> NodeRef<marker::Owned, Type> {
    /// Mutably borrows the owned root node. Unlike `reborrow_mut`, this is safe
    /// because the return value cannot be used to destroy the root, and there
    /// cannot be other references to the tree.
    fn borrow_mut(&mut self) -> NodeRef<marker::Mut<'_>, Type> {
        NodeRef { height: self.height, node: self.node, _marker: PhantomData }
    }

    /// Irreversibly transitions to a reference that permits traversal and offers
    /// destructive methods and little else.
    fn into_dying(self) -> NodeRef<marker::Dying, Type> {
        NodeRef { height: self.height, node: self.node, _marker: PhantomData }
    }
}

impl<'a> NodeRef<marker::Mut<'a>, marker::Leaf> {
    /// Adds a key-value pair to the end of the node, and returns
    /// a handle to the inserted value.
    ///
    /// # Safety
    ///
    /// The returned handle has an unbound lifetime.
    unsafe fn push_with_handle<'b>(
        &mut self,
    ) -> Handle<NodeRef<marker::Mut<'b>, marker::Leaf>, marker::KV> {
        let len = self.len_mut();
        let idx = usize::from(*len);
        assert!(idx < CAPACITY);
        *len += 1;
        unsafe {
            Handle::new_kv(
                NodeRef { height: self.height, node: self.node, _marker: PhantomData },
                idx,
            )
        }
    }

    /// Adds a key-value pair to the end of the node, and returns
    /// the mutable reference of the inserted value.
    fn push(&mut self) -> i32 {
        // SAFETY: The unbound handle is no longer accessible.
        let _ = unsafe { self.push_with_handle() };
        0
    }
}

impl<BorrowType> NodeRef<BorrowType, marker::Leaf> {
    /// Removes any static information asserting that this node is a `Leaf` node.
    fn forget_type(self) -> NodeRef<BorrowType, marker::Leaf> {
        NodeRef { height: self.height, node: self.node, _marker: PhantomData }
    }
}

impl<BorrowType> NodeRef<BorrowType, marker::Internal> {
    /// Removes any static information asserting that this node is an `Internal` node.
    fn forget_type(self) -> NodeRef<BorrowType, marker::Leaf> {
        NodeRef { height: self.height, node: self.node, _marker: PhantomData }
    }
}

impl<BorrowType> NodeRef<BorrowType, marker::Leaf> {
    /// Checks whether a node is an `Internal` node or a `Leaf` node.
    fn force(self) -> ForceResult<NodeRef<BorrowType, marker::Leaf>> {
        if self.height == 0 {
            ForceResult::Leaf(NodeRef {
                height: self.height,
                node: self.node,
                _marker: PhantomData,
            })
        } else {
            panic!()
        }
    }
}

/// A reference to a specific key-value pair or edge within a node. The `Node` parameter
/// must be a `NodeRef`, while the `Type` can either be `KV` (signifying a handle on a key-value
/// pair) or `Edge` (signifying a handle on an edge).
///
/// Note that even `Leaf` nodes can have `Edge` handles. Instead of representing a pointer to
/// a child node, these represent the spaces where child pointers would go between the key-value
/// pairs. For example, in a node with length 2, there would be 3 possible edge locations - one
/// to the left of the node, one between the two pairs, and one at the right of the node.
struct Handle<Node, Type> {
    node: Node,
    idx: usize,
    _marker: PhantomData<Type>,
}

impl<Node, Type> Handle<Node, Type> {
    /// Retrieves the node that contains the edge or key-value pair this handle points to.
    fn into_node(self) -> Node {
        self.node
    }
}

impl<BorrowType, NodeType> Handle<NodeRef<BorrowType, NodeType>, marker::KV> {
    /// Creates a new handle to a key-value pair in `node`.
    /// Unsafe because the caller must ensure that `idx < node.len()`.
    unsafe fn new_kv(node: NodeRef<BorrowType, NodeType>, idx: usize) -> Self {
        debug_assert!(idx < node.len());

        Handle { node, idx, _marker: PhantomData }
    }

    fn right_edge(self) -> Handle<NodeRef<BorrowType, NodeType>, marker::Edge> {
        unsafe { Handle::new_edge(self.node, self.idx + 1) }
    }
}

impl<BorrowType, NodeType> Handle<NodeRef<BorrowType, NodeType>, marker::Edge> {
    /// Creates a new handle to an edge in `node`.
    /// Unsafe because the caller must ensure that `idx <= node.len()`.
    unsafe fn new_edge(node: NodeRef<BorrowType, NodeType>, idx: usize) -> Self {
        debug_assert!(idx <= node.len());

        Handle { node, idx, _marker: PhantomData }
    }

    fn right_kv(self) -> Result<Handle<NodeRef<BorrowType, NodeType>, marker::KV>, Self> {
        if self.idx < self.node.len() {
            Ok(unsafe { Handle::new_kv(self.node, self.idx) })
        } else {
            Err(self)
        }
    }
}

impl<BorrowType> Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge> {
    fn forget_node_type(self) -> Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge> {
        unsafe { Handle::new_edge(self.node.forget_type(), self.idx) }
    }
}

impl<BorrowType> Handle<NodeRef<BorrowType, marker::Internal>, marker::Edge> {
    fn forget_node_type(self) -> Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge> {
        unsafe { Handle::new_edge(self.node.forget_type(), self.idx) }
    }
}

impl<BorrowType, Type> Handle<NodeRef<BorrowType, marker::Leaf>, Type> {
    /// Checks whether the underlying node is an `Internal` node or a `Leaf` node.
    fn force(self) -> ForceResult<Handle<NodeRef<BorrowType, marker::Leaf>, Type>> {
        match self.node.force() {
            ForceResult::Leaf(node) => {
                ForceResult::Leaf(Handle { node, idx: self.idx, _marker: PhantomData })
            }
        }
    }
}
enum ForceResult<Leaf> {
    Leaf(Leaf),
}

mod marker {
    use std::marker::PhantomData;

    pub enum Leaf {}
    pub enum Internal {}
    pub enum Owned {}
    pub enum Dying {}
    pub struct Mut<'a>(PhantomData<&'a mut ()>);

    pub trait BorrowType {
        /// If node references of this borrow type allow traversing to other
        /// nodes in the tree, this constant is set to `true`. It can be used
        /// for a compile-time assertion.
        const TRAVERSAL_PERMIT: bool = true;
    }
    impl BorrowType for Owned {
        /// Reject traversal, because it isn't needed. Instead traversal
        /// happens using the result of `borrow_mut`.
        /// By disabling traversal, and only creating new references to roots,
        /// we know that every reference of the `Owned` type is to a root node.
        const TRAVERSAL_PERMIT: bool = false;
    }
    impl BorrowType for Dying {}
    impl<'a> BorrowType for Mut<'a> {}

    pub enum KV {}
    pub enum Edge {}
}

enum LazyLeafHandle<BorrowType> {
    Root(NodeRef<BorrowType, marker::Leaf>), // not yet descended
    Edge(Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge>),
}

// `front` and `back` are always both `None` or both `Some`.
struct LazyLeafRange<BorrowType> {
    front: Option<LazyLeafHandle<BorrowType>>,
}

impl<BorrowType> LazyLeafRange<BorrowType> {
    fn none() -> Self {
        LazyLeafRange { front: None }
    }
}

impl LazyLeafRange<marker::Dying> {
    fn take_front(&mut self) -> Option<Handle<NodeRef<marker::Dying, marker::Leaf>, marker::Edge>> {
        match self.front.take()? {
            LazyLeafHandle::Root(root) => Some(root.first_leaf_edge()),
            LazyLeafHandle::Edge(edge) => Some(edge),
        }
    }

    #[inline]
    unsafe fn deallocating_next_unchecked(
        &mut self,
    ) -> Handle<NodeRef<marker::Dying, marker::Leaf>, marker::KV> {
        debug_assert!(self.front.is_some());
        let front = self.init_front().unwrap();
        unsafe { front.deallocating_next_unchecked() }
    }

    #[inline]
    fn deallocating_end(&mut self) {
        if let Some(front) = self.take_front() {
            front.deallocating_end()
        }
    }
}

impl<BorrowType: marker::BorrowType> LazyLeafRange<BorrowType> {
    fn init_front(
        &mut self,
    ) -> Option<&mut Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge>> {
        if let Some(LazyLeafHandle::Root(root)) = &self.front {
            self.front = Some(LazyLeafHandle::Edge(unsafe { ptr::read(root) }.first_leaf_edge()));
        }
        match &mut self.front {
            None => None,
            Some(LazyLeafHandle::Edge(edge)) => Some(edge),
            // SAFETY: the code above would have replaced it.
            Some(LazyLeafHandle::Root(_)) => panic!(),
        }
    }
}

fn full_range<BorrowType: marker::BorrowType>(
    root1: NodeRef<BorrowType, marker::Leaf>,
) -> LazyLeafRange<BorrowType> {
    LazyLeafRange { front: Some(LazyLeafHandle::Root(root1)) }
}

impl NodeRef<marker::Dying, marker::Leaf> {
    /// Splits a unique reference into a pair of leaf edges delimiting the full range of the tree.
    /// The results are non-unique references allowing massively destructive mutation, so must be
    /// used with the utmost care.
    fn full_range(self) -> LazyLeafRange<marker::Dying> {
        // We duplicate the root NodeRef here -- we will never access it in a way
        // that overlaps references obtained from the root.
        full_range(self)
    }
}

impl Handle<NodeRef<marker::Dying, marker::Leaf>, marker::Edge> {
    /// Given a leaf edge handle into a dying tree, returns the next leaf edge
    /// on the right side, and the key-value pair in between, if they exist.
    ///
    /// If the given edge is the last one in a leaf, this method deallocates
    /// the leaf, as well as any ancestor nodes whose last edge was reached.
    /// This implies that if no more key-value pair follows, the entire tree
    /// will have been deallocated and there is nothing left to return.
    ///
    /// # Safety
    /// - The given edge must not have been previously returned by counterpart
    ///   `deallocating_next_back`.
    /// - The returned KV handle is only valid to access the key and value,
    ///   and only valid until the next call to a `deallocating_` method.
    unsafe fn deallocating_next(
        self,
    ) -> Option<(Self, Handle<NodeRef<marker::Dying, marker::Leaf>, marker::KV>)> {
        let mut edge = self.forget_node_type();
        loop {
            edge = match edge.right_kv() {
                Ok(kv) => return Some((unsafe { ptr::read(&kv) }.next_leaf_edge(), kv)),
                Err(last_edge) => match unsafe { last_edge.into_node().deallocate_and_ascend() } {
                    Some(parent_edge) => parent_edge.forget_node_type(),
                    None => return None,
                },
            }
        }
    }

    /// Deallocates a pile of nodes from the leaf up to the root.
    /// This is the only way to deallocate the remainder of a tree after
    /// `deallocating_next` and `deallocating_next_back` have been nibbling at
    /// both sides of the tree, and have hit the same edge. As it is intended
    /// only to be called when all keys and values have been returned,
    /// no cleanup is done on any of the keys or values.
    fn deallocating_end(self) {
        let mut edge = self.forget_node_type();
        while let Some(parent_edge) = unsafe { edge.into_node().deallocate_and_ascend() } {
            edge = parent_edge.forget_node_type();
        }
    }
}

impl Handle<NodeRef<marker::Dying, marker::Leaf>, marker::Edge> {
    /// Moves the leaf edge handle to the next leaf edge and returns the key and value
    /// in between, deallocating any node left behind while leaving the corresponding
    /// edge in its parent node dangling.
    ///
    /// # Safety
    /// - There must be another KV in the direction travelled.
    /// - That KV was not previously returned by counterpart
    ///   `deallocating_next_back_unchecked` on any copy of the handles
    ///   being used to traverse the tree.
    ///
    /// The only safe way to proceed with the updated handle is to compare it, drop it,
    /// or call this method or counterpart `deallocating_next_back_unchecked` again.
    unsafe fn deallocating_next_unchecked(
        &mut self,
    ) -> Handle<NodeRef<marker::Dying, marker::Leaf>, marker::KV> {
        replace(self, |leaf_edge| unsafe { leaf_edge.deallocating_next().unwrap() })
    }
}

impl<BorrowType: marker::BorrowType> NodeRef<BorrowType, marker::Leaf> {
    /// Returns the leftmost leaf edge in or underneath a node - in other words, the edge
    /// you need first when navigating forward (or last when navigating backward).
    #[inline]
    fn first_leaf_edge(self) -> Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge> {
        let node = self;
        match node.force() {
            ForceResult::Leaf(leaf) => return leaf.first_edge(),
        }
    }
}

impl<BorrowType: marker::BorrowType> Handle<NodeRef<BorrowType, marker::Leaf>, marker::KV> {
    /// Returns the leaf edge closest to a KV for forward navigation.
    fn next_leaf_edge(self) -> Handle<NodeRef<BorrowType, marker::Leaf>, marker::Edge> {
        match self.force() {
            ForceResult::Leaf(leaf_kv) => leaf_kv.right_edge(),
        }
    }
}

struct Foo {
    root: Option<Root>,
    length: usize,
}

impl Drop for Foo {
    fn drop(&mut self) {
        drop(unsafe { core::ptr::read(self) }.into_iter())
    }
}

impl IntoIterator for Foo {
    type Item = ();
    type IntoIter = IntoIter;

    fn into_iter(self) -> IntoIter {
        let mut me = ManuallyDrop::new(self);
        if let Some(root) = me.root.take() {
            let full_range = root.into_dying().full_range();

            IntoIter { range: full_range, length: me.length }
        } else {
            IntoIter { range: LazyLeafRange::none(), length: 0 }
        }
    }
}

impl Drop for IntoIter {
    fn drop(&mut self) {
        while let Some(_kv) = self.dying_next() {}
    }
}

impl IntoIter {
    /// Core of a `next` method returning a dying KV handle,
    /// invalidated by further calls to this function and some others.
    fn dying_next(&mut self) -> Option<Handle<NodeRef<marker::Dying, marker::Leaf>, marker::KV>> {
        if self.length == 0 {
            self.range.deallocating_end();
            None
        } else {
            self.length -= 1;
            Some(unsafe { self.range.deallocating_next_unchecked() })
        }
    }
}

impl Iterator for IntoIter {
    type Item = ();

    fn next(&mut self) -> Option<()> {
        None
    }
}

/// An owning iterator over the entries of a `BTreeMap`.
///
/// This `struct` is created by the [`into_iter`] method on [`BTreeMap`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: IntoIterator::into_iter
struct IntoIter {
    range: LazyLeafRange<marker::Dying>,
    length: usize,
}

#[cfg_attr(kani, kani::proof, kani::unwind(3))]
fn main() {
    let mut f = Foo { root: None, length: 0 };
    let mut root: NodeRef<marker::Owned, marker::Leaf> = NodeRef::new_leaf();
    root.borrow_mut().push();
    f.root = Some(root.forget_type());
    f.length = 1;
}
