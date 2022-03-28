# Loops, unwinding, and bounds

Consider code like this:

```rust,noplaypen
{{#include tutorial/loops-unwinding/src/lib.rs:code}}
```

This code has an off-by-one error that only occurs on the last iteration of the loop (when called with an input that will trigger it).
We can try to find this bug with a proof harness like this:

```rust,noplaypen
{{#include tutorial/loops-unwinding/src/lib.rs:kani}}
```

When we run Kani on this, we run into an unfortunate result: non-termination.
This non-termination is caused by the model checker trying to unroll the loop an unbounded number of times.

> **NOTE:** Presently, [due to a bug](https://github.com/model-checking/kani/issues/493), this is especially bad: we don't see any output at all.
> You are supposed to see some log lines that might give some clue that an infinite loop is occurring.
> If Kani doesn't terminate, it's almost always the problem that this section is covering, however.

To verify programs like this, we really need to do two things:

1. Create an upper bound on the size of the problem.
We've actually already done part of this: our proof harness seems to be trying to set an upper limit of 10.
2. Tell Kani about this limit, if it's not able to figure it out on its own.

> **NOTE:** In the future, Kani may eventually support specifying _loop invariants_, which allow us to do away with fixed upper bounds like this.
> That support is not ready yet, however.

Bounding proofs like this means we may no longer be proving as much as we originally hoped.
Who's to say, if we prove everything works up to size 10, that there isn't a novel bug lurking, expressible only with problems of size 11+?
But, let's get back to the practical issue at hand.

We can "make progress" in our work by giving Kani a global bound on the problem size using the `--unwind <bound>` flag.
This flag puts a fixed upper bound on loop unrolling.
Kani will automatically generate verification conditions that help us understand if that bound isn't enough.
Let's start with the "sledge hammer" by dropping all the way down to size 1:

```
# kani src/lib.rs --unwind 1
Check 69: .unwind.0
         - Status: FAILURE
         - Description: "unwinding assertion loop 0"
[...]
VERIFICATION:- FAILED
```

This output is showing us two things:

1. Kani tells us we haven't unwound enough. This is the failure of the "unwinding assertion."
2. We aren't seeing other failures if we only unroll the loop once.
The execution can't progress far enough to reveal the bug we're interested in (which actually only happens in the last iteration of the loop).

Doing an initial `--unwind 1` is generally enough to force termination, but often too little to do any practical verification.

We were clearly aiming at a size limit of 10 in our proof harness, so let's try a few things here:

```
# kani src/lib.rs --unwind 10 | grep Failed
Failed Checks: unwinding assertion loop 0
```

A bound of 10 still isn't enough because we generally need to unwind one greater than the number of executed loop iterations:

```
# kani src/lib.rs --unwind 11 | grep Failed
Failed Checks: index out of bounds: the length is less than or equal to the given index
Failed Checks: dereference failure: pointer outside object bounds
Failed Checks: unwinding assertion loop 0
```

We're still not seeing the unwinding assertion failure go away!
This is because our error is really an off by one problem, we loop one too many times, so let's add one more:

```
# kani src/lib.rs --unwind 12 | grep Failed
Failed Checks: index out of bounds: the length is less than or equal to the given index
Failed Checks: dereference failure: pointer outside object bounds
```

Kani is now sure we've unwound the loop enough to verify our proof harness, and now we're seeing just the bound checking failures from the off by one error.

1. Exercise: Fix the off-by-one bounds error and get Kani to verify successfully.
2. Exercise: After fixing the error, `--unwind 11` works. Why?

## Customizing individual loop bounds

Setting `--unwind` globally affects every loop.
Once you know which loop is the culprit, it can sometimes be helpful to provide specific bounds on specific loops.

In the general case, specifying just the highest bound globally for all loops shouldn't cause any problems, except that the solver may take more time because _all_ loops will be unwound to the specified bound.

1. Exercise: Try increasing the unwind bound on the code from the previous section and then time how long solving takes.
For example, we see 0.5s at unwinding 12, and 3s at unwinding 100.

> **NOTE:** Kani does not yet support annotating code with unwinding bounds.
> What follows is a hacky way to make things happen, if you need it.

In situations where you need to optimize solving time better, specific bounds for specific loops can be provided on the command line.

```
# kani src/lib.rs --output-format old --cbmc-args --show-loops
[...]
Loop _RNvCs6JP7pnlEvdt_3lib17initialize_prefix.0:
  file ./src/lib.rs line 11 column 5 function initialize_prefix

Loop _RNvMs8_NtNtCswN0xKFrR8r_4core3ops5rangeINtB5_14RangeInclusivejE8is_emptyCs6JP7pnlEvdt_3lib.0:
  file $RUST/library/core/src/ops/range.rs line 540 column 9 function std::ops::RangeInclusive::<Idx>::is_empty

Loop gen-repeat<[u8; 10]::16806744624734428132>.0:
```

> **NOTE:** `--show-loops` is a flag to the underlying model checker, CBMC, and so it needs to appear after `--cbmc-args`.
> This flag `--cbmc-args` "switches modes" in the command line from Kani flags to CBMC flags, so we place all Kani flags and arguments before it.
> Also, the `--output-format old` flag turns off the post-processing of output from CBMC, which is needed here because with `--show-loops`,
> CBMC is not running the actual verification step.

This command shows us the mangled names of the loops involved.
Then we can specify the bound for specific loops by name, from the command line:

```
kani src/lib.rs --cbmc-args --unwindset _RNvCs6JP7pnlEvdt_3lib17initialize_prefix.0:12
```

The general format of the `--unwindset` option is: `label_1:bound_1,label_2:bound_2,...`.
The label is revealed by the output of `--show-loops` as we saw above.

## Summary

In this section:

1. We saw Kani fail to terminate.
2. We saw how `--unwind 1` can "sledgehammer" Kani into terminating, possibly with additional and/or missing failures.
3. We saw how "unwinding assertions" can warn us that we've set the unwinding limit too low.
4. We saw how to put a practical bound on problem size in our proof harness.
5. We saw how to pick an unwinding size large enough to successfully verify that bounded proof.
