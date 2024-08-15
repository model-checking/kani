const STACK_DEPTH: usize = 15;
type PointerTag = u8;

use crate::mem::pointer_object;
use crate::mem::pointer_offset;
use crate::shadow::ShadowMem;
use crate::{any, assume};

/// The stacked borrows state.
///
/// The stacked borrows state associates every pointer value
/// (IE reference or raw pointer) with a unique tag and permission
/// in shadow memory.
///
/// The tags correspond to the time of the creation of the pointer
/// value, and the permissions correspond to the mutability
/// of the pointer value and its status as a raw pointer or reference.
///
/// It also associates each byte of the program's memory
/// with a stack of tags, tracking the borrows of the memory
/// containing that byte in temporal order. Every time a
/// pointer value is used, the stack is popped down to that pointer value's
/// tag, effectively marking the borrows that occur after that variable
/// as dead. If the borrows associated with the tags popped are later used,
/// the search for them at that byte fails and the stacked borrows state
/// is considered violated.
///
/// For example:
/// ```rust
/// let mut x: i32 = 10;
/// // Stack allocate 10 and store it at x.
/// // Stack at addr_of!(x) through addr_of!(x) + 4:
/// // [(0, Permission::UNIQUE)]
/// let y = &mut x;
/// // Make the pointer object `&mut x`. Associate `&mut x`
/// // with the tag and permission `(1, Permission::UNIQUE)`
/// // by associating `addr_of!(y)` with `(1, Permission::UNIQUE)`
/// // in shadow memory. Push the tag to the borrow stacks of
/// // `addr_of!(x)` through `addr_of!(x) + 4` yielding
/// // the stacks [(0, Permission::UNIQUE), (1, Permission::UNIQUE)]
/// let z = y as *mut i32;
/// // Make the pointer object `y as *mut i32`.
/// // associate `addr_of!(z)` and push the stacks as
/// // above with the tag (2, Permission::SHAREDRW),
/// // corresponding to a raw pointer, yielding the stacks
/// // [(0, Permission::UNIQUE), (1, Permission::UNIQUE),
/// //  (2, Permission::SHAREDRW)].
/// *y = 10;
/// // Pop the stack down to the tag associated with the pointer
/// // object created at `&mut x` yielding
/// // [(0, Permission::UNIQUE), (1, Permission::UNIQUE)]
/// unsafe { *(&mut *z) = 10; }
/// // Run stack lookup on the tag associated with the pointer
/// // object created at `y as *mut i32`, ie, (2, Permission::SHAREDRW)
/// // resulting in an error.
/// ```
/// When demonic nondeterminism is used (currently it is always used),
/// a nondeterminism oracle is queried to select a single byte of the program's
/// memory. This way, if a single byte is ever invalid, the nondeterminism
/// oracle will select it, and allow an error to be thrown.
/// This can be used with the restriction that assertions over
/// relations between the stacks (such as, for example, equality between
/// the top two tags of two different stacks) are never needed.
pub mod sstate {
    use super::*;
    /// Associate every pointer object with a tag
    static mut TAGS: ShadowMem<PointerTag> = ShadowMem::new(0);
    /// Next pointer id: the next pointer id in sequence
    static mut NEXT_TAG: PointerTag = 0;

    /// Set to true whenever the stack has been
    /// invalidated by a failed lookup.
    static mut STACK_VALID: bool = true;

    #[rustc_diagnostic_item = "KaniStackValid"]
    pub fn stack_valid() -> bool {
        unsafe {STACK_VALID}
    }

    #[non_exhaustive]
    struct Access;
    #[allow(unused)]
    impl Access {
        const READ: bool = false;
        const WRITE: bool = true;
    }

    #[non_exhaustive]
    struct Permission;
    impl Permission {
        const UNIQUE: u8 = 0;
        const SHAREDRW: u8 = 1;
        const SHAREDRO: u8 = 2;
        const DISABLED: u8 = 3;

        fn grants(access: bool, tag: u8) -> bool {
            tag != Self::DISABLED && (access != Access::WRITE || tag != Self::SHAREDRO)
        }
    }

    /// Associate every pointer object with a permission
    static mut PERMS: ShadowMem<u8> = ShadowMem::new(0);

    pub(super) mod monitors {
        static mut STATE: bool = false;
        static mut OBJECT: usize = 0;
        static mut OFFSET: usize = 0;
        static mut STACK_TAGS: [u8; STACK_DEPTH] = [0; STACK_DEPTH];
        static mut STACK_PERMS: [u8; STACK_DEPTH] = [0; STACK_DEPTH];
        static mut STACK_TOP: usize = 0;

        #[non_exhaustive]
        struct MonitorState;
        impl MonitorState {
            const UNINIT: bool = false;
            const INIT: bool = true;
        }

        use super::*;

        /// Monitors:
        /// If there are K bytes in the address space,
        /// every stacked borrows instrumentation has
        /// between 0 and K monitors.
        /// These monitors track a single byte of the program,
        /// associating it with a stack of pointer values
        /// (represented by tags).
        /// Whenever a pointer borrows an object containing
        /// the byte, its tag is pushed to the stack;
        /// when a read or write is performed through this pointer,
        /// writes from pointers above its location on the stack
        /// are disabled.
        /// This function prepares N monitors,
        /// writes them to global heap memory, then
        /// stores them in pointers.
        /// An N+1th monitor is allocated as a "garbage"
        /// area to be used when no monitor is picked.
        pub fn prepare_monitors() {
            unsafe {
                OBJECT = 0usize;
                OFFSET = 0usize;
                STATE = MonitorState::UNINIT;
                STACK_TAGS = [NEXT_TAG; STACK_DEPTH];
                STACK_PERMS = [Permission::UNIQUE; STACK_DEPTH];
                STACK_TOP = 0usize;
            }
        }

