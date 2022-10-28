- **Feature Name:** Resource limits (e.g. timeouts) for proof harnesses
- **Feature Request Issue:** https://github.com/model-checking/kani/issues/1687
- **RFC PR:** TBD
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** N/A

## Summary

The RFC describes the introduction of resource limits for proof harnesses in Kani. This includes:

1. Timeouts
2. Memory limits

## User Impact

Timeouts are an often requested feature, with many reasons given for the request:

1. Non-termination in any form is generally a bad user experience.
2. Unwinding failures manifest as non-termination, and timeouts during the unwinding process would allow us to report the problem as an unwinding failure and give suggestions.
3. Putting Kani in CI is often difficult if a failure may simply result in a CI process continuing out of control.
4. Lack of timeouts encourages "abuse" of low unwinding values, to try to "force" termination early.
5. (For our own purposes) We'd like to be able to create a "suite" of problems that are "too hard" for Kani to presently solve, and observe improvements over time.
6. (Future) Parallel runners will be able to prioritize longer-running harnesses, so parallelism can be kept high throughout the verification process.

Memory limits have not yet been requested, but are an essential feature to accomodate a parallel harness runner.
Without memory limits, it would be easy for too many solvers in parallel to run a machine out of memory.
Past tools for CBMC have required special annotations for "hard" harnesses to run serially.

Further, to realize these benefits, we **must** implement resource limits by default.
Consequently, this will likely be a "breaking change" for many customers.
Either we should get feedback on whether this would be welcome, or we should consider a staged strategy for rolling out the feature (considered later in this doc).

## User Experience

The aim of this RFC is to introduce two new annotations to be placed along side proof harnesses.
For example, from our unwinding tutorial:

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(1)] // deliberately too low
fn check_initialize_prefix()
```

This shows our abuse of `unwind` to force termination.
We would prefer this to be unnecessary, and for the user to be introduced to unwinding via a default timeout.

In the case where we do want to give higher limits, users should have to write something like:

```rust
#[cfg(kani)]
#[kani::proof]
#[kani::timeout(60)]
#[kani::memory(10)]
fn check_initialize_prefix()
```

> **Question 1: Should we consider alternative approach to specifying these limits?**
>
> I argue yes: We're now proliferating the number of attributes, so perhaps it's time to consider supporting:
> `#[kani::proof(unwind = 5, timeout = 60, memory = 10)]`
>
> As we'll see later, the disadvantage of default limit choices is the friction of adding limit annotations to harnesses.
> Making this as easy as possible helps mitigate those disadvantages.

> **Question 2: Should we include units in the attributes?**
>
> I'm presently giving "seconds" and "gigabytes" and these seem totally reasonable units that we probably wouldn't ever deviate from.
> However, it might be nice to either require units (to be explicit) or include units (if we think people might need them... perhaps just minutes).
> I believe it would be reasonable to omit units however, and I'm not sure how difficult it would be to support `timeout = 2min` or `timeout = 4:00`.

Beyond specifying limits on each proof harness, users would interact with these limits in two other ways:

1. The output from running Kani.
2. The command line arguments for manipulating limits.

> **Question 3: How much flexibility should be provide on the command-line?**
>
> I believe we can keep this small.
> I propose we only add a few new options, and I'm not sure if we should even go that far:
> 1. `--default-timeout`
> 2. `--default-memory`
> 3. `--timeout-multiplier=X`
> 4. `--ignore-limits`
>
> The purpose of the first two is not really for the command line but instead `Cargo.toml` (which currently is just an alternative way to supply command line arguments).
>
> The purpose of the multiplier is to allow a slow/inconsistent CI system to have an extra "grace period" for timeouts.
> This way, CI can run `cargo kani --timeout-multiplier=1.3` to get 30% extra slack on timeouts.
>
> The `--ignore-limits` options should accomplish two things:
> 1. Replicate current behavior.
> 2. Allow Kani to measure memory usage and CPU time, and report suggested limits.

For output, we have a number of differences from current Kani:

1. When timeout triggers during unwinding, we can report `unwind` as a suggestion.
2. When timeout triggers during solving, we can suggest raising the `timeout` or reducing the problem size.
3. When memory limits get hit, we can suggest raising the limit. (Or: reporting a Kani bug perhaps?)
4. If verification completes during `--timeout-multiplier` "grace period" we can emit a warning.
5. If limits were exceeded but with `--ignore-limits`, we can report that actual CPU/memory usage.

> **Question 4: Should we emit a warning when verification time comes close to (but under) the timeout?**
> Ditto memory usage. I'm not sure.

Our tutorial and documentation will need updating as a result of integrating this feature.
Parts of the tutorial can be improved (such as the unwinding section) as we no longer need `unwind(1)` to force termination.
We might wish to write specific documentation to link to from (e.g.) the error message we emit when a limit was exceeded.

