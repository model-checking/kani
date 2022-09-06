# GitHub Action

Kani offers a GitHub Action for running Kani in CI. As of now, only
Ubuntu 20.04 with `x86_64-unknown-linux-gnu` is supported for Kani in
CI.

## Using Kani in your GitHub workflow
Our GitHub Action is available in the GitHub Marketspace with the name
`model-checking/kani`

The following workflow snippet will checkout your repository and run
`cargo kani` on it whenever a push or pull request occurs. Replace
`VER.SION` with the version of Kani you want to run with.

```yaml
name: Kani CI
on:
  pull_request:
  push:
jobs:
  run-kani:
    runs-on: ubuntu-20.04
    steps:
      - name: 'Checkout your code.'
        uses: actions/checkout@v2

      - name: 'Run Kani on your code.'
        uses: model-checking/kani@vVER.SION
```

This will run `cargo kani --workspace` on the code you checked
out. You can also provide a custom command for running Kani. For
example, the below code runs 2 commands in sequence where the first
command compiles the crate without running Kani, and the second runs
Kani on specified packages.

```
      - name: 'Run Kani on your code.'
        uses: model-checking/kani@vVER.SION
        with:
          command: |
            cargo kani --only-codegen
            cargo kani -p mypackage-a -p mypackage-b
```

## FAQ
- **Kani takes too long for my CI**: Try running Kani on a
  [schedule](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#schedule)
  with desired frequency.
- **Kani Silently Crashes with no logs**: Few possible reasons:
  - Kani ran out of RAM. GitHub offers up to 7GB of RAM, but Kani may
    use more. Run locally to confirm.
  - GitHub terminates jobs longer than 6 hours.
  - Otherwise, consider filing an issue [here](https://github.com/model-checking/kani/issues).
