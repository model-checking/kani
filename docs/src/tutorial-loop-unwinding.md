# Loops, unwinding, and bounds

Consider code like this (available [here](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/loops-unwinding/src/lib.rs)):

```rust
{{#include tutorial/loops-unwinding/src/lib.rs:code}}
```

This code has an off-by-one error that only occurs on the last iteration of the loop (when called with an input that will trigger it).
We can try to find this bug with a proof harness like this:

```rust
{{#include tutorial/loops-unwinding/src/lib.rs:kani}}
```

But we've just used a [new attribute](reference/attributes.md#kaniunwindnumber) (`#[kani::unwind(1)]`) that requires some explanation.
When we run `cargo kani` on this code as we have written it, we see an odd verification failure:

```
SUMMARY:
 ** 1 of 67 failed (66 undetermined)
Failed Checks: unwinding assertion loop 0

VERIFICATION:- FAILED
```

If we try removing this "unwind" annotation and re-running Kani, the result is worse: non-termination.
Kani simply doesn't produce a result.

The problem we're struggling with is the technique Kani uses to verify code.
We're not able to handle code with "unbounded" loops, and what "bounded" means can be quite subtle.
It has to have a constant number of iterations that's _"obviously constant"_ enough for the verifier to actually figure this out.
In practice, very few loops are like this.

To verify programs like this with Kani as it exists today, we need to do two things:

1. Set an upper bound on the size of the problem.
We've actually already done part of this: our proof harness above seems to be trying to set an upper `LIMIT` of 10.
2. Tell Kani about this limit if (or when) it's not able to figure it out on its own.
This is the purpose of the `kani::unwind` annotation.

Bounding proofs like this means we may no longer be proving as much as we originally hoped.
Who's to say, if we prove everything works up to size 10, that there isn't a novel bug lurking, reachable only with problems of size 11+?
Perhaps!
But, let's get back to the issue at hand.

By putting `#[kani::unwind(1)]` on the proof harness, we've placed an upper bound of 1 loop iteration.
The "unwinding assertion" failure that Kani reports is because this bound is not high enough.
The code tries to execute more than 1 loop iteration.
(And, because the unwinding isn't high enough, many of the other properties Kani is verifying become "undetermined": we don't really know if they're true or false, because we can't get far enough.)

**Exercise**: Try increasing the bound. Where might you start? How high do you need to go to get rid of the "unwinding assertion" failure?

<details>
<summary>Click to see explanation for the exercise</summary>

Since the proof harness is trying to limit the array to size 10, an initial unwind value of 10 seems like the obvious place to start.
But that's not large enough for Kani, and we still see the "unwinding assertion" failure.

At size 11, the "unwinding assertion" goes away, and now we can see the actual failure we're trying to find too.
We'll explain why we see this behavior in a moment.

</details>

Once we have increased the unwinding limit high enough, we're left with these failures:

```
SUMMARY:
 ** 1 of 68 failed
Failed Checks: index out of bounds: the length is less than or equal to the given index
 File: "./src/lib.rs", line 12, in initialize_prefix

VERIFICATION:- FAILED
```

**Exercise**: Fix the off-by-one error, and get the (bounded) proof to go through.

We now return to the question: why is 11 the unwinding bound?

Kani needs the unwinding bound to be "one more than" the number of loop iterations.
We previously had an off-by-one error that tried to do 11 iterations on an array of size 10.
So... the unwinding bound needed to be 11, then.

> **NOTE**: Presently, there are some situations where "number of iterations of a loop" can be less obvious than it seems.
> This can be easily triggered with use of `break` or `continue` within loops.
> Often this manifests itself as needing "two more" or "three more" iterations in the unwind bound than seems like it would actually run.
> In those situations, we might still need a bound like `kani::unwind(13)`, despite looking like a loop bounded to 10 iterations.

The approach we've taken here is a general method for getting a bounded proof to go through:

1. Put an actual upper bound on the problem itself.
Here that's accomplished via `LIMIT` in our proof harness.
We don't create a slice any bigger than that, and that's what we loop over.
2. Start at a reasonable guess for a `kani::unwind` bound, and increase until the unwinding assertion failure goes away.
3. Or, if that starts to take too long to verify, decrease your problem's bound, to accommodate the verifier's performance.

## Unwinding value specification

The best approach to supplying Kani with unwind bounds is using the annotation `kani::unwind`, as we show above.

You might want to supply one via command line when experimenting, however.
In that case you can either use `--default-unwind x` to set an unwind bound for every proof harness that **does not** have an explicit bound.

Or you can _override_ a harness's bound, but only when running a specific harness:

```
cargo kani --harness check_initialize_prefix --unwind 11
```

Finally, you might be interested in defaulting the unwind bound to 1, to force termination (and force supplying a bound) on all your proof harnesses.
You can do this by putting this into your `Cargo.toml` file:

```toml
[workspace.metadata.kani.flags]
default-unwind = 1
```

## Bounded proof

Before we finish, it's worth revisiting the implications of what we've done here.
Kani frequently needs to do "bounded proof", which contrasts with unbounded or full verification.

We've written a proof harness that shows `initialize_prefix` has no errors on input slices of size 10, but no higher.
The particular size we choose is usually determined by balancing the level of assurance we want, versus runtime of Kani.
It's often not worth running proofs for large numbers of iterations, unless either very high assurance is necessary, or there's reason to suspect larger problems will contain novel failure modes.

**Exercise**: Try increasing the problem size (both the unwind and the `LIMIT` constant). When does it start to take more than a few seconds?

<details>
<summary>Click to see explanation for the exercise</summary>

On your friendly neighborhood author's machine, a `LIMIT` of 100 takes about 3.8 seconds end-to-end.
This is a relatively simple bit of code, though, and it's not uncommon for some proofs to scale poorly even to 5 iterations.

</details>

One consequence of this, however, is that Kani often scales poorly to "big string problems" like parsing.
Often a parser will need to consume inputs larger than 10-20 characters to exhibit strange behaviors.

## Summary

In this section:

1. We saw Kani fail to terminate.
2. We saw how `#[kani::unwind(1)]` can help force Kani to terminate (with a verification failure).
3. We saw "unwinding assertions" verify that we've set the unwinding limit high enough.
4. We saw how to put a practical bound on problem size in our proof harness.
5. We saw how to pick an unwinding size large enough to successfully verify that bounded proof.
