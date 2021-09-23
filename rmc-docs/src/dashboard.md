# RMC Dashboard

The [RMC Dashboard](./dashboard/index.html) is a testing tool based on [Compiletest](https://rustc-dev-guide.rust-lang.org/tests/intro.html) and [Litani](https://github.com/awslabs/aws-build-accumulator).

The purpose of the dashboard to show the level of support in RMC for all Rust features.
To this end, we use Rust code snippet examples from the following general Rust documentation books:
 * The Rust Reference
 * The Rustonomicon
 * The Rust Unstable Book
 * Rust by Example

However, not all examples from these books are suited for verification.
Because of that, we run three different types of jobs when generating the dashboard:
 * `check` jobs (`BUILD`): This check only uses the Rust front-end to detect if the example is valid Rust code.
 * `codegen` jobs (`TEST`): This check uses the Rust front-end and the RMC back-end to determine if we can generate GotoC code.
 * `verification` jobs (`REPORT`): This check uses all of above and CBMC to obtain a verification result.

Before running the above mentioned jobs, we pre-process the examples to:
 1. Set the expected output according to flags present in the code snippet.
 2. Add any required compiler/RMC flags (e.g., CBMC unwinding flags).
 3. Include custom assertions for verification (only in the case of `verification` jobs).

Finally, we run all jobs, collect their outputs and compare them against the expected outputs.

The [RMC Dashboard](./dashboard/index.html) displays a summary of the obtained results.