        /// Initialize local when track local is true, picking a monitor,
        /// and setting its object and offset to within pointer.
        pub(super) unsafe fn track_local<U>(tag: u8, pointer: *const U) {
            // Decide whether to initialize the stacks
            // for location:location+size_of(U).
            // Offset has already been picked earlier.
            unsafe {
                if demonic_nondet() && STATE == MonitorState::UNINIT {
                    STATE = MonitorState::INIT;
                    OBJECT = pointer_object(pointer);
                    assume(OFFSET < std::mem::size_of::<U>());
                    STACK_TAGS[STACK_TOP] = tag;
                    STACK_PERMS[STACK_TOP] = Permission::UNIQUE;
                    STACK_TOP += 1;
                }
            }
        }

        /// Push a tag with a permission perm at pointer
        pub(super) fn push<U>(tag: u8, perm: u8, pointer: *const U) {
            // Decide whether to initialize the stacks
            // for location:location+size_of(U).
            // Offset has already been picked earlier.
            unsafe {
                use self::*;
                if STATE == MonitorState::INIT
                    && OBJECT == pointer_object(pointer)
                    && OFFSET == pointer_offset(pointer)
                {
                    STACK_TAGS[STACK_TOP] = tag;
                    STACK_PERMS[STACK_TOP] = perm;
                    STACK_TOP += 1;
                }
            }
        }

        pub(super) fn stack_check<U>(tag: u8, access: bool, address: *const U) {
            unsafe {
                use self::*;
                if STATE == MonitorState::INIT
                    && OFFSET == pointer_offset(address)
                    && OBJECT == pointer_object(address)
                {
                    let mut found = false;
                    let mut j = 0;
                    let mut new_top = 0;
                    assert!(STACK_TOP < STACK_DEPTH);
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

    #[rustc_diagnostic_item = "KaniInitializeSState"]
    pub fn initialize() {
        self::monitors::prepare_monitors();
    }

    /// Push the permissions at the given location
    pub fn push<U>(tag: u8, perm: u8, address: *const U) {
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
    pub fn initialize_local<U>(pointer: *const U) {
        unsafe {
            let tag = NEXT_TAG;
            TAGS.set(pointer, tag);
            PERMS.set(pointer, Permission::UNIQUE);
            NEXT_TAG += 1;
            monitors::track_local(tag, pointer);
        }
    }

    #[rustc_diagnostic_item = "KaniStackCheckPtr"]
    pub fn stack_check_ptr<U>(pointer_value: *const *mut U) {
        unsafe {
            let tag = TAGS.get(pointer_value);
            let perm = PERMS.get(pointer_value);
            let pointer = *pointer_value;
            for i in 0..std::mem::size_of::<U>() {
                for access in [false, true] {
                    if Permission::grants(access, perm) {
                        self::monitors::stack_check(tag, access, pointer.byte_add(i));
                    }
                }
            }
        }
    }

    #[rustc_diagnostic_item = "KaniStackCheckRef"]
    pub fn stack_check_ref<U>(pointer_value: *const &mut U) {
        stack_check_ptr(pointer_value as *const *mut U);
    }

    /// Update the stacked borrows state for the case created: &mut T = &mut (referent:T)
    /// by associating the location of the created value, stored at pointer_to_created,
    /// with a new tag, and pushing the new tag to the created reference, stored at
    /// pointer_to_val.
    #[rustc_diagnostic_item = "KaniNewMutRefFromValue"]
    pub fn new_mut_ref_from_value<T>(pointer_to_created: *const &mut T, pointer_to_val: *const T) {
        unsafe {
            // Then associate the lvalue and push it
            TAGS.set(pointer_to_created, NEXT_TAG);
            push(NEXT_TAG, Permission::SHAREDRW, pointer_to_val);
            NEXT_TAG += 1;
        }
    }

    /// Update the stacked borrows state for the case created = (reference: &mut T) as *mut T,
    /// associating the location of the created value, stored at pointer_to_created, with a new
    /// tag, running a stack check on the tag associated with the reference, accessed by
    /// pointer_to_ref, and pushing the tag to the original location.
    #[rustc_diagnostic_item = "KaniNewMutRawFromRef"]
    pub fn new_mut_raw_from_ref<T>(
        pointer_to_created: *const *mut T,
        pointer_to_ref: *const &mut T,
    ) {
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
    pub fn new_mut_ref_from_raw<T>(
        pointer_to_created: *const &mut T,
        pointer_to_ref: *const *mut T,
    ) {
        unsafe {
            // Then associate the lvalue and push it
            TAGS.set(pointer_to_created, NEXT_TAG);
            push(NEXT_TAG, Permission::SHAREDRW, *pointer_to_ref);
            NEXT_TAG += 1;
        }
    }
}

pub fn demonic_nondet() -> bool {
    any::<bool>()
}
