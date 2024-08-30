// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![allow(unused)] // All functions hidden; some may be queried by diagnostic
//! The stacked borrows state.
//!
//! The stacked borrows state associates every pointer value
//! (IE reference or raw pointer) with a unique tag and permission
//! in shadow memory.
//!
//! The tags correspond to the time of the creation of the pointer
//! value, and the permissions correspond to the mutability
//! of the pointer value and its status as a raw pointer or reference.
//!
//! It also associates each byte of the program's memory
//! with a stack of tags, tracking the borrows of the memory
//! containing that byte in temporal order. Every time a
//! pointer value is used, the stack is popped down to that pointer value's
//! tag, effectively marking the borrows that occur after that variable
//! as dead. If the borrows associated with the tags popped are later used,
//! the search for them at that byte fails and the stacked borrows state
//! is considered violated.
//!
//! For example:
//! ```rust
//! // Stack allocate 10 and store it at x.
//! // Stack at addr_of!(x) through addr_of!(x) + 4:
//! // [(TAG_0, Permission::UNIQUE)]
//! let mut x: i32 = 10;
//! // Make the pointer object `&mut x`. Associate `&mut x`
//! // with the tag and permission `(1, Permission::UNIQUE)`
//! // by associating `addr_of!(y)` with `(1, Permission::UNIQUE)`
//! // in shadow memory. Push the tag to the borrow stacks of
//! // `addr_of!(x)` through `addr_of!(x) + 4` yielding
//! // the stacks [(TAG_0, Permission::UNIQUE), (TAG_1, Permission::UNIQUE)]
//! let y = &mut x;
//! // Associate `addr_of!(z)` and push the stacks as
//! // above with the tag (2, Permission::SHAREDRW),
//! // corresponding to a raw pointer, yielding the stacks
//! // [(TAG_0, Permission::UNIQUE), (TAG_1, Permission::UNIQUE),
//! //  (TAG_2, Permission::SHAREDRW)].
//! let z = y as *mut i32;
//! // Pop elements from the pointee object stack until it matches
//! // the tag associated with the pointer value, yielding
//! // [(TAG_0, Permission::UNIQUE), (TAG_1, Permission::UNIQUE)]
//! *y = 10;
//! // Run stack lookup on the tag associated with the pointer
//! // object created at `y as *mut i32`, ie, (TAG_2, Permission::SHAREDRW)
//! // resulting in an error.
//! unsafe { *(&mut *z) = 10; }
//! ```
//! When demonic nondeterminism is used (currently it is always used),
//! a nondeterminism oracle is queried to select a single byte of the program's
//! memory. This way, if a single byte is ever invalid, the nondeterminism
//! oracle will select it, and allow an error to be thrown.
//! This can be used with the restriction that assertions over
//! relations between the stacks (such as, for example, equality between
//! the top two tags of two different stacks) are never needed.

use crate::mem::{pointer_object, pointer_offset};
use crate::shadow::ShadowMem;

/// Bounds on the space usage of the analysis
mod limits {
    pub(super) const STACK_DEPTH: usize = 15;
}

/// Types used in the analysis
mod types {
    /// Pointer tag.
    /// Up to 256 pointers are tracked; and so they are
    /// given a symbolic name by the PointerTag type.
    pub(super) type PointerTag = u8;

    /// Access bit.
    /// Encoded as associated constants
    /// instead of as an enum to ensure
    /// that the representation uses
    /// 1 bit.
    pub(super) type AccessBit = bool;
    pub(super) struct Access;
    impl Access {
        pub(super) const READ: AccessBit = false;
        pub(super) const WRITE: AccessBit = true;
    }

    /// Type of permission.
    /// To ensure that 8 bit, instead of larger,
    /// repreesentations are used in cbmc, this
    /// is encoded using associated constants.
    pub(super) type PermissionByte = u8;
    pub(super) struct Permission;
    impl Permission {
        /// Unique ownership of a memory location
        pub(super) const UNIQUE: u8 = 0;
        /// Raw pointer read/write permission
        pub(super) const SHAREDRW: u8 = 1;
        /// Raw pointer read permission
        pub(super) const SHAREDRO: u8 = 2;
        /// Disabled -- no accesses allowed
        pub(super) const DISABLED: u8 = 3;
    }

    impl Permission {
        /// Returns whether the access bit is granted by the permission
        /// byte
        pub(super) fn grants(access: AccessBit, perm: PermissionByte) -> bool {
            perm != Permission::DISABLED
                && (access != Access::WRITE || perm != Permission::SHAREDRO)
        }
    }

    /// Tracks whether the monitor is on or off.
    /// Encoded as associated constants instead
    /// of as an enum to ensure that the representation
    /// uses 1 bit.
    pub(super) type MonitorBit = bool;
    pub(super) struct MonitorState;
    impl MonitorState {
        pub(super) const ON: MonitorBit = false;
        pub(super) const OFF: MonitorBit = true;
    }
}

