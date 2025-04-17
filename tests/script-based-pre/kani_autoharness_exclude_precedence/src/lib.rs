// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod foo {
    fn foo_function() {}

    mod bar {
        fn bar_function() {}

        fn foo_bar_function() {}
    }

    mod baz {
        fn foo_baz_function() {}
    }
}

mod other {
    fn regular_function() {}

    fn with_bar_name() {}
}

fn foo_top_level() {}

fn bar_top_level() {}
