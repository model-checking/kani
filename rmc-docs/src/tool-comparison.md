# Comparison with other tools

**Fuzzing** (for example, with [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)) is a blind approach to random testing.
A fuzzer generally provides an input of random bytes, and then examines fairly generic properties ("doesn't crash or commit undefined behavior") about the behavior of the resulting program.
Fuzzers generally get their power through a kind of evolutionary algorithm that rewards new mutant inputs that "discover" new branches of the program under test.
Fuzzers are excellent for testing security boundaries, precisely because they make no validity assumptions about the input.

**Property testing** (for example, with [`proptest`](https://github.com/AltSysrq/proptest)) is a non-blind approach to random testing.
Specific ranges of random values for specific types are generated several times, and the assertions of the test are checked for each.
Tests in this style do actually state properties: "forall values (of some kind), this condition holds."
But property testing can only sample randomly a few of those values to test (though property testing libraries frequently give interesting "edge cases" higher probability, making them more effective at bug-finding).

**Model checking** is similar to these techniques in how you use them, but model checking is non-random and exhaustive.
Thus, properties checked with a model checker are effectively proofs.
Instead of naively trying all possible inputs (which could be infeasible and blow up exponentially), model checkers like RMC will cleverly encode program traces as "SMT" problems, and hand them off to SMT solvers.
Again, SMT solving is an NP-complete problem, but most practical programs can be model checked within milliseconds to seconds (with notable exceptions: you can easily try to reverse a cryptographic hash with a model checker, but good luck getting it to terminate!)

Model checking allows you to prove non-trivial properties about programs, and check those proofs in roughly the same amount of time as a traditional test suite would take to run.
The downside is many types of properties can quickly become "too large" to practically model check, and so writing "proof harnesses" (very similar to property tests and fuzzer harnesses) requires some skill to understand why the solver is not terminating, and fix the structure of the problem you're giving it so that it does.
This process basically boils down to "debugging" the proof.
