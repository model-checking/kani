# RMC on a single file

For small examples, or initial learning, it's very common to run RMC on just one source file.
The command line format for invoking RMC directly has a few common formats:

```
rmc filename.rs
# or
rmc filename.rs [--rmc-flags]
# or
rmc filename.rs [--rmc-flags] --cbmc-args [--cbmc-flags]
```

For example,

```
rmc filenames.rs --visualize --cbmc-args --object-bits 11 --unwind 15
```

## Common RMC arguments

**`--visualize`** will generate a report in the local directory accessible through `report/html/index.html`.
This report will shows coverage information, as well as give traces for each failure RMC finds.

**`--function <name>`** RMC defaults to assuming the starting function is called `main`.
You can change it to a different function with this argument.
Note that to "find" the function given, it needs to be given the `#[no_mangle]` annotation.

**`--gen-c`** will generate a C file that roughly corresponds to the input Rust file.
This can sometimes be helpful when trying to debug a problem with RMC.

**`--keep-temps`** will preserve generated files that RMC generates.
In particular, this will include a `.json` file which is the "CBMC symbol table".
This can be helpful in trying to diagnose bugs in RMC, and may sometimes be requested in RMC bug reports.

## Common CBMC arguments

RMC invokes CBMC to do the underlying solving.
(CBMC is the "C Bounded Model Checker" but is actually a framework that supports model checking multiple languages.)
CBMC arguments are sometimes necessary to get good results.

To give arguments to CBMC, you pass `--cbmc-args` to RMC.
This "switches modes" from RMC arguments to CBMC arguments.
Everything else given on the command line will be assumed to be a CBMC argument, and so all RMC arguments should be provided before this flag.

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

