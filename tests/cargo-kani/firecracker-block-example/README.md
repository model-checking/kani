This example accompanies Kani's post on Firecracker. This describes a proof harness for ensuring that the Firecracker block device `parse` method adheres to a virtio requirement.

## Reproducing results locally

### Dependencies

  - Rust edition 2018
  - [Kani](https://model-checking.github.io/kani/getting-started.html)

If you have problems installing Kani then please file an [issue](https://github.com/model-checking/kani/issues/new/choose).

### Using Kani

```bash
$ cargo kani --harness requirement_2642 --output-format terse
# expected result: verification success
```
