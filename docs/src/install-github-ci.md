# GitHub CI Action

Kani offers a GitHub action for running Kani in the CI. As of now,
only Ubuntu 20.04 wtih `x86_64-unknown-linux-gnu` is supported for
Kani in the CI.

## Using Kani in your GitHub workflow
Our GitHub CI Action is available in the GitHub Marketspace with the
name `model-checking/kani`

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


## Configuring Kani with flags

The github action itself does not take any flags that `cargo kani`
would take. Instead, they should be configured in `Cargo.toml`. See
["Configuration in Cargo.toml"](usage.md#configuration-in-cargotoml)
for details.
