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
//! // Make the pointer object `y as *mut i32`.
//! // associate `addr_of!(z)` and push the stacks as
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

use std::ptr::read;

use monitors::MONITORED;

use crate::mem::{pointer_object, pointer_offset};
use crate::shadow::ShadowMem;

type PointerTag = u8;

use super::*;
/// Associate every pointer object with a tag
static mut TAGS: ShadowMem<PointerTag> = ShadowMem::new(0);
/// Next pointer id: the next pointer id in sequence
const INITIAL_TAG: PointerTag = 0;
static mut NEXT_TAG: PointerTag = INITIAL_TAG;

/// Set to true whenever the stack has been
/// invalidated by a failed lookup.
static mut STACK_VALID: bool = true;

#[rustc_diagnostic_item = "KaniStackValid"]
fn stack_valid() -> bool {
    unsafe { STACK_VALID }
}

/// Access bit.
/// Encoded as associated constants
/// instead of as an enum to ensure
/// that the representation uses
/// 1 bit.
type AccessBit = bool;
struct Access;
impl Access {
    const READ: AccessBit = false;
    const WRITE: AccessBit = true;
}

/// Type of permission.
/// To ensure that 8 bit, instead of larger,
/// repreesentations are used in cbmc, this
/// is encoded using associated constants.
type PermissionByte = u8;
struct Permission;
impl Permission {
    /// Unique ownership of a memory location
    const UNIQUE: u8 = 0;
    /// Raw pointer read/write permission
    const SHAREDRW: u8 = 1;
    /// Raw pointer read permission
    const SHAREDRO: u8 = 2;
    /// Disabled -- no accesses allowed
    const DISABLED: u8 = 3;
}

impl Permission {
    /// Returns whether the access bit is granted by the permission
    /// byte
    fn grants(access: AccessBit, perm: PermissionByte) -> bool {
        perm != Permission::DISABLED && (access != Access::WRITE || perm != Permission::SHAREDRO)
    }
}

/// Associate every pointer object with a permission
static mut PERMS: ShadowMem<PermissionByte> = ShadowMem::new(Permission::SHAREDRO);

/// State of the borrows stack monitor for a byte
pub(super) mod monitors {
    /// Tracks whether the monitor is on or off.
    /// Encoded as associated constants instead
    /// of as an enum to ensure that the representation
    /// uses 1 bit.
    type MonitorBit = bool;
    struct MonitorState;
    impl MonitorState {
        const ON: MonitorBit = false;
        const OFF: MonitorBit = true;
    }

    #[allow(unused)]
    const STACK_DEPTH: usize = 15;

    /// Whether the monitor is on. Initially, the monitor is
    /// "off", and it will remain so until an allocation is found
    /// to track.
    static mut STATE: MonitorBit = MonitorState::OFF;
    /// Object + offset being monitored
    pub static mut MONITORED: *const u8 = std::ptr::null();
    /// The tags of the pointer objects borrowing the byte
    static mut STACK_TAGS: [PointerTag; STACK_DEPTH] = [0; STACK_DEPTH];
    /// The permissions of the pointer objects borrowing the byte
    static mut STACK_PERMS: [PermissionByte; STACK_DEPTH] = [Permission::UNIQUE; STACK_DEPTH];
    /// The "top" of the stack
    static mut STACK_TOP: usize = 0;

    use super::*;

    /// Initialize local when track local is true, picking a monitor,
    /// and setting its object and offset to within pointer.
    pub(super) unsafe fn track_local<U>(tag: u8, pointer: *const U) {
        // Decide whether to initialize the stacks
        // for location:location+size_of(U).
        unsafe {
            if demonic_nondet() && STATE == MonitorState::OFF {
                STATE = MonitorState::ON;
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
    pub(super) fn push<U>(tag: u8, perm: PermissionByte, pointer: *const U) {
        // Decide whether to initialize the stacks
        // for location:location+size_of(U).
        // Offset has already been picked earlier.
        unsafe {
            use self::*;
            if STATE == MonitorState::ON
                && pointer_object(MONITORED) == pointer_object(pointer)
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
    pub(super) fn stack_check(tag: u8, access: AccessBit) {
        unsafe {
            use self::*;
            if STATE == MonitorState::ON {
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
}

/// Push the permissions at the given location
fn push<U>(tag: u8, perm: PermissionByte, address: *const U) {
    self::monitors::push(tag, perm, address)
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
        let tag = NEXT_TAG;
        TAGS.set(pointer, tag);
        PERMS.set(pointer, Permission::UNIQUE);
        NEXT_TAG += 1;
        monitors::track_local(tag, pointer);
    }
}

#[rustc_diagnostic_item = "KaniStackCheckPtr"]
fn stack_check_ptr<U>(pointer_value: *const *mut U) {
    unsafe {
        let tag = TAGS.get(pointer_value);
        let perm = PERMS.get(pointer_value);
        let pointer = *pointer_value;
        if pointer_object(pointer) == pointer_object(MONITORED)
            && pointer_offset(MONITORED) < std::mem::size_of::<U>()
        {
            if Permission::grants(Access::READ, perm) {
                self::monitors::stack_check(tag, Access::READ);
            } else if Permission::grants(Access::WRITE, perm) {
                self::monitors::stack_check(tag, Access::WRITE);
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
        TAGS.set(pointer_to_created, NEXT_TAG);
        PERMS.set(pointer_to_created, Permission::SHAREDRW);
        push(NEXT_TAG, Permission::SHAREDRW, pointer_to_val);
        NEXT_TAG += 1;
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
        TAGS.set(pointer_to_created, NEXT_TAG);
        push(NEXT_TAG, Permission::SHAREDRW, *pointer_to_ref);
        NEXT_TAG += 1;
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
        TAGS.set(pointer_to_created, NEXT_TAG);
        push(NEXT_TAG, Permission::SHAREDRW, *pointer_to_ref);
        NEXT_TAG += 1;
    }
}

fn demonic_nondet() -> bool {
    crate::any()
}
