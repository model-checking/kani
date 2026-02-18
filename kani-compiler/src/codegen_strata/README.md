# Kani Strata Backend

This implementation adds a new codegen backend to Kani that emits Strata Core dialect intermediate representation.

## Overview

Strata (https://github.com/strata-org/Strata) is a unified platform for formalizing language syntax and semantics. The Strata Core dialect is similar to the Boogie Intermediate Verification Language and provides:

- Procedures with specifications (preconditions, postconditions, frame conditions)
- Imperative statements (assignments, conditionals, loops)
- Built-in types (booleans, bitvectors, integers, maps)
- SMT-based verification

## Architecture

The Strata backend translates Rust MIR (Mid-level Intermediate Representation) to Strata Core dialect:

```
Rust Source → MIR → Strata Core IR (.core.st files) → Strata Verifier
```

### Components

1. **codegen_strata/mod.rs** - Module entry point
2. **codegen_strata/strata_builder.rs** - Builder for constructing Strata programs
3. **codegen_strata/mir_to_strata.rs** - MIR to Strata translation logic
4. **codegen_strata/compiler_interface.rs** - Rustc compiler backend interface

## Usage

### Building with Strata Backend

To build Kani with the Strata backend enabled:

```bash
cd kani
cargo build --features strata
```

### Running Verification

To verify a Rust program using the Strata backend:

```bash
cargo kani --backend strata your_file.rs
```

This will generate an `output.core.st` file containing the Strata IR.

### Verifying with Strata

Once you have the `.core.st` file, you can verify it using Strata:

```bash
cd /path/to/Strata
lake exe StrataVerify /path/to/output.core.st
```

## Implementation Status

### Current Implementation (Minimal)

The current implementation provides a minimal proof-of-concept:

- ✅ Basic module structure
- ✅ Strata IR builder for procedures and variables
- ✅ MIR traversal skeleton
- ✅ Compiler backend integration
- ✅ Feature flag configuration

### TODO: Complete Implementation

To make this production-ready, the following needs to be implemented:

#### 1. Type Translation
- [ ] Map Rust types to Strata types (bool, bitvectors, integers)
- [ ] Handle complex types (structs, enums, arrays)
- [ ] Translate lifetimes and references

#### 2. Expression Translation
- [ ] Arithmetic operations
- [ ] Logical operations
- [ ] Comparisons
- [ ] Function calls
- [ ] Memory operations (loads, stores)

#### 3. Statement Translation
- [ ] Assignments
- [ ] Conditionals (if/else)
- [ ] Loops (while, for)
- [ ] Pattern matching
- [ ] Assertions and assumptions

#### 4. Control Flow
- [ ] Basic block translation
- [ ] Goto statements
- [ ] Return statements
- [ ] Panic/abort handling

#### 5. Specifications
- [ ] Extract Kani proof harness attributes
- [ ] Generate preconditions from `kani::assume`
- [ ] Generate postconditions from `assert!`
- [ ] Frame conditions for global variables

#### 6. Advanced Features
- [ ] Trait method calls
- [ ] Closures
- [ ] Async/await
- [ ] Unsafe code
- [ ] FFI calls

#### 7. Testing & Validation
- [ ] Comprehensive test suite
- [ ] Integration tests with Strata verifier
- [ ] Benchmarks comparing with CBMC backend

## Example Translation

### Rust Code
```rust
#[kani::proof]
fn test_add() {
    let x: u32 = kani::any();
    let y: u32 = kani::any();
    kani::assume(x < 100);
    kani::assume(y < 100);
    let z = x + y;
    assert!(z < 200);
}
```

### Expected Strata Core IR
```
program Core;

procedure test_add() returns ()
spec {
  requires (x < 100);
  requires (y < 100);
  ensures (z < 200);
}
{
  var x : bv32;
  var y : bv32;
  var z : bv32;
  
  havoc x;
  havoc y;
  assume (x < 100);
  assume (y < 100);
  z := x + y;
  assert (z < 200);
}
```

## Design Decisions

### Why Strata Core?

1. **Verification-focused**: Strata Core is designed for program verification
2. **SMT-based**: Leverages mature SMT solvers
3. **Extensible**: Can be extended with custom dialects
4. **Formal semantics**: Has formal operational semantics in Lean

### Translation Strategy

The translation follows these principles:

1. **Preserve semantics**: Maintain Rust's operational semantics
2. **Explicit control flow**: Make all control flow explicit
3. **Type safety**: Preserve type information where possible
4. **Verification-friendly**: Generate IR suitable for SMT encoding

## Contributing

To extend this implementation:

1. Start with simple cases (arithmetic, basic control flow)
2. Add comprehensive tests for each feature
3. Validate against Strata verifier
4. Document translation patterns

## References

- [Strata Repository](https://github.com/strata-org/Strata)
- [Strata Architecture](https://github.com/strata-org/Strata/blob/main/docs/Architecture.md)
- [Boogie IVL](https://github.com/boogie-org/boogie)
- [Kani Documentation](https://model-checking.github.io/kani/)
