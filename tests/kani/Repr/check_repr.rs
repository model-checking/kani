// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#![feature(decl_macro)]

pub type Key = i8;
pub struct Metadata {
    header: u32,
    typ: Option<i8>,
}

impl Metadata {
    pub fn unknown(&self) -> bool {
        self.typ.is_some()
    }
}

pub fn find(key: &Key) -> Result<Metadata, defs::Error> {
    use crate::defs::INVALID_KEY_ERROR;

    let metadata = Metadata { header: 0, typ: None };
    if metadata.unknown() && (*key < 0) {
        return Err(INVALID_KEY_ERROR);
    }
    Ok(metadata)
}

#[cfg(kani)]
mod proof_harnesses {
    use super::*;

    #[kani::proof]
    fn find_error() {
        let key = kani::any::<Key>();
        kani::assume(key > 0);
        assert!(find(&key).is_ok());
    }
}

/// This module is a reduced testcase from file handling in the std.
pub mod defs {

    use std::ptr::NonNull;

    pub(crate) const INVALID_KEY_ERROR: Error = const_io_error!(
        ErrorKind::InvalidInput,
        "the source path is neither a regular file nor a symlink to a regular file",
    );

    /// Create and return an `Error` for a given `ErrorKind` and constant
    /// message. This doesn't allocate.
    pub(crate) macro const_io_error($kind:expr, $message:expr $(,)?) {
        $crate::defs::Error::from_static_message({
            const MESSAGE_DATA: $crate::defs::SimpleMessage =
                $crate::defs::SimpleMessage::new($kind, $message);
            &MESSAGE_DATA
        })
    }

    pub struct Error {
        repr: Repr,
    }

    //#[repr(transparent)]
    pub(super) struct Repr(std::ptr::NonNull<()>);

    impl Repr {
        #[inline]
        pub(super) const fn new_simple_message(m: &'static SimpleMessage) -> Self {
            // Safety: References are never null.
            Self(unsafe { NonNull::new_unchecked(m as *const _ as *mut ()) })
        }
    }

    impl Error {
        /// Creates a new I/O error from a known kind of error as well as a constant
        /// message.
        ///
        /// This function does not allocate.
        ///
        /// You should not use this directly, and instead use the `const_io_error!`
        /// macro: `const_io_error!(ErrorKind::Something, "some_message")`.
        ///
        /// This function should maybe change to `from_static_message<const MSG: &'static
        /// str>(kind: ErrorKind)` in the future, when const generics allow that.
        #[inline]
        pub(crate) const fn from_static_message(msg: &'static SimpleMessage) -> Error {
            Self { repr: Repr::new_simple_message(msg) }
        }
    }

    //#[repr(align(4))]
    #[derive(Debug)]
    pub(crate) struct SimpleMessage {
        kind: ErrorKind,
        message: &'static str,
    }

    impl SimpleMessage {
        pub(crate) const fn new(kind: ErrorKind, message: &'static str) -> Self {
            Self { kind, message }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[allow(deprecated)]
    #[non_exhaustive]
    pub enum ErrorKind {
        NotFound,
        PermissionDenied,
        InvalidInput,
    }
}
