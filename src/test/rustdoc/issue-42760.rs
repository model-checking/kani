#![allow(rustdoc::invalid_rust_codeblocks)]

// @has issue_42760/struct.NonGen.html
// @has - '//h2' 'Example'

/// Item docs.
///
#[doc="Hello there!"]
///
/// # Example
///
/// ```rust
/// // some code here
/// ```
pub struct NonGen;
