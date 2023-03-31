# Changelog

This file contains notable changes (e.g. breaking changes, major changes, etc.) in Kani releases.

This file was introduced starting Kani 0.23.0, so it only contains changes from version 0.23.0 onwards.

## [0.23.0]

### Breaking Changes

- Remove the second parameter in the `kani::any_where` function by @zhassan-aws in #2257  
We removed the second parameter in the `kani::any_where` function (`_msg: &'static str`) to make the function more ergonomic to use.
We suggest moving the explanation for why the assumption is introduced into a comment.
For example, the following code:
```rust
    let len: usize = kani::any_where(|x| *x < 5, "Restrict the length to a value less than 5");
```
should be replaced by:
```rust
    // Restrict the length to a value less than 5
    let len: usize = kani::any_where(|x| *x < 5);
```

### Major Changes

- Enable the build cache to avoid recompiling crates that haven't changed, and introduce `--force-build` option to compile all crates from scratch by @celinval in #2232. 
