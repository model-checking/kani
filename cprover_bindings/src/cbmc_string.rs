// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::cell::RefCell;
use string_interner::StringInterner;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;

/// This class implements an interner for Strings.
/// CBMC objects to have a large number of strings which refer to names: symbols, files, etc.
/// These tend to be reused many times, which causes signifcant memory usage.
/// If we intern the strings, each unique string is only allocated once, saving memory.
/// On the stdlib test, interning led to a 15% savings in peak memory usage.
/// Since they're referred to by index, InternedStrings become `Copy`, which simplifies APIs.
/// The downside is that interned strings live the lifetime of the execution.
/// So you should only intern strings that will be used in long-lived data-structures, not temps.
///
/// We use a single global string interner, which is protected by a Mutex (i.e. threadsafe).
/// To create an interned string, either do
/// `let i : InternedString = s.into();` or
/// `let i = s.intern();`
#[derive(Clone, Hash, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternedString(SymbolU32);

// This [StringInterner] is a thread local, letting us get away with less synchronization.
// See the `sync` module below for a full explanation of this choice's consequences.
thread_local! {
    static INTERNER: RefCell<StringInterner<StringBackend>> =
        RefCell::new(StringInterner::default());
}

impl InternedString {
    pub fn is_empty(&self) -> bool {
        self.map(|s| s.is_empty())
    }

    pub fn len(&self) -> usize {
        self.map(|s| s.len())
    }

    /// Apply the function `f` to the interned string, represented as an &str.
    /// Needed because exporting the &str backing the InternedString is blocked by lifetime rules.
    /// Instead, this allows users to operate on the &str when needed.
    pub fn map<T, F: FnOnce(&str) -> T>(&self, f: F) -> T {
        INTERNER.with_borrow(|i| f(i.resolve(self.0).unwrap()))
    }

    pub fn starts_with(&self, pattern: &str) -> bool {
        self.map(|s| s.starts_with(pattern))
    }
}

impl std::fmt::Display for InternedString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        INTERNER.with_borrow(|i| write!(fmt, "{}", i.resolve(self.0).unwrap()))
    }
}
/// Custom-implement Debug, so our debug logging contains meaningful strings, not numbers
impl std::fmt::Debug for InternedString {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        INTERNER.with_borrow(|i| write!(fmt, "{:?}", i.resolve(self.0).unwrap()))
    }
}

impl<T> From<T> for InternedString
where
    T: AsRef<str>,
{
    fn from(s: T) -> InternedString {
        InternedString(INTERNER.with_borrow_mut(|i| i.get_or_intern(s)))
    }
}

impl<T> PartialEq<T> for InternedString
where
    T: AsRef<str>,
{
    fn eq(&self, other: &T) -> bool {
        INTERNER.with_borrow(|i| i.resolve(self.0).unwrap() == other.as_ref())
    }
}

pub trait InternString {
    fn intern(self) -> InternedString;
}

impl<T> InternString for T
where
    T: Into<InternedString>,
{
    fn intern(self) -> InternedString {
        self.into()
    }
}
pub trait InternStringOption {
    fn intern(self) -> Option<InternedString>;
}

impl<T> InternStringOption for Option<T>
where
    T: Into<InternedString>,
{
    fn intern(self) -> Option<InternedString> {
        self.map(|s| s.into())
    }
}

/// At a high level, the key design choice here is to keep our [StringInterner]s as thread locals.
/// This works because whichever thread is generating `SymbolTable`s will be updating the interner in a way that
/// affects serialization, but the serialization doesn't affect the interner in a way that affects generating
/// `SymbolTable`s.
///
/// Thus, it makes a lot of sense to have threads each maintain their own copy of a [StringInterner]. Then, when the main
/// codegen thread wants to tell another thread to write a new `SymbolTable` to disk, it can just send along
/// its master copy of the [StringInterner] that they can use to update theirs.
///
/// To enforce this, [InternedString] is marked `!Send`--preventing them from being sent between threads
/// unless they're wrapped in a [WithInterner](sync::WithInterner) type that will ensure the recieving thread updates
/// its local interner to match the sending thread's.
pub(crate) mod sync {
    use string_interner::{StringInterner, backend::StringBackend};

    use crate::{InternedString, cbmc_string::INTERNER};

    /// The value of an [InternedString] is defined based on a thread local [INTERNER] so they cannot safely
    /// be sent between threads.
    impl !Send for InternedString {}

    /// A type that is only `!Send` because it contains types specific to a thread local [INTERNER]
    /// (e.g. [InternedString]s). This forces users to annotate that the types they want to wrap in [WithInterner]
    /// are `!Send` just for that specific reason rather than using it to make arbitrary types `Send`.
    ///
    /// # Safety
    ///
    /// Should only be implemented for types which are `!Send` **solely** because they contain information specific
    /// to their thread local [INTERNER] (e.g. by containing [InternedString]s).
    pub unsafe trait InternerSpecific {}

    /// Since [WithInterner<T>] guarantees that the inner `T` cannot be accessed without updating the
    /// thread local [INTERNER] to a copy of what was used to generate `T`, it is safe to send between threads,
    /// even if the inner `T` contains [InternedString]s which are not [Send] on their own.
    unsafe impl<T: InternerSpecific> Send for WithInterner<T> {}

    /// A type `T` bundled with the [StringInterner] that was used to generate it.
    ///
    /// The only way to access the inner `T` is by calling `into_inner()`, which will automatically
    /// update the current thread's interner to the interner used the generate `T`,
    /// ensuring interner coherence between the sending & receiving threads.
    pub struct WithInterner<T> {
        interner: StringInterner<StringBackend>,
        inner: T,
    }

    impl<T> WithInterner<T> {
        /// Create a new wrapper of `inner` with a clone of the current thread local [INTERNER].
        pub fn new_with_current(inner: T) -> Self {
            let interner = INTERNER.with_borrow(|i| i.clone());
            WithInterner { interner, inner }
        }

        /// Get the inner wrapped `T` and implicitly update the current thread local [INTERNER] with a
        /// copy of the one used to generate `T`.
        pub fn into_inner(self) -> T {
            INTERNER.with_borrow_mut(|i| *i = self.interner);
            self.inner
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cbmc_string::InternedString;

    #[test]
    fn test_string_interner() {
        let a: InternedString = "A".into();
        let b: InternedString = "B".into();
        let aa: InternedString = "A".into();

        assert_eq!(a, aa);
        assert_ne!(a, b);
        assert_ne!(aa, b);

        assert_eq!(a, "A");
        assert_eq!(b, "B");
        assert_eq!(aa, "A");
    }
}
