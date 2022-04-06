# Comparison with other tools

**Fuzzing** (for example, with [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)) is a unguided approach to random testing.
A fuzzer generally provides an input of random bytes, and then examines fairly generic properties (such as "doesn't crash" or "commit undefined behavior") about the resulting program.

Fuzzers generally get their power through a kind of evolutionary algorithm that rewards new mutant inputs that "discover" new branches of the program under test.
Fuzzers are excellent for testing security boundaries, precisely because they make no validity assumptions (hence, they are "unguided") when generating the input.

**Property testing** (for example, with [Proptest](https://github.com/AltSysrq/proptest)) is a guided approach to random testing.
"Guided" in the sense that the test generally provides a strategy for generating random values that constrains their range.
The purpose of this strategy is to either focus on interesting values, or avoid failing assertions that only hold for a constrained set of inputs.
Tests in this style do actually state properties: *For all inputs (of some constrained kind), this condition should hold*.

Property testing is often quite effective, but the engine can't fully prove the property: It can only sample randomly a few of those values to test (though property testing libraries frequently give interesting "edge cases" a higher probability, making them more effective at bug-finding).

**Model checking** is similar to these techniques in how you use them, but it's non-random and exhaustive (though often only up to some bound on input or problem size).
Thus, properties checked with a model checker are effectively proofs.
Instead of naively trying all possible _concrete_ inputs (which could be infeasible and blow up exponentially), model checkers like Kani will cleverly encode program traces as _symbolic_ "[SAT](https://en.wikipedia.org/wiki/Boolean_satisfiability_problem)/[SMT](https://en.wikipedia.org/wiki/Satisfiability_modulo_theories)" problems, and hand them off to SAT/SMT solvers.
Again, SAT/SMT solving is an [NP-complete](https://en.wikipedia.org/wiki/NP-completeness) problem, but most practical programs can be model- checked within milliseconds to seconds (with notable exceptions: you can easily try to reverse a cryptographic hash with a model checker, but good luck getting it to terminate!)

Model checking allows you to prove non-trivial properties about programs, and check those proofs in roughly the same amount of time as a traditional test suite would take to run.
The downside is many types of properties can quickly become "too large" to practically model-check, and so writing "proof harnesses" (very similar to property tests and fuzzer harnesses) requires some skill to understand why the solver is not terminating and fix the structure of the problem you're giving it so that it does.
This process basically boils down to "debugging" the proof.

## Looking for concurrency?

At present, Kani [does not support verifying concurrent code](./rust-feature-support.md).
Two tools of immediate interest are [Loom](https://github.com/tokio-rs/loom) and [Shuttle](https://github.com/awslabs/shuttle).
Loom attempts to check all possible interleavings, while Shuttle chooses interleavings randomly.
The former is sound (like Kani), but the latter is more scalable to large problem spaces (like property testing).

## Other tools

The Rust Formal Methods Interest Group maintains [a list of interesting Rust verification tools](https://rust-formal-methods.github.io/tools.html).
