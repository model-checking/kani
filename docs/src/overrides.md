## Overrides

As explained in [Comparison with other
tools](./tool-comparison.md#comparison-with-other-tools), Kani is based on a
technique called model checking, which verifies a program without actually
executing it. It does so through encoding the program and analyzing the encoded
version. The encoding process often requires "modeling" some of the library
functions to make them suitable for analysis. Typical examples of functionality
that requires modeling are system calls and I/O operations. In some cases, Kani
performs such encoding through overriding some of the definitions in the Rust
standard library. The following table lists some of the symbols that Kani
overrides and a description of their behavior compared to the `std` versions:

Name | Description |
---  | --- |
`print`, `eprint`, `println`, and `eprintln` macros | Kani's version of these macros skips functionality that is not relevant for verification, e.g., string formatting and I/O operations |
`assert`, `assert_eq`, and `assert_ne` macros | Kani overrides these macros in order to skip string formatting code, control the message that is displayed in the verification report for those asserts, and perform some instrumentation. |
