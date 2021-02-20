## 0.3.0

### Breaking changes

- The minimum supported Rust version has been increased to 1.50.0.

### Bug Fixes

- Certain `enum`s could not be derived before, and now can be.

- Structs with more than 10 fields can now be derived.

## 0.2.0

### Breaking changes

- Generated code now requires `proptest` 0.10.0.

## 0.1.2

### Other Notes

- Dervied enums now use `LazyTupleUnion` instead of `TupleUnion` for better
  efficiency.

## 0.1.1

This is a minor release to correct a packaging error. The licence files are now
included in the files published to crates.io.
