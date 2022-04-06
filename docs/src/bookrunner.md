# Book runner

The [book runner](./bookrunner/index.html) is a testing tool based on [Litani](https://github.com/awslabs/aws-build-accumulator).

The purpose of the book runner is to get data about feature coverage in Kani.
To this end, we use Rust code snippet examples from the following general Rust documentation books:
 * The Rust Reference
 * The Rustonomicon
 * The Rust Unstable Book
 * Rust By Example

However, not all examples from these books are suited for verification.
For instance, some of them are only included to show what is valid Rust code (or what is not).

Because of that, we run up to three different types of jobs when generating the report:
 * `check` jobs: This check uses the Rust front-end to detect if the example is valid Rust code.
 * `codegen` jobs: This check uses the Kani back-end to determine if we can generate GotoC code.
 * `verification` jobs: This check uses CBMC to obtain a verification result.

Note that these are incremental: A `verification` job depends on a previous `codegen` job.
Similary, a `codegen` job depends on a `check` job.

> **NOTE**: [Litani](https://github.com/awslabs/aws-build-accumulator) does not
> support hierarchical views at the moment. For this reason, we are publishing a
> [text version of the book runner report](./bookrunner/bookrunner.txt) which
> displays the same results in a hierarchical way while we work on [improvements
> for the visualization and navigation of book runner
> results](https://github.com/model-checking/kani/issues/699).

Before running the above mentioned jobs, we pre-process the examples to:
 1. Set the expected output according to flags present in the code snippet.
 2. Add any required compiler/Kani flags (e.g., unwinding).

Finally, we run all jobs, collect their outputs and compare them against the expected outputs.
The results are summarized as follows: If the obtained and expected outputs differ,
the color of the stage bar will be red. Otherwise, it will be blue.
If an example shows one red bar, it's considered a failed example that cannot be handled by Kani.

The [book runner report](./bookrunner/index.html) and [its text version](./bookrunner/bookrunner.txt) are
automatically updated whenever a PR gets merged into Kani.

## The book running procedure

This section describes how the book runner operates at a high level.

To kick off the book runner process use:

```bash
cargo run -p bookrunner
```

The main function of the bookrunner is `generate_run()` (code available
[here](https://github.com/model-checking/kani/blob/main/tools/bookrunner/src/books.rs))
which follows these steps:
 1. Sets up all the books, including data about their summaries.
 2. Then, for each book:
  * Calls the `parse_hierarchy()` method to parse its summary
    files.
  * Calls the `extract_examples()` method to extract all
    examples from the book. Note that `extract_examples()` uses `rustdoc`
    functions to ensure the extracted examples are runnable.
  * Checks if there is a corresponding `.props` file
    in `src/tools/bookrunner/configs/`. If there is, prepends the contents of these files
    ([testing options](./regression-testing.md#testing-options)) to the example.
  * The resulting examples are written to the `src/test/bookrunner/books/` folder.

> In general, the path to a given example is
> `src/test/bookrunner/books/<book>/<chapter>/<section>/<subsection>/<line>.rs`
> where `<line>` is the line number where the example appears in the markdown
> file where it's written. The `.props` files mentioned above follow the same
> naming scheme in order to match them and detect conflicts.

 3. Runs all examples using
   [Litani](https://github.com/awslabs/aws-build-accumulator) with the
   `litani_run_tests()` function.
 4. Parses the Litani log file with `parse_litani_output(...)`.
 5. Generates the [text version of the bookrunner](./bookrunner/bookrunner.txt)
    with `generate_text_bookrunner(...)`.

> **NOTE**: Any changes done to the examples in `src/test/bookrunner/books/` may
> be overwritten if the bookrunner is executed.