// The global state of the analysis.
mod global {
    use super::limits::*;
    use super::types::*;
    use super::ShadowMem;

    pub(super) const INITIAL_TAG: PointerTag = 0;

    /// Associate every pointer object with a tag
    pub(super) static mut TAGS: ShadowMem<PointerTag> = ShadowMem::new(INITIAL_TAG);

    /// Associate every pointer object with a permission
    pub(super) static mut PERMS: ShadowMem<PermissionByte> = ShadowMem::new(Permission::SHAREDRO);

    /// Next pointer id: the next pointer id in sequence
    pub(super) static mut NEXT_TAG: PointerTag = INITIAL_TAG;

    /// Set to true whenever the stack has been
    /// invalidated by a failed lookup.
    pub(super) static mut STACK_VALID: bool = true;

    /// Object + offset being monitored
    pub(super) static mut MONITORED: *const u8 = std::ptr::null();
    /// The tags of the pointer objects borrowing the byte
    pub(super) static mut STACK_TAGS: [PointerTag; STACK_DEPTH] = [INITIAL_TAG; STACK_DEPTH];
    /// The permissions of the pointer objects borrowing the byte
    pub(super) static mut STACK_PERMS: [PermissionByte; STACK_DEPTH] =
        [Permission::UNIQUE; STACK_DEPTH];
    /// The "top" of the stack
    pub(super) static mut STACK_TOP: usize = 0;
}

#[rustc_diagnostic_item = "KaniStackValid"]
fn stack_valid() -> bool {
    unsafe { global::STACK_VALID }
}

/// Manipulation of the monitor of the stacked
/// borrows state
pub(super) mod monitor_transitions {
    use super::global::*;
    use super::limits::*;
    use super::types::*;
    use crate::mem::{pointer_object, pointer_offset};

    fn demonic_nondet() -> bool {
        crate::any()
    }

    #[allow(unused)]
    pub(super) const STACK_DEPTH: usize = 15;

    /// Initialize local when track local is true, picking a monitor,
    /// and setting its object and offset to within pointer.
    pub(super) unsafe fn track_local<U>(tag: PointerTag, pointer: *const U)
    where
        U: Sized,
    {
        // Decide whether to initialize the stacks
        // for location:location+size_of(U).
        unsafe {
            if demonic_nondet() {
                let offset: usize = kani::any();
                crate::assume(offset < std::mem::size_of::<U>());
                MONITORED = pointer.byte_add(offset) as *const u8;
                STACK_TAGS[STACK_TOP] = tag;
                STACK_PERMS[STACK_TOP] = Permission::UNIQUE;
                STACK_TOP += 1;
            }
        }
    }

    /// Push a tag with a permission perm at pointer
    pub(super) fn push<U>(tag: PointerTag, perm: PermissionByte, pointer: *const U)
    where
        U: Sized,
    {
        // Decide whether to initialize the stacks
        // for location:location+size_of(U).
        // Offset has already been picked earlier.
        unsafe {
            use self::*;
            if pointer_object(MONITORED) == pointer_object(pointer)
                && pointer_offset(MONITORED) <= std::mem::size_of::<U>()
            {
                STACK_TAGS[STACK_TOP] = tag;
                STACK_PERMS[STACK_TOP] = perm;
                STACK_TOP += 1;
            }
        }
    }

    /// Run a stack check on the monitored byte for the given
    /// tag and the given access permission.
    pub(super) fn stack_check(tag: PointerTag, access: AccessBit) {
        unsafe {
            use self::*;
            let mut found = false;
            let mut j = 0;
            let mut new_top = 0;
            crate::assert(STACK_TOP < STACK_DEPTH, "Max # of nested borrows (15) exceeded");
            while j < STACK_DEPTH {
                if j < STACK_TOP {
                    let id = STACK_TAGS[j];
                    let kind = STACK_PERMS[j];
                    if Permission::grants(access, kind) && id == tag {
                        new_top = j + 1;
                        found = true;
                    }
                }
                j += 1;
            }
            STACK_TOP = new_top;
            STACK_VALID = STACK_VALID && found;
        }
    }
}

/// Push the permissions at the given location
fn push<U>(tag: types::PointerTag, perm: types::PermissionByte, address: *const U) {
    self::monitor_transitions::push(tag, perm, address)
}

/// Initialize the local stored at reference if initialized is set to false,
/// and track it using a monitor when using demonic nondeterminism.
///
/// Every function call in the source program stack-allocates
/// the local variables that it uses; references to these
/// variables are only valid after these variables are initialized (first written).
/// Therefore this function can be used by supplying an initialized flag
/// set to true after the first write, a track flag set to the value
/// of a query to a demonic nondeterminism oracle (when this feature is used)
/// and a reference to the stack location.
#[rustc_diagnostic_item = "KaniInitializeLocal"]
fn initialize_local<U>(pointer: *const U) {
    unsafe {
        let tag = global::NEXT_TAG;
        global::TAGS.set(pointer, tag);
        global::PERMS.set(pointer, types::Permission::UNIQUE);
        global::NEXT_TAG += 1;
        monitor_transitions::track_local(tag, pointer);
    }
}

