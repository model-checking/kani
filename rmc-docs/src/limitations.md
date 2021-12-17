# Limitations

## Rust feature support

*Define Yes, No and Partial*

Reference | Feature | Support | Observations |
--- | --- | --- | --- |
3.1 | Macros By Example | Yes | |
3.2 | Procedural Macros | Yes | |
4 | Crates and source files | Yes | |
5 | Conditional compilation | Yes | |
6.1 | Modules | Yes | |
6.2 | Extern crates | Yes | |
6.3 | Use declarations | Yes | |
6.4 | Functions | Yes | |
6.5 | Type aliases | Yes | |
6.6 | Structs | Yes | |
6.7 | Enumerations | Yes | |
6.8 | Unions | Yes | |
6.9 | Constant items | Yes | |
6.10 | Static items | Yes | |
6.11 | Traits | Yes | |
6.12 | Implementations | Yes | |
6.13 | External blocks | Yes | |
6.14 | Generic parameters | Yes | |
6.15 | Associated Items | Yes | |
7 | Attributes | Yes | |
8.1 | Statements | Yes | |
8.2.1 | Literal expressions | Yes | |
8.2.2 | Path expressions | Yes | |
8.2.3 | Block expressions | Yes | |
8.2.4 | Operator expressions | Yes | |
8.2.5 | Grouped expressions | Yes | |
8.2.6 | Array and index expressions | Yes | |
8.2.7 | Tuple and index expressions | Yes | |
8.2.8 | Struct expressions | Yes | |
8.2.9 | Call expressions | Yes | |
8.2.10 | Method call expressions | Yes | |
8.2.11 | Field access expressions | Yes | |
8.2.12 | Closure expressions | Yes | |
8.2.13 | Loop expressions | Yes | |
8.2.14 | Range expressions | Yes | |
8.2.15 | If and if let expressions | Yes | |
8.2.16 | Match expressions | Yes | |
8.2.17 | Return expressions | Yes | |
8.2.18 | Await expressions | No | [Concurrency](#concurrency) |
9 | Patterns | Partial | [Patterns](#patterns) |
10.1.1 | Boolean type | Yes | |
10.1.2 | Numeric types | Yes | |
10.1.3 | Textual types | Yes | |
10.1.4 | Never type | Yes | |
10.1.5 | Tuple types | Yes | |
10.1.6 | Array types | Yes | |
10.1.7 | Slice types | Yes | |
10.1.8 | Struct types | Yes | |
10.1.9 | Enumerated types | Yes | |
10.1.10 | Union types | Yes | |
10.1.11 | Function item types | Yes | |
10.1.12 | Closure types | Partial | |
10.1.13 | Pointer types | Partial | |
10.1.14 | Function pointer types | Partial | |
10.1.15 | Trait object types | Partial | |
10.1.16 | Impl trait type | Partial | |
10.1.17 | Type parameters | Partial | |
10.1.18 | Inferred type | Partial | |
10.2 | Dynamically Sized Types | Partial | |
10.3 | Type layout | Yes | |
10.4 | Interior mutability | Yes | |
10.5 | Subtyping and Variance | Yes | |
10.6 | Trait and lifetime bounds | Yes | |
10.7 | Type coercions | Partial | |
10.8 | Destructors | Partial | |
10.9 | Lifetime elision | Yes | |
11 | Special types and traits | Partial | |
| | `Box<T>` | Yes | |
| | `Rc<T>` | Yes | |
| | `Arc<T>` | Yes | |
| | `Pin<T>` | Yes | |
| | `UnsafeCell<T>` | Partial | |
| | `PhantomData<T>` | Partial | *Review this* |
| | Operator Traits | Partial | |
| | `Deref` and `DerefMut` | Yes | |
| | `Drop` | Partial | |
| | `Copy` | Yes | |
| | `Clone` | Yes | |
14 | Linkage | Yes | |
15.1 | Unsafe functions | Yes | |
15.2 | Unsafe blocks | Yes | |
15.3 | Behavior considered undefined | Partial | |
| | Data races | No | [Concurrency](#concurrency) |
| | Dereferencing dangling raw pointers | Yes | |
| | Dereferencing unaligned  raw pointers | No | |
| | Breaking pointer aliasing rules | No | |
| | Mutating immutable data | No | |
| | Invoking undefined behavior via compiler intrinsics | Partial | [Intrinsics](#intrinsics) |
| | Executing code compiled with platform features that the current platform does not support | No | |
| | Producing an invalid value, even in private fields and locals | No | |

## Observations on partially or unsupported features

### Assembly

To be written.

### Concurrency

Concurrent features are out of scope for RMC. In general, the verification of
concurrent programs continues to be an open research problem. In particular,
support for concurrent verification in CBMC is limited and RMC emits warnings
whenever it finds concurrent constructs in programs.

### Patterns

The code for handling patterns has not been tested in a thorough manner. We
expect to fully support this feature in the near future.

### Intrinsics

TBD