//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::char`.

use std::char::*;
use std::iter::once;
use std::ops::Range;

use strategy::*;
use strategy::statics::static_map;
use arbitrary::*;

#[cfg(all(feature = "alloc", not(feature="std")))]
use alloc::vec::Vec;
#[allow(unused_imports)]
#[cfg(feature = "std")]
use std::vec::Vec;

macro_rules! impl_wrap_char {
    ($type: ty, $mapper: expr) => {
        arbitrary!($type, SMapped<char, Self>;
            static_map(any::<char>(), $mapper));
    };
}

impl_wrap_char!(EscapeDebug, char::escape_debug);
impl_wrap_char!(EscapeDefault, char::escape_default);
impl_wrap_char!(EscapeUnicode, char::escape_unicode);
#[cfg(feature = "unstable")]
impl_wrap_char!(ToLowercase, char::to_lowercase);
#[cfg(feature = "unstable")]
impl_wrap_char!(ToUppercase, char::to_uppercase);

#[cfg(feature = "unstable")]
use collection::vec;

#[cfg(feature = "unstable")]
const VEC_MAX: usize = ::std::u16::MAX as usize;

#[cfg(feature = "unstable")]
arbitrary!(DecodeUtf8<<Vec<u8> as IntoIterator>::IntoIter>,
    SMapped<Vec<u8>, Self>;
    static_map(vec(any::<u8>(), ..VEC_MAX), decode_utf8)
);

#[cfg(feature = "unstable")]
arbitrary!(DecodeUtf16<<Vec<u16> as IntoIterator>::IntoIter>,
    SMapped<Vec<u16>, Self>;
    static_map(vec(any::<u16>(), ..VEC_MAX), decode_utf16)
);

arbitrary!(ParseCharError, IndFlatten<Mapped<bool, Just<Self>>>;
    any::<bool>().prop_ind_flat_map(|is_two|
        Just((if is_two { "__" } else { "" }).parse::<char>().unwrap_err()))
);

#[cfg(feature = "unstable")]
arbitrary!(CharTryFromError; {
    use std::convert::TryFrom;
    char::try_from(0xD800 as u32).unwrap_err()
});

arbitrary!(DecodeUtf16Error, SFnPtrMap<Range<u16>, Self>;
    static_map(0xD800..0xE000, |x|
        decode_utf16(once(x)).next().unwrap().unwrap_err())
);

#[cfg(test)]
mod test {
    no_panic_test!(
        escape_debug => EscapeDebug,
        escape_default => EscapeDefault,
        escape_unicode => EscapeUnicode,
        parse_char_error => ParseCharError,
        decode_utf16_error => DecodeUtf16Error
    );

    #[cfg(feature = "unstable")]
    no_panic_test!(
        to_lowercase => ToLowercase,
        to_uppercase => ToUppercase,
        decode_utf16 => DecodeUtf16<<Vec<u16> as IntoIterator>::IntoIter>,
        decode_utf8 => DecodeUtf8<<Vec<u8> as IntoIterator>::IntoIter>,
        char_try_from_error => CharTryFromError
    );
}
