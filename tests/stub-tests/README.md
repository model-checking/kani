This folder contains tests which can be used to test the compatibility of the
various abstractions against the standard library implementation.

They are extracted mostly verbatim directly from the [Rust reference manual for the
Vector](https://doc.rust-lang.org/std/vec/struct.Vec.html).

To run these tests through the compiletest framework for the kani abstraction:

```bash
$ ./x.py test -i stub-tests
```
