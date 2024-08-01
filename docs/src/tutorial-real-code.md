# Where to start on real code

It can be daunting to find the right place to start writing proofs for a real-world project.
This section will try to help you get over that hurdle.

In general, you're trying to do three things:

1. Find a place where it'd be valuable to have a proof.
2. Find a place where it won't be too difficult to prove something, just to start.
3. Figure out what a feasible longer-term goal might be.

**By far, the best strategy is to follow your testing.**
Places where proof will be valuable are often places where you've written a lot of tests, because they're valuable there for the same reasons.
Likewise, code structure changes to make functions more unit-testable will also make functions more amenable to proof.
Often, by examining existing unit tests (and especially property tests), you can easily find a relatively self-contained function that's a good place to start.

## Where is proof valuable?

1. Where complicated things happen with untrusted user input.
These are often the critical "entry points" into the code.
These are also places where you probably want to try other techniques (e.g., fuzz testing).

2. Where `unsafe` is used extensively.
These are often places where you'll already have concentrated a lot of tests.

3. Where you have a complicated implementation that accomplishes a much simpler abstract problem.
Ideal places for property testing, if you haven't tried that already.
But the usual style of property tests you often write here (generate large random lists of operations, then compare between concrete and abstract model) won't be practical to directly port to model checking.

4. Where normal testing "smells" intractable.

## Where is it easier to start?

1. Find crates or files with smaller lists of dependencies.
Dependencies can sometimes blow up the tractability of proofs.
This can usually be handled, but requires a lot more investment to make it happen, and so isn't a good place to start.

2. Don't forget to consider starting with your dependencies.
Sometimes the best place to start won't be your code, but the code that you depend on.
If it's used by more projects that just yours, it will be valuable to more people, too!

3. Find well-tested code.
When you make changes to improve the unit-testability of code, that also makes it more amenable to proof, too.

Here are some things to avoid, when starting out:

1. Lots of loops, or at least nested loops.
As we saw in the [tutorial](./tutorial-loop-unwinding.md), right now we often need to put upper bounds on loops to make more limited claims.

2. Inductive data structures.
These are data structures with unbounded size (e.g., linked lists or trees.)
These can be hard to model since you need to set bounds on their size, similar to what happens with loops.

3. Input/Output code.
Kani doesn't model I/O, so if your code depends on behavior like reading/writing to a file, you won't be able to prove anything.
This is one obvious area where testability helps provability: often we separate I/O and "pure" computation into different functions, so we can unit-test the latter.

4. Deeper call graphs.
Functions that call a lot of other functions can require more investment to make tractable.
They may not be a good starting point.

5. Significant global state.
Rust tends to discourage this, but it still exists in some forms.


## Your first proof

A first proof will likely start in the following form:

1. Nondeterministically initialize variables that will correspond to function inputs, with as few constraints as possible.
2. Call the function in question with these inputs.
3. Don't (yet) assert any post-conditions.

Running Kani on this simple starting point will help figure out:

1. What unexpected constraints might be needed on your inputs (using `kani::assume`) to avoid "expected" failures.
2. Whether you're over-constrained. Check the coverage report using `--coverage -Z line-coverage`. Ideally you'd see 100% coverage, and if not, it's usually because you've assumed too much (thus over-constraining the inputs).
3. Whether Kani will support all the Rust features involved.
4. Whether you've started with a tractable problem.
(Remember to try setting `#[kani::unwind(1)]` to force early termination and work up from there.)

Once you've got something working, the next step is to prove more interesting properties than just what Kani covers by default.
You accomplish this by adding new assertions (not just in your harness, but also to the code being run).
Even if a proof harness has no post-conditions being asserted directly, the assertions encountered along the way can be meaningful proof results by themselves.


## Examples of the use of Kani

On the [Kani blog](https://model-checking.github.io/kani-verifier-blog/), we've documented worked examples of applying Kani:

1. [The `Rectangle` example of the Rust Book](https://model-checking.github.io/kani-verifier-blog/2022/05/04/announcing-the-kani-rust-verifier-project.html)
2. [A Rust standard library CVE](https://model-checking.github.io/kani-verifier-blog/2022/06/01/using-the-kani-rust-verifier-on-a-rust-standard-library-cve.html)
3. [Verifying a part of Firecracker](https://model-checking.github.io/kani-verifier-blog/2022/07/13/using-the-kani-rust-verifier-on-a-firecracker-example.html)
