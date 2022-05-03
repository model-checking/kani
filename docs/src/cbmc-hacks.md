# Working with CBMC

This section describes how to access more advanced CBMC options from Kani.

## CBMC arguments

Kani is able to handle common CBMC arguments as if they were its own (e.g.,
`--default-unwind <n>`), but sometimes it may be necessary to use CBMC arguments which
are not handled by Kani.

To pass additional arguments for CBMC, you pass `--cbmc-args` to Kani. Note that
this "switches modes" from Kani arguments to CBMC arguments: Any arguments that
appear after `--cbmc-args` are considered to be CBMC arguments, so all Kani
arguments must be placed before it.

Thus, the command line format to invoke `cargo kani` with CBMC arguments is:

```bash
cargo kani [<kani-args>]* --cbmc-args [<cbmc-args>]*
```

> **NOTE**: In cases where CBMC is not expected to emit a verification output,
> you have to use Kani's argument `--output-format old` to turn off the
> post-processing of output from CBMC.

### Individual loop bounds

Setting `--default-unwind <n>` affects every loop in a harness.
Once you know a particular loop is causing trouble, sometimes it can be helpful to provide a specific bound for it.

In the general case, specifying just the highest bound globally for all loops
shouldn't cause any problems, except that the solver may take more time because
_all_ loops will be unwound to the specified bound.

In situations where you need to optimize for the solver, individual bounds for
each loop can be provided on the command line. To do so, we first need to know
the labels assigned to each loop with the CBMC argument `--show-loops`:

```
# kani src/lib.rs --output-format old --cbmc-args --show-loops
[...]
Loop _RNvCs6JP7pnlEvdt_3lib17initialize_prefix.0:
  file ./src/lib.rs line 11 column 5 function initialize_prefix

Loop _RNvMs8_NtNtCswN0xKFrR8r_4core3ops5rangeINtB5_14RangeInclusivejE8is_emptyCs6JP7pnlEvdt_3lib.0:
  file $RUST/library/core/src/ops/range.rs line 540 column 9 function std::ops::RangeInclusive::<Idx>::is_empty

Loop gen-repeat<[u8; 10]::16806744624734428132>.0:
```

This command shows us the labels of the loops involved. Note that, as mentioned
in [CBMC arguments](#cbmc-arguments), we need to use `--output-format old` to
avoid post-processing the output from CBMC.

> **NOTE**: At the moment, these labels are constructed using the mangled name
> of the function and an index. Mangled names are likely to change across
> different versions, so this method is highly unstable.

Then, we can use the CBMC argument `--unwindset
label_1:bound_1,label_2:bound_2,...` to specify an individual bound for each
loop as follows:

```bash
kani src/lib.rs --cbmc-args --unwindset _RNvCs6JP7pnlEvdt_3lib17initialize_prefix.0:12
```
