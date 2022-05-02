This example accompanies Kani's post on Firecracker. This describes a proof
harness for ensuring that the Firecracker block device `parse` method adheres
to a virtio requirement (see below). We implement this as a standalone example
with some simplifications (search for "Kani change" in the source).

This example is based on code from Firecracker. In particular,

  - <https://github.com/firecracker-microvm/firecracker/tree/main/src/devices/src/virtio/block>
  - <https://github.com/firecracker-microvm/firecracker/blob/main/src/devices/src/virtio/queue.rs>

## Virtio requirement

We implement a simple finite-state-machine checker in `descriptor_permission_checker.rs` that ensures the following:

> 2.6.4.2 Driver Requirements: Message Framing
>
> The driver MUST place any device-writable descriptor elements after any device-readable descriptor elements.
>
> Source: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-280004

## Reproducing results locally

### Dependencies

  - Rust edition 2018
  - [Kani](https://model-checking.github.io/kani/getting-started.html)

If you have problems installing Kani then please file an [issue](https://github.com/model-checking/kani/issues/new/choose).

### Using Kani

Since there is only one harness in this example you can simply do the
following, where the expected result is verification success.

```bash
$ cargo kani
# expected result: verification success
```
