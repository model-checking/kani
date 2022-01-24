# Kani tutorial

If you're interested in applying Kani, then you're probably in this situation:

1. You're working on a moderately important project in Rust.
2. You've already invested heavily in testing to ensure correctness, and possibly fuzzing to ensure the absence of shallow security issues.
3. You want to invest further, to gain a much higher degree of assurance.

> If you haven't already, we recommend techniques like property testing (e.g. with [`proptest`](https://github.com/AltSysrq/proptest)) before attempting model checking.
> These yield good results, are very cheap to apply, and are often easier to adopt and debug.
> Refactoring work to make your code more property-testable will generally also make the code more model-checkable as well.
> Kani is a next step: a tool that can be applied once cheaper tactics are no longer yielding results, or once the easier to detect issues have already been dealt with.

This tutorial will step you through a progression from simple to moderately complex tasks with Kani.
It's meant to ensure you can get started, and see at least some simple examples of how typical proofs are structured.
It will also teach you the basics of "debugging" proof harnesses, which mainly consists of diagnosing and resolving non-termination issues with the solver.

1. [Begin with Kani installation.](./install-guide.md) This will take through to running `kani` on your first Rust program.
2. Consider reading our [tool comparison](./tool-comparison.md) to understand what Kani is.
3. [Work through the tutorial.](./tutorial-first-steps.md)
4. Consider returning to the [tool comparison](./tool-comparison.md) after trying the tutorial to see if any of the abstract ideas have become more concrete.
