- **Feature Name:** MIR Linker (`mir_linker`)
- **RFC Tracking Issue**: <https://github.com/model-checking/kani/issues/1588>
- **RFC PR:** <https://github.com/model-checking/kani/pull/1600>
- **Status:** Stable
- **Version:** 3

-------------------

## Summary

Fix linking issues with the rust standard library in a scalable manner by only generating goto-program for
code that is reachable from the user harnesses.

## User Impact

The main goal of this RFC is to enable Kani users to link against all supported constructs from the `std` library.
Currently, Kani will only link to items that are either generic or have an inline annotation.

The approach introduced in this RFC will have the following secondary benefits.
- Reduce spurious warnings about unsupported features for cases where the feature is not reachable from any harness.
- In the verification mode, we will likely see a reduction on the compilation time and memory consumption
 by pruning the inputs of symtab2gb and goto-instrument.
  - Compared to linking against the standard library goto-models that take more than 5 GB.
- In a potential assessment mode, only analyze code that is reachable from all public items in the target crate.

One downside is that we will include a pre-compiled version of the std, our release bundle will double in size
(See [Rational and Alternatives](0001-mir-linker.md#rational-and-alternatives)
for more information on the size overhead).
This will negatively impact the time taken to set up Kani
(triggered by either the first time a user invokes `kani | cargo-kani` , or explicit invoke the subcommand `setup`).

## User Experience

Once this RFC has been stabilized users shall use Kani in the same manner as they have been today.
Until then, we wil add an unstable option `--mir-linker` to enable the cross-crate reachability analysis
and the generation of the goto-program only when compiling the target crate.

Kani setup will likely take longer and more disk space as mentioned in the section above.
This change will not be guarded by `--mir-linker` option above.

## Detailed Design

In a nutshell, we will no longer generate a goto-program for every crate we compile.
Instead, we will generate the MIR for every crate, and we will generate only one goto-program.
This model will only include items reachable from the target crate's harnesses.

The current system flow for a crate verification is the following (Kani here represents either `kani | cargo-kani`
executable):

1. Kani compiles the user crate as well as all its dependencies.
   For every crate compiled, `kani-compiler` will generate a goto-program.
   This model includes everything reachable from the crate's public functions.
2. After that, Kani links all models together by invoking `goto-cc`.
   This step will also link against Kani's `C` library.
3. For every harness, Kani invokes `goto-instrument` to prune the linked model to only include items reachable from the given harness.
4. Finally, Kani instruments and verify each harness model via `goto-instrument` and `cbmc` calls.

After this RFC, the system flow would be slightly different:

1. Kani compiles the user crate dependencies up to the MIR translation.
   I.e., for every crate compiled, `kani-compiler` will generate an artifact that includes the MIR representation
  of all items in the crate.
2. Kani will generate the goto-program only while compiling the target user crate.
  It will generate one goto-program that includes all items reachable from any harness in the target crate.
3. `goto-cc` will still be invoked to link the generated model against Kani's `C` library.
4. Steps #3 and #4 above will be performed without any change.

This feature will require three main changes to Kani which are detailed in the sub-sections below.

### Kani's Sysroot

Kani currently uses `rustup` sysroot to gather information from the standard library constructs.
The artifacts from this `sysroot` include the MIR for generic items as well as for items that may be included in
a crate compilation (e.g.: functions marked with `#[inline]` annotation).
The artifacts do not include the MIR for items that have already been compiled to the `std` shared library.
This leaves a gap that cannot be filled by the `kani-compiler`;
thus, we are unable to translate these items into goto-program.

In order to fulfill this gap, we must compile the standard library from scratch.
This RFC proposes a similar method to what [`MIRI`](https://github.com/rust-lang/miri) implements.
We will generate our own sysroot using the `-Z always-encode-mir` compilation flag.
This sysroot will be pre-compiled and included in our release bundle.

We will compile `kani`'s libraries (`kani` and `std`) also with `-Z always-encode-mir`
and with the new sysroot.


### Cross-Crate Reachability Analysis

`kani-compiler` will include a new `reachability` module to traverse over the local and external MIR items.
This module will `monomorphize` all generic code as it's performing the traversal.

The traversal logic will be customizable allowing different starting points to be used.
The two options to be included in this RFC is starting from all local harnesses
(tagged with `#[kani::proof]`) and all public functions in the local crate.

The `kani-compiler` behavior will be customizable via a new flag:

  ```
  --reachability=[ harnesses | pub_fns |  none | legacy | tests ]
  ```

where:

 - `harnesses`: Use the local harnesses as the starting points for the reachability analysis.
 - `pub_fns`: Use the public local functions as the starting points for the reachability analysis.
 - `none`: This will be the default value if `--reachability` flag is not provided. It will skip
 reachability analysis. No goto-program will be generated.
  This will be used to compile dependencies up to the MIR level.
   `kani-compiler` will still generate artifacts with the crate's MIR.
 - `tests`: Use the functions marked as tests with `#[tests]` as the starting points for the analysis.
 - `legacy`: Mimics `rustc` behavior by invoking
   `rustc_monomorphizer::collect_and_partition_mono_items()` to collect the items to be generated.
   This will not include many items that go beyond the crate boundary.
   *This option was only kept for now for internal usage in some of our compiler tests.*
   *It cannot be used as part of the end to end verification flow, and it will be removed in the future.*

These flags will not be exposed to the final user.
They will only be used for the communication between `kani-driver` and `kani-compiler`.

### Dependencies vs Target Crate Compilation

The flags described in the section above will be used by `kani-driver` to implement the new system flow.
For that, we propose the following mechanism:

- For standalone `kani`, we will pass the option `--reachability=harnesses` to `kani-compiler`.
- For `cargo-kani`, we will replace
  ```
  cargo build <FLAGS>
  ```

  with:

  ```
  cargo rustc <FLAGS> -- --reachability=harnesses
  ```

  to build everything.
  This command will compile all dependencies without the `--reachability` argument, and it will only pass `harnesses`
  value to the compiler when compiling the target crate.

## Rational and Alternatives

Not doing anything is not an alternative, since this fixes a major gap in Kani's usability.

### Benefits

- The MIR linker will allow us to fix the linking issues with Rust's standard library.
- Once stabilized, the MIR linker will be transparent to the user.
- It will enable more powerful and precise static analysis to `kani-compiler`.
- It won't require any changes to our dependencies.
- This will fix the harnesses' dependency on the`#[no_mangle]` annotation
([Issue-661](https://github.com/model-checking/kani/issues/661)).

### Risks

Failures in the linking stage would not impact the tool soundness. I anticipate the following failure scenarios:
- ICE (Internal compiler error): Some logic is incorrectly implemented and the linking stage crashes.
  Although this is a bad experience for the user, this will not impact the verification result.
- Missing items: This would either result in ICE during code generation or a verification failure if the missing
  item is reachable.
- Extra items: This shouldn't impact the verification results, and they should be pruned by CBMC's reachability
  analysis.
  This is already the case today. In extreme cases, this could include a symbol that we cannot compile and cause an ICE.

The new reachability code would be highly dependent on the `rustc` unstable APIs, which could increase
the cost of the upstream synchronization.
That said, the APIs that would be required are already used today.

Finally, this implementation relies on a few unstable options from `cargo` and `rustc`.
These APIs are used by other tools such as MIRI, so we don't see a high risk that they would be removed.

### Alternatives

The other options explored were:
1. Pre-compile the standard library, and the kani library, and ship the generated `*symtab.json` files.
2. Pre-compile the standard library, and the kani library, convert the standard library and dependencies to goto-program
  (via`symtab2gb`) and link them into one single goto-program.
  Ship the generated model.

Both would still require shipping the compiler metadata (via `rlib` or `rmeta`) for the kani library, its
dependencies, and `kani_macro.so`.

Both alternatives are very similar. They only differ on the artifact that would be shipped.
They require generating and shipping a custom `sysroot`;
however, there is no need to implement the reachability algorithm.

We implemented a prototype for the MIR linker and one for the alternatives.
Both prototypes generate the sysroot as part of the `cargo kani` flow.

We performed a small experiment (on a `c5.4xlarge` ec2 instance running Ubuntu 20.04) to assess the options.

For this experiment, we used the following harness:
```rust
#[kani::proof]
#[kani::unwind(4)]
pub fn check_format() {
    assert!("2".parse::<u32>().unwrap() == 2);
}
```
The experiment showed that the MIR linker approach is much more efficient.

See the table bellow for the breakdown of time (in seconds) taken for each major step of
the harness verification:


| Stage                     | MIR Linker | Alternative 1 |
----------------------------|------------|-------------|
| compilation               |   22.2s    |    64.7s    |
| goto-program generation |   2.4s     |    90.7s    |
| goto-program linking    |   0.8s     |    33.2s    |
| code instrumentation      |   0.8s     |    33.1     |
| verification              |   0.5s     |    8.5s     |

It is possible that `goto-cc` time can be improved, but this would also require further experimentation and
expertise that we don't have today.

Every option would require a custom sysroot to either be built or shipped with Kani.
The table below shows the size of the sysroot files for the alternative #2
(goto-program files) vs compiler artifacts (`*.rmeta` files)
files with `-Z always-encode-mir` for `x86_64-unknown-linux-gnu` (on Ubuntu 18.04).

| File Type      | Raw size | Compressed size |
|----------------|----------|-----------------|
| `symtab.json`  |   950M   |     26M         |
| `symtab.out`   |   84M    |     24M         |
| `*.rmeta`      |   92M    |     25M         |

These results were obtained by looking at the artifacts generated during the same experiment.

## Open questions

- ~~Should we build or download the sysroot during `kani setup`?~~
  We include pre-built MIR artifacts for the `std` library.
- ~~What's the best way to enable support to run Kani in the entire `workspace`?~~
  We decided to run `cargo rustc` per package.
- ~~Should we codegen all static items no matter what?~~
 We only generate code for static items that are collected by the reachability analysis.
 Static objects can only be initialized via constant function.
 Thus, it shouldn't have any side effect.
- ~~What's the best way to handle `cargo kani --tests`?~~
  We are going to use the test profile and iterate over all the targets available in the crate:
  - `cargo rustc --profile test -- --reachability=harnesses`


## Future possibilities

- Split the goto-program into two or more items to optimize compilation result caching.
  - Dependencies: One model will include items from all the crate dependencies.
    This model will likely be more stable and require fewer updates.
  - Target crate: The model for all items in the target crate.
- Do the analysis per-harness. This might be adequate once we have a mechanism to cache translations.
- Add an option to include external functions to the analysis starting point in order to enable verification when
calls are made from `C` to `rust`.
- Contribute the reachability analysis code back to upstream.
