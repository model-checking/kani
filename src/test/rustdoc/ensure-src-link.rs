#![crate_name = "foo"]

// This test ensures that the [src] link is present on traits items.

// @has foo/trait.Iterator.html '//div[@id="method.zip"]//a[@class="srclink"]' "source"
pub use std::iter::Iterator;
