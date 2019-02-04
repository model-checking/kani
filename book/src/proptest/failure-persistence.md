# Failure Persistence

By default, when Proptest finds a failing test case, it _persists_ that
failing case in a file named after the source containing the failing test,
but in a separate directory tree rooted at `proptest-regressions`. Later
runs of tests will replay those test cases before generating novel cases.
This ensures that the test will not fail on one run and then spuriously
pass on the next, and also exposes similar tests to the same
known-problematic input.

(If you do not have an obvious source directory, you may instead find files
next to the source files, with a different extension.)

It is recommended to check these files in to your source control so that
other test runners (e.g., collaborators or a CI system) also replay these
cases.

Note that, by default, all tests in the same crate will share that one
persistence file. If you have a very large number of tests, it may be
desirable to separate them into smaller groups so the number of extra test
cases that get run is reduced. This can be done by adjusting the
`failure_persistence` flag on `Config`.

There are two ways this persistence could theoretically be done.

The immediately obvious option is to persist a representation of the value
itself, for example by using Serde. While this has some advantages,
particularly being resistant to changes like tweaking the input strategy,
it also has a lot of problems. Most importantly, there is no way to
determine whether any given value is actually within the domain of the
strategy that produces it. Thus, some (likely extremely fragile) mechanism
to ensure that the strategy that produced the value exactly matches the one
in use in a test case would be required.

The other option is to store the _seed_ that was used to produce the
failing test case. This approach requires no support from the strategy or
the produced value. If the strategy in use differs from the one used to
produce failing case that was persisted, the seed may or may not produce
the problematic value, but nonetheless produces a valid value. Due to these
advantages, this is the approach Proptest uses.
