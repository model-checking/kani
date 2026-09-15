// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(dead_code)]

pub fn consume_escape_unicode(value: std::char::EscapeUnicode) -> usize {
    value.count()
}