/// Stack check the object pointed by pointer_value.
///
/// This is done by checking `crate::mem::pointer_object` and
/// `crate::mem::pointer_offset`, which for arrays:
/// ```rust
///     let mut x: [i32; 100] = [0; 100];
///     let x_ptr: *const [i32; 100] = std::ptr::addr_of!(x);
///     let y = &mut x[10];
///     let y_ptr = y as *mut i32;
///     crate::assert(crate::mem::pointer_object(x_ptr) ==
///                   crate::mem::pointer_object(y_ptr), "pointers =");
/// ```
/// and for fields:
/// ```rust
///    struct TwoElements {
///       x: i32,
///       y: i32,
///    }
///    let mut x: TwoElements = TwoElements { x: 0, y: 0 };
///    let x_ptr: *const TwoElements = std::ptr::addr_of!(x);
///    let y = &mut x.x;
///    let y_ptr = y as *mut i32;
///    crate::assert(crate::mem::pointer_object(x_ptr) ==
///                  crate::mem::pointer_object(y_ptr), "pointers =");
/// ```
/// will succeed, given that offsets within the same allocation
/// are considered parts of the same pointer object by cbmc.
#[rustc_diagnostic_item = "KaniStackCheckPtr"]
fn stack_check_ptr<U>(pointer_value: *const *mut U) {
    unsafe {
        let tag = global::TAGS.get(pointer_value);
        let perm = global::PERMS.get(pointer_value);
        let pointer = *pointer_value;
        let size = unsafe { std::mem::size_of_val_raw::<U>(pointer) };
        if pointer_object(pointer) == pointer_object(global::MONITORED)
            && pointer_offset(global::MONITORED) < size
        {
            if types::Permission::grants(types::Access::READ, perm) {
                self::monitor_transitions::stack_check(tag, types::Access::READ);
            } else if types::Permission::grants(types::Access::WRITE, perm) {
                self::monitor_transitions::stack_check(tag, types::Access::WRITE);
            }
        }
    }
}

#[rustc_diagnostic_item = "KaniStackCheckRef"]
fn stack_check_ref<U>(pointer_value: *const &mut U) {
    stack_check_ptr(pointer_value as *const *mut U);
}

/// Update the stacked borrows state for the case created: &mut T = &mut (referent:T)
/// by associating the location of the created value, stored at pointer_to_created,
/// with a new tag, and pushing the new tag to the created reference, stored at
/// pointer_to_val.
#[rustc_diagnostic_item = "KaniNewMutRefFromValue"]
fn new_mut_ref_from_value<T>(pointer_to_created: *const &mut T, pointer_to_val: *const T) {
    unsafe {
        // Then associate the lvalue and push it
        global::TAGS.set(pointer_to_created, global::NEXT_TAG);
        global::PERMS.set(pointer_to_created, types::Permission::SHAREDRW);
        push(global::NEXT_TAG, types::Permission::SHAREDRW, pointer_to_val);
        global::NEXT_TAG += 1;
    }
}

/// Update the stacked borrows state for the case created = (reference: &mut T) as *mut T,
/// associating the location of the created value, stored at pointer_to_created, with a new
/// tag, running a stack check on the tag associated with the reference, accessed by
/// pointer_to_ref, and pushing the tag to the original location.
#[rustc_diagnostic_item = "KaniNewMutRawFromRef"]
fn new_mut_raw_from_ref<T>(pointer_to_created: *const *mut T, pointer_to_ref: *const &mut T) {
    unsafe {
        // Then associate the lvalue and push it
        global::TAGS.set(pointer_to_created, global::NEXT_TAG);
        push(global::NEXT_TAG, types::Permission::SHAREDRW, *pointer_to_ref);
        global::NEXT_TAG += 1;
    }
}

/// Update the stacked borrows state for the case created = (reference: &mut T) as *mut T,
/// associating the location of the created value, stored at pointer_to_created, with a new
/// tag, running a stack check on the tag associated with the reference, accessed by
/// pointer_to_ref, and pushing the tag to the original location.
#[rustc_diagnostic_item = "KaniNewMutRefFromRaw"]
fn new_mut_ref_from_raw<T>(pointer_to_created: *const &mut T, pointer_to_ref: *const *mut T) {
    unsafe {
        // Then associate the lvalue and push it
        global::TAGS.set(pointer_to_created, global::NEXT_TAG);
        push(global::NEXT_TAG, types::Permission::SHAREDRW, *pointer_to_ref);
        global::NEXT_TAG += 1;
    }
}
