// compile-flags: --edition 2021
#![allow(unused)]
#![allow(unused_imports)]
use std::path::{self, Path, PathBuf};  // good: std is a crate name
use crate::foo::baz::foobaz;    // good: foo is at the root of the crate

mod foo {

    pub mod example {
        pub mod iter {}
    }

    use crate::foo::example::iter; // good: foo is at crate root
//  use example::iter;      // bad in 2015 edition: relative paths are not allowed without `self`; good in 2018 edition
    use self::baz::foobaz;  // good: self refers to module 'foo'
    use crate::foo::bar::foobar;   // good: foo is at crate root

    pub mod bar {
        pub fn foobar() { }
    }

    pub mod baz {
        use super::bar::foobar; // good: super refers to module 'foo'
        pub fn foobaz() { }
    }
}

fn main() {}