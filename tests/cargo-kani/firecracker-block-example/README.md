This example accompanies Kani's post on Firecracker. This describes a proof harness for ensuring that the Firecracker block device `parse` method adheres to a virtio requirement. We implement this as a standalone example with some simplifications (search for "Kani change" in the source).

## Reproducing results locally

### Dependencies

  - Rust edition 2018
  - [Kani](https://model-checking.github.io/kani/getting-started.html)

If you have problems installing Kani then please file an [issue](https://github.com/model-checking/kani/issues/new/choose).

### Using Kani

Since there is only one harness in this example you can simply do:

```bash
$ cargo kani
# expected result: verification success
```
