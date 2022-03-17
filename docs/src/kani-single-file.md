# Kani on a single file

For small examples, or initial learning, it's very common to run Kani on just one source file.
The command line format for invoking Kani directly has a few common formats:

```
kani filename.rs
# or
kani filename.rs [--kani-flags]
# or
kani filename.rs [--kani-flags] --cbmc-args [--cbmc-flags]
```

For example,

```
kani filenames.rs --visualize --cbmc-args --object-bits 11 --unwind 15
```

## Common Kani arguments

**`--visualize`** will generate a report in the local directory accessible through `report/html/index.html`.
This report will shows coverage information, as well as give traces for each failure Kani finds.

**`--harness <name>`** Kani defaults to running all found proof harnesses.
You can switch to running just one using this flag.
Proof harnesses are functions that have been given the `#[kani::proof]` annotation.

**`--gen-c`** will generate a C file that roughly corresponds to the input Rust file.
This can sometimes be helpful when trying to debug a problem with Kani.

**`--keep-temps`** will preserve generated files that Kani generates.
In particular, this will include a `.json` file which is the "CBMC symbol table".
This can be helpful in trying to diagnose bugs in Kani, and may sometimes be requested in Kani bug reports.

## Common CBMC arguments

Kani invokes CBMC to do the underlying solving.
(CBMC is the "C Bounded Model Checker" but is actually a framework that supports model checking multiple languages.)
CBMC arguments are sometimes necessary to get good results.

To give arguments to CBMC, you pass `--cbmc-args` to Kani.
This "switches modes" from Kani arguments to CBMC arguments.
Everything else given on the command line will be assumed to be a CBMC argument, and so all Kani arguments should be provided before this flag.

**`--unwind <n>`** Give a global upper bound on all loops.
This can force termination when CBMC tries to unwind loops indefinitely.

**`--object-bits <n>`** CBMC, by default, assumes there are only going to be 256 objects allocated on the heap in a single trace.
This corresponds to a default of `--object-bits 8`.
Rust programs often will use more than this, and so need to raise this limit.
However, very large traces with many allocations often prove intractable to solve.
If you run into this issue, a good first start is to raise the limit to 2048, i.e. `--object-bits 11`.

**`--unwindset label_1:bound_1,label_2:bound_2,...`** Give specific unwinding bounds on specific loops.
The labels for each loop can be discovered by running with the following CBMC flag:

**`--show-loops`** Print the labels of each loop in the program.
Useful for `--unwindset`.
