# GitHub CI Action

Kani offers a GitHub action for running Kani in the CI. As of now,
only Ubuntu 20.04 wtih `x86_64-unknown-linux-gnu` is supported for
Kani in the CI.

Other platforms are either not yet supported or require instead that
you [build from source](build-from-source.md).

## Using Kani in Your GitHub Workflow.
Our GitHub CI Action is available in the GitHub Marketspace with the
name `model-checking/kani`

The following workflow snippet will checkout your repository and Run
Kani on it whenever a push or pull request occurs. Replace `VER.SION`
with the version of Kani you want to run with.

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
