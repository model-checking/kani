# RMC Dashboard

The [RMC Dashboard](./dashboard/index.html) is a testing tool based on [Litani](https://github.com/awslabs/aws-build-accumulator).

The purpose of the dashboard to show the level of support in RMC for all Rust features.
To this end, we use Rust code snippet examples from the following general Rust documentation books:
 * The Rust Reference
 * The Rustonomicon
 * The Rust Unstable Book
 * Rust By Example

However, not all examples from these books are suited for verification.
For instance, some of them are only included to show what is valid Rust code (or what is not).

Because of that, we run up to three different types of jobs when generating the dashboard:
 * `check` jobs (`BUILD` stage): This check uses the Rust front-end to detect if the example is valid Rust code.
 * `codegen` jobs (`TEST` stage): This check uses the RMC back-end to determine if we can generate GotoC code.
 * `verification` jobs (`REPORT` stage): This check uses CBMC to obtain a verification result.

Note that these are incremental: A `verification` job depends on a previous `codegen` job.
Similary, a `codegen` job depends on a `check` job.

> **Warning:** [Litani](https://github.com/awslabs/aws-build-accumulator) does not support
> hierarchical views nor custom stages at the moment. For this reason, the results are
> displayed for each example using Litani's default stages (`BUILD`, `TEST` and `REPORT`).

Before running the above mentioned jobs, we pre-process the examples to:
 1. Set the expected output according to flags present in the code snippet.
 2. Add any required compiler/RMC flags (e.g., CBMC unwinding flags).
 3. Include custom assertions for verification (only in the case of `verification` jobs).

Finally, we run all jobs, collect their outputs and compare them against the expected outputs.
The results are summarized as follows: If the obtained and expected outputs differ,
the color of the stage bar will be red. Otherwise, it will be blue.
If an example shows one red bar, it is considered a failed example that cannot be handled by RMC.

The [RMC Dashboard](./dashboard/index.html) is automatically updated whenever
a PR gets merged into RMC.

> **Tip:** In addition, we publish a [text version of the dashboard](./dashboard/dashboard.txt)
> while we work on adding more features to [Litani](https://github.com/awslabs/aws-build-accumulator).
> The [text-based dashboard](./dashboard/dashboard.txt) displays the same results in hierarchical way.
