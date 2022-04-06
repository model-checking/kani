# Loops, unwinding, and bounds

Consider code like this (available [here](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/loops-unwinding/src/lib.rs)):

```rust,noplaypen
{{#include tutorial/loops-unwinding/src/lib.rs:code}}
```

This code has an off-by-one error that only occurs on the last iteration of the loop (when called with an input that will trigger it).
We can try to find this bug with a proof harness like this:

```rust,noplaypen
{{#include tutorial/loops-unwinding/src/lib.rs:kani}}
```

When we run Kani on this, we run into an unfortunate result: non-termination.
This non-termination is caused by CBMC trying to unwind the loop an unlimited number of times.

> **NOTE**: Presently, [due to a bug](https://github.com/model-checking/kani/issues/493), this is especially bad: we don't see any output at all.
> Kani is supposed to emit some log lines that might give some clue that an infinite loop is occurring.
> If Kani doesn't terminate, it's almost always the problem that this section covers.

To verify programs like this, we need to do two things:

1. Set an upper bound on the size of the problem.
We've actually already done part of this: our proof harness seems to be trying to set an upper limit of 10.
2. Tell Kani about this limit if it's not able to figure it out on its own.

Bounding proofs like this means we may no longer be proving as much as we originally hoped.
Who's to say, if we prove everything works up to size 10, that there isn't a novel bug lurking, expressible only with problems of size 11+?
But, let's get back to the issue at hand.

We can "make progress" in our work by giving Kani a global bound on the problem size using the `--unwind <bound>` flag.
This flag puts a fixed upper bound on loop unwinding.
Kani will automatically generate verification conditions that help us understand if that bound isn't enough.
Let's start with a small unwinding value:

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
2. We aren't seeing other failures if we only unwind the loop once.
The execution can't progress far enough to reveal the bug we're interested in (which actually only happens in the last iteration of the loop).

Doing an initial `--unwind 1` is generally enough to force termination, but often too little for verification.

We were clearly aiming at a size limit of 10 in our proof harness, so let's try a few things:

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
This is because our error is really an off-by-one problem, we loop one too many times, so let's add one more:

```
# kani src/lib.rs --unwind 12 | grep Failed
Failed Checks: index out of bounds: the length is less than or equal to the given index
Failed Checks: dereference failure: pointer outside object bounds
```

Kani is now sure we've unwound the loop enough to verify our proof harness, and now we're seeing just the bound checking failures from the off-by-one error.

1. Exercise: Fix the off-by-one bounds error and get Kani to verify successfully.
2. Exercise: After fixing the error, `--unwind 11` works. Why?
