# Usage on a single file

For small examples or initial learning, it's very common to run Kani on just one source file.

The command line format for invoking Kani directly is the following:

```
kani filename.rs [<kani-args>]*
```

For example,

```
kani example.rs
```

runs Kani on all the proof harnesses from file `example.rs`.
A proof harness is simply a function with the `#[kani::proof]` annotation.

## Common arguments

The most common `kani` arguments are the following:

 * `--harness <name>`: By default, Kani checks all proof harnesses it finds. You
   can switch to checking a single harness using this flag.

 * `--unwind <n>`: Set a global upper [loop
   unwinding](./tutorial-loop-unwinding.md) bound on all loops. This can force
   termination when CBMC tries to unwind loops indefinitely.

 * `output-format <regular|terse|old>`: By default (`regular`), Kani
   post-processes CBMC's output to produce more comprehensible results. In
   contrast, `terse` outputs only a summary of these results, and `old` forces
   Kani to emit the original output from CBMC.

 * `--visualize`: Generates an HTML report in the local directory accessible
   through `report/html/index.html`. This report shows coverage information and
   provides traces (i.e., counterexamples) for each failure found by Kani.

Run `kani --help` to see a complete list of arguments.