## Detailed Design

This will impact:

1. Two new attributes in the `kani` library: `kani::timeout(X)` and `kani::memory(X)`.
Also `kani::proof` if we decide to add convenient annotations there.
2. `kani-compiler` and `kani_metadata` will be updated to record this metadata.
3. `kani-driver` needs to updated to apply these limits.
4. New command-line arguments, to affect the per-harness limits in some (limited!) ways.

Most of these impacts are straight-forward, but a few merit thought.

### Question 5: How do we measure resource usage?

I propose these limits only affect the `cbmc` process, which means measuring only what that process does.
This means we do not count time spent in (e.g.) `goto-instrument`.

We can apply memory limits by mimicking `ulimit -v` using [`pre_exec`](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec) and the [`rlimit`](https://crates.io/crates/rlimit) or just `libc` directly, depending.
This should work fine on Linux and Mac, which are our only currently supported platforms.

The exact means of measuring time in CBMC is TBD pending some testing, but mimicing `ulimit -t` is an option.

Finally, we can measure actual resource usage by the process using [`wait4`](https://crates.io/crates/wait4).

### Question 6: What about other memory usage problems?

While build times may be long and annoying, they shouldn't be unbounded.
However, we might run into exceeding available machine memory when running `goto-instrument` to specialize goto binaries to a particular harness.

For the moment, I propose we ignore this problem.
Long term, it should be possible to optimize memory usage within these processes, they should not need many gigabytes of memory.
I think time is better spent fixing the fundamental issue there than trying to work around it.

## Rationale and alternatives

### Quesiton 7: Do limits need to be applied by default?

Yes.

If we did not apply limits by default, we will lose many of the benefits of having limits.

1. Users may encounter non-termination.
2. Harnesses that go from success to failure could take unbounded time, causing CI problems.
3. Users starting out will not benefit from timeouts when dealing with unwind failures.

### Quation 8: What choice should we make for the default memory limit?

There are many options here.
Let me first start with some data.

* Github Actions gives runners with 1:3.5 core:GB ratio (with mac runners getting more RAM).
* AWS `c` type instances give 1:2 core:GB. (with `m` 1:4 and `r` 1:8)
* "Typical" machines for quite awhile were 4 core 8 GB memory. Lately there is more variety but 8 core 16 GB seems common, which is also a 1:2.
* In the Kani test suite, most `cbmc` processes used about 1 GB or less. A small number hit 3-4 GB.
* CBMC often keeps "virtual memory" and "resident set size" in pretty close alignment.

The primary goal of a memory limit is making a parallel harness runner reliable.
Enforcing the memory limits means we can plan on parallel processes not exceeding available machine memory.

The advantages of a lower default memory limit are:

1. We could still achieve a high level of parallelism even in the presence of higher-limit harnesses.
(Workaround: Explicitly setting a lower limit than default would allow this too!)

The advantages of a higher default memory limit are:

1. Fewer harnesses will need an annotation, allowing more users to be unbothered by the limit.

Given these trade-offs, and the data on common CI machine sizes, I think a default memory limit of **2 GB** seems best.
This should be the largest value that would still allow maximum parallelism on common machine types.
We may want to collect data on real-world impact to customer harnesses.

### Question 9: What choice should we make for the default timeout?

Here the data we would like to examine is less about machines and more about common proofs.

The advantages of a higher default timeout are:

1. Fewer harnesses would be impacted by needing an explicit limit.

The advantages of a lower default timeout are:

1. Customers experiencing non-termination will get faster feedback from our tools.
2. Solving "plans" will be able to more accurately forecast how long each harness will take to solve.
3. Parallel runners can prioritize long-running harness, to ensure high CPU utilizaiton throughout verification.

We may need more data to make a final decision here, but I'd suggest we bias towards small: **15 seconds** default timeout.

### Question 10: Do we need more than per-harness timeouts?

No.

We could decide to add a over-all timeout (encompassing build and link times), but I argue this is not helpful.
The timeout we really care about (where nontermination exists and affects users) is for running `cbmc` on harnesses.

## Roll-out plan for this breaking change

Application of default limits will be a breaking change.
I propose the following approach:

1. Introduce limits and `--ignore-limits` but make this option do nothing at first: it is always applied.
This replicates the current behavior with respect to enforcing limits, but will cause warnings to be issued with suggested limit settings.
2. Keep this as the default for 2 kani releases (4 weeks).
Perhaps longer if issues arise.
3. Flip the default to enforcing.
This will finally cause the breaking change, but will have had some warning, and we retain an option to revert to the old behavior.

## Open questions

TBD

## Future possibilities

1. A parallel harness runner in out of scope for this RFC, but is clearly the motivation for memory limits.
