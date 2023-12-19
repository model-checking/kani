![](./kani-logo.png)

**This is a feature branch of Kani that contains an experimental Boogie-based backend. If you are looking for the main version of Kani, visit https://github.com/model-checking/kani instead.**

The main version of Kani translates [MIR](https://blog.rust-lang.org/2016/04/19/MIR.html) to Goto and uses CBMC as the verification engine.

This branch implements a translation of MIR to [Boogie](https://github.com/boogie-org/boogie) that can be verified using the Boogie Verifier.
It currently supports a very small subset of Rust.

The Boogie backend is not included in Kani's [releases](https://github.com/model-checking/kani/releases), so in order to use it, you need to clone this branch and build from source.
Refer to the [Installing from source code](https://model-checking.github.io/kani/build-from-source.html) section of the documentation for instructions on how to build Kani from source.

## Prerequisites

You need to have the Boogie Verifier installed.
Refer to https://github.com/boogie-org/boogie#installation for the installation instructions.

## Instructions for Using the Boogie Backend

To invoke the Boogie backend, pass the unstable `-Zboogie` option to the Kani command, e.g.
```
kani test.rs -Zboogie
```
or
```
cargo kani -Zboogie
```
Kani will print the name of the generated Boogie file, e.g.
```
Writing Boogie file to /home/ubuntu/test1_main.symtab.bpl
```
You need to manually invoke the Boogie verifier on the generated Boogie file to verify it:
```bash
$ boogie test1_main.symtab.bpl 

Boogie program verifier finished with 1 verified, 0 errors
```

### Example

Consider the following function (whose source file can be found [here](https://github.com/model-checking/kani/blob/features/boogie/tests/script-based-boogie/unbounded_array_copy/test.rs)) which creates an array of integers, `src`, and copies it into another array of integers, `dst`:
```rust
#[kani::proof]
fn copy() {
    let src = kani::array::any_array::<i32>();
    let mut dst = kani::array::any_array::<i32>();
    let src_len: usize = src.len();
    let dst_len: usize = dst.len();

    // copy as many elements as possible of `src` to `dst`
    let mut i: usize = 0;
    // Loop invariant: forall j: usize :: j < i => dst[j] == src[j])
    while i < src_len && i < dst_len {
        dst[i] = src[i];
        i = i + 1;
    }

    // check that the data was copied
    i = 0;
    while i < src_len && i < dst_len {
        kani::assert(dst[i] == src[i], "element doesn't have the correct value");
        i = i + 1;
    }
}
```
The arrays are created using `kani::array::any_array`, which creates an array with non-deterministic content and length.

Running Kani with `-Zboogie` produces the following Boogie file:
<details>
  <summary>Click to expand Boogie file contents</summary>

```boogie
// Datatypes:
datatype $Array<T> { $Array(data: [bv64]T, len: bv64) }

// Functions:
function {:bvbuiltin "bvult"} $BvUnsignedLessThan<T>(lhs: T, rhs: T) returns (bool);

function {:bvbuiltin "bvslt"} $BvSignedLessThan<T>(lhs: T, rhs: T) returns (bool);

function {:bvbuiltin "bvugt"} $BvUnsignedGreaterThan<T>(lhs: T, rhs: T) returns (bool);

function {:bvbuiltin "bvsgt"} $BvSignedGreaterThan<T>(lhs: T, rhs: T) returns (bool);

function {:bvbuiltin "bvadd"} $BvAdd<T>(lhs: T, rhs: T) returns (T);

function {:bvbuiltin "bvor"} $BvOr<T>(lhs: T, rhs: T) returns (T);

function {:bvbuiltin "bvand"} $BvAnd<T>(lhs: T, rhs: T) returns (T);

function {:bvbuiltin "bvshl"} $BvShl<T>(lhs: T, rhs: T) returns (T);

function {:bvbuiltin "bvlshr"} $BvShr<T>(lhs: T, rhs: T) returns (T);

// Procedures:
procedure _RNvCs7Oe89NXlEjS_4test4copy() 
{
  var src: $Array bv32;
  var dst: $Array bv32;
  var src_len: bv64;
  var _4: $Array bv32;
  var dst_len: bv64;
  var _6: $Array bv32;
  var i: bv64;
  var _8: bool;
  var _9: bv64;
  var _10: bool;
  var _11: bv64;
  var _12: bv32;
  var _13: bv32;
  var _14: $Array bv32;
  var _15: bv64;
  var _18: bv64;
  var _19: bv64;
  var _20: bv64;
  var _21: bool;
  var _22: bv64;
  var _23: bool;
  var _24: bv64;
  var _26: bool;
  var _27: bv32;
  var _28: bv32;
  var _29: $Array bv32;
  var _30: bv64;
  var _31: bv32;
  var _32: bv32;
  var _33: $Array bv32;
  var _34: bv64;
  var _35: bv64;
  var _36: bv64;
  bb0:
  havoc src; 
  goto bb1;
  bb1:
  havoc dst; 
  goto bb2;
  bb2:
  src_len := src->len;
  bb3:
  dst_len := dst->len;
  bb4:
  i := 0bv64;
  goto bb5;
  bb5:
  _9 := i;
  _8 := $BvUnsignedLessThan(_9, src_len);
  if ((_8 == false)) {
    goto bb11;
  } else {
    goto bb6;
  }
  bb6:
  _11 := i;
  _10 := $BvUnsignedLessThan(_11, dst_len);
  if ((_10 == false)) {
    goto bb11;
  } else {
    goto bb7;
  }
  bb11:
  i := 0bv64;
  goto bb12;
  bb12:
  _22 := i;
  _21 := $BvUnsignedLessThan(_22, src_len);
  if ((_21 == false)) {
    goto bb19;
  } else {
    goto bb13;
  }
  bb13:
  _24 := i;
  _23 := $BvUnsignedLessThan(_24, dst_len);
  if ((_23 == false)) {
    goto bb19;
  } else {
    goto bb14;
  }
  bb19:
  return;
  bb14:
  _30 := i;
  _28 := dst->data[(_30)];
  bb15:
  _27 := _28;
  _34 := i;
  _32 := src->data[(_34)];
  bb16:
  _31 := _32;
  _26 := (_27 == _31);
  assert _26;
  bb17:
  _35 := i;
  _36 := $BvAdd(_35, 1bv64);
  bb18:
  i := _36;
  goto bb12;
  bb7:
  _15 := i;
  _13 := src->data[(_15)];
  bb8:
  _12 := _13;
  _18 := i;
  bb9:
  dst->data[(_18)] := _12;
  _19 := i;
  _20 := $BvAdd(_19, 1bv64);
  bb10:
  i := _20;
  goto bb5;
}
```
</details>

If we invoke the Boogie Verifier on the generated file, it fails to prove the assertion:
```bash
$ boogie test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl
test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(119,3): Error: this assertion could not be proved
Execution trace:
    test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(59,3): bb0
    test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(72,3): bb5
    test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(76,5): anon8_Then
    test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(91,3): bb12
    test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(97,5): anon10_Else
    test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl(105,5): anon11_Else

Boogie program verifier finished with 0 verified, 1 error
```
This is due to the presence of a loop, which requires specifying a loop invariant.
If we add the following assertion after the `bb5:` line (which is the loop head):
```
assert (forall j: bv64 :: $BvUnsignedLessThan(j, i) ==> dst->data[j] == src->data[j]);
```
verification succeeds:
```bash
$ boogie test__RNvCs7Oe89NXlEjS_4test4copy.symtab.bpl

Boogie program verifier finished with 1 verified, 0 errors
```

## GitHub Action

Use Kani in your CI with `model-checking/kani-github-action@VERSION`. See the
[GitHub Action section in the Kani
book](https://model-checking.github.io/kani/install-github-ci.html)
for details.

## Security
See [SECURITY](https://github.com/model-checking/kani/security/policy) for more information.

## Contributing
If you are interested in contributing to Kani, please take a look at [the developer documentation](https://model-checking.github.io/kani/dev-documentation.html).

## License
### Kani
Kani is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.

### Rust
Kani contains code from the Rust project.
Rust is primarily distributed under the terms of both the MIT license and the Apache License (Version 2.0), with portions covered by various BSD-like licenses.

See [the Rust repository](https://github.com/rust-lang/rust) for details.
