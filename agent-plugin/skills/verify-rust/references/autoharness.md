# AutoHarness

`autoharness` generates a harness for each eligible function (all arguments implement
`kani::Arbitrary`, or have a contract) and verifies them, with no hand-written code. It is the
fastest way to sweep a crate for panics, overflow, and UB. It is unstable and **CLI only**:
there is no MCP tool for it, so run it directly.

```bash
cargo kani autoharness -Z autoharness                       # whole crate
kani autoharness -Z autoharness src/lib.rs                  # single file
cargo kani autoharness -Z autoharness --include-pattern 'parser::' --exclude-pattern 'tests::'
cargo kani autoharness -Z autoharness --list                # only list what would be generated
```

Add `-Z export-json --export-json /tmp/auto.json` and read the result with
`scripts/summarize-results.py /tmp/auto.json --failures-only`; autoharness usually produces many
harnesses, and the raw output is very long.

Notes:

- `-Z autoharness` implies `-Z function-contracts` and `-Z loop-contracts`. Functions with a
  contract are verified against it.
- `--bounded-arguments` also covers functions taking slices/`&str`/`Vec`; results then hold
  only up to `--slice-bound`, `--string-bound`, and `--bounded-arbitrary-bound`. Such harnesses
  are marked "bounded" in the summary; report the bounds.
- `--constructor-args` builds values of types without `Arbitrary` through their constructors;
  results only cover values reachable that way.
- An autoharness checks that the function does not panic or trigger UB on *any* input. A
  failure may be a precondition the function documents but does not check; in that case write
  a contract (`references/contracts.md`) or a manual harness with the right assumptions.
- Run `kani autoharness --help` for all options.
