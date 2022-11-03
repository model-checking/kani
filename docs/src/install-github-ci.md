# GitHub Action

Kani offers a GitHub Action for running Kani in CI. As of now, only
Ubuntu 20.04 with `x86_64-unknown-linux-gnu` is supported for Kani in
CI.

## Using Kani in your GitHub workflow
Our GitHub Action is available in the GitHub Marketspace with the name
`model-checking/kani-github-action`

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
        uses: actions/checkout@v3

      - name: 'Run Kani on your code.'
        uses: model-checking/kani-github-action@vVER.SION
```

This will run `cargo kani` on the code you checked out.

### Options

The action takes the following optional parameters:

- `command`: The command to run. 
  Defaults to `cargo kani`.
  Most often, you will not need to change this.
- `working-directory`: The directory to execute the command in.
  Defaults to `.`.
  Useful if your repository has multiple crates, and you only want to run on one of them.
- `args`: The arguments to pass to `cargo kani.
  See `cargo kani --help` for a full list of options.
  Useful options include:
  - `--output-format=terse` to generate terse output.
  - `--tests` to run on proofs inside the `test` module (needed for running Bolero).
  - `--workspace` to run on all creates within your repository.

## FAQ
- **Kani takes too long for my CI**: Try running Kani on a
  [schedule](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#schedule)
  with desired frequency.
- **Kani Silently Crashes with no logs**: Few possible reasons:
  - Kani ran out of RAM. GitHub offers up to 7GB of RAM, but Kani may
    use more. Run locally to confirm.
  - GitHub terminates jobs longer than 6 hours.
  - Otherwise, consider filing an issue [here](https://github.com/model-checking/kani/issues).
