# Application

You may be interested in applying Kani if you're in this situation:

1. You're working on a moderately important project in Rust.
2. You've already invested heavily in testing to ensure correctness.
3. You want to invest further, to gain a much higher degree of assurance.

> If you haven't already, we recommend techniques like property testing (e.g. with [`proptest`](https://github.com/AltSysrq/proptest)) before attempting model checking.
> These yield good results, are very cheap to apply, and are often easier to adopt and debug.
> Kani is a next step: a tool that can be applied once cheaper tactics are no longer yielding results, or once the easier to detect issues have already been dealt with.

In this section, we explain [how Kani compares with other tools](./tool-comparison.md)
and suggest [where to start applying Kani in real code](./tutorial-real-code.md).
