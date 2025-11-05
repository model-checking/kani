- **Feature Name:** JSON Export (`json-export`)
- **Feature Request Issue:** 
  - <https://github.com/model-checking/kani/issues/2572>
  - <https://github.com/model-checking/kani/issues/2636>
  - <https://github.com/model-checking/kani/issues/2574>
  - <https://github.com/model-checking/kani/issues/2562>
  - <https://github.com/model-checking/kani/issues/1194>
  - <https://github.com/model-checking/kani/issues/1219>
  - <https://github.com/model-checking/kani/issues/1045>
  - <https://github.com/model-checking/kani/issues/598>
  - <https://github.com/model-checking/kani/issues/3357>
- **RFC PR:** <https://github.com/yimingyinqwqq/kani-output/pull/2>
- **Status:** Unstable
- **Version:** 0

-------------------

## Summary

Export Kani verification results in structured JSON format with a validated schema, enabling programmatic analysis and tool integration.

## User Impact

It is difficult to integrate Kani verification results into automated workflows because results are only available in human-readable output text format. This limits:
- Cross-run analysis
- Integration with external applications/tools (dashboards, databases, LLMs)
- Automated harness result generation

This RFC adds JSON export as an opt-in feature, maintaining backward compatibility while enabling these automation scenarios.

## User Experience

Users export verification results using the `--export-json` flag:

```bash
cargo kani --export-json results.json
```

This works with all existing Kani options:

```bash
cargo kani --harness my_harness --export-json output.json
cargo kani --tests --export-json test_results.json
```

### JSON Schema

The output contains eight top-level blocks:

**1. Metadata** - Execution environment
```json
{
  "metadata": {
    "version": "1.0",
    "timestamp": "2025-10-30T12:00:00.000000Z",
    "kani_version": "0.65.0",
    "target": "x86_64-unknown-linux-gnu",
    "build_mode": "debug"
  }
}
```

**2. Project** - Codebase identification
```json
{
  "project": {
    "crate_name": ["example_crate"],
    "workspace_root": "/path/to/workspace"
  }
}
```

**3. Harness Metadata** - Source locations and attributes
```json
{
  "harness_metadata": [{
    "pretty_name": "example_harness",
    "mangled_name": "_RNvCs_example",
    "source": {
      "file": "src/lib.rs",
      "start_line": 10,
      "end_line": 15
    },
    "attributes": {
      "kind": "Proof",
      "should_panic": false
    },
    "contract": {
      "contracted_function_name": null
    },
    "has_loop_contracts": false
  }]
}
```

**4. Verification Results** - Harness results and checks
```json
{
  "verification_results": {
    "summary": {
      "total_harnesses": 1,
      "executed": 1,
      "successful": 1,
      "failed": 0,
      "duration_ms": 500
    },
    "results": [{
      "harness_id": "example_harness",
      "status": "Success",
      "duration_ms": 500,
      "checks": [{
        "id": 1,
        "function": "example_function",
        "status": "Success",
        "description": "assertion failed: x > 0",
        "location": {
          "file": "src/lib.rs",
          "line": "20",
          "column": "13"
        },
        "category": "assertion"
      }]
    }]
  }
}
```

**5. Error Details** - Top-level error classification
```json
{
  "error_details": {
    "has_errors": false,
    "error_type": null,
    "failed_properties_type": null,
    "exit_status": "success"
  }
}
```

**6. Property Details** - Property statistics
```json
{
  "property_details": [{
    "property_details": {
      "total_properties": 1,
      "passed": 1,
      "failed": 0,
      "unreachable": 0
    }
  }]
}
```

**7. CBMC Statistics** - Backend performance metrics
```json
{
  "cbmc": [{
    "harness_id": "example_harness",
    "cbmc_metadata": {
      "version": "6.7.1",
      "os_info": "x86_64 linux unix"
    },
    "configuration": {
      "object_bits": 16,
      "solver": "Cadical"
    },
    "cbmc_stats": {
      "runtime_symex_s": 0.005,
      "runtime_solver_s": 0.0003,
      "vccs_generated": 1,
      "vccs_remaining": 1
    }
  }]
}
```

**8. Coverage** - Coverage configuration
```json
{
  "coverage": {
    "enabled": false
  }
}
```

### Design Notes

- **Harness correlation**: Data is keyed by `harness_id` across blocks (`verification_results.results[]`, `cbmc[]`) for easy filtering
- **Optional fields**: `error_details` only populates `error_type`, `failed_properties_type`, and `exit_status` on failure
- **Complete state**: Captures all verification data including CBMC performance metrics for analysis

### Error Handling

- File write errors produce clear error messages with non-zero exit codes
- Schema validation (during regression testing) catches missing/malformed fields

## Software Design

The implementation touches several components in the Kani driver. We'll describe each component and how they work together to produce the JSON export.

### Core Implementation

The main change is in `kani-driver`, where we add a new frontend module `frontend/schema_utils.rs` that handles all JSON serialization. This module defines a `JsonHandler` struct that collects all the verification data (harness metadata, verification results, CBMC statistics) and serializes it to JSON using standard Rust serialization.

To support this, we enhance `call_cbmc.rs` to extract CBMC performance statistics from CBMC's output. Since CBMC prints timing and statistics information in a structured format, we can parse this using regular expressions to extract values like symbolic execution time, solver time, and VCC counts.

The driver's main entry point (`main.rs`) is modified to accept the `--export-json <filename>` flag. When this flag is present, after verification completes successfully (or fails), we trigger the JSON serialization and write the output to the specified file. File I/O errors are reported clearly to the user with appropriate error messages.

### Schema Validation and Testing

Rather than hardcoding expected JSON fields in our tests, we take a template-based approach. The schema file itself (`kani_json_schema.json`) serves as both documentation and validation template. A Python validation script (`scripts/validate_json_export.py`) reads this schema file and recursively validates that any JSON export matches the expected structure.

The validation approach treats all fields as required by default. However, some fields only make sense in certain contexts—for example, `error_details` contains detailed error information only when verification fails. To handle this, the schema supports `_optional` arrays that list fields which may be absent:

```json
{
  "error_details": {
    "_optional": ["error_type", "failed_properties_type", "exit_status"],
    "has_errors": false,
    "error_type": null
  }
}
```

When validation encounters a field listed in `_optional`, it validates the field only if present in the actual data. This keeps the schema flexible while ensuring we catch missing required fields. Metadata fields (those starting with underscore) are excluded from validation entirely.

The validator supports a `--field-path` option for validating specific sections of the JSON (e.g., the `cbmc` block).

### Test Suite Organization

We add a new test suite under `tests/json-handler/` with four test scenarios that cover different aspects of JSON export:

The `basic-export/` test verifies that the JSON export flag works and produces valid JSON with the expected top-level structure. The `schema-validation/` test is more comprehensive—it runs a verification with multiple harnesses and validates the entire JSON structure against the schema template. This directory also houses the canonical `kani_json_schema.json` file, making it easy to find and update.

The `multiple-harnesses/` test specifically checks that results from multiple harnesses are correctly aggregated and that the `harness_id` correlation works across different blocks. Finally, `failed-verification/` tests that error information is correctly captured and that optional error fields are populated on failure.

### Implementation Flow

When a user runs `cargo kani --export-json output.json`, the following happens:

1. Kani driver parses the command-line arguments and enables JSON export mode
2. During verification, CBMC is invoked for each harness as usual
3. For each harness, we capture and parse CBMC's output to extract statistics
4. After all harnesses complete, we aggregate the results into the `VerificationOutput` struct
5. We serialize this struct to JSON format
6. The JSON is written to the specified file, with any I/O errors reported to the user

The schema structure organizes information by category (metadata, project info, harness details, results, CBMC stats) with `harness_id` serving as the correlation key across blocks. This organization makes it easy for external tools to either process everything or filter by specific harnesses.

### Corner Cases

Several edge cases need consideration:

**CBMC output parsing**: We extract CBMC statistics using regex patterns. If CBMC's output format changes in future versions, parsing may fail. To handle this gracefully, we can omit CBMC statistics in the json schema.

**Large output**: For projects with hundreds of harnesses, JSON files can grow to multi-megabyte sizes. While this is acceptable for current use cases, we may need to consider streaming serialization or compression if users report performance issues.

**Partial verification runs**: If verification is interrupted (user cancellation, system crash), the JSON file may be incomplete or missing entirely. Since JSON is written only after verification completes, interrupted runs produce no output rather than partial/corrupt JSON.

**Schema evolution**: The schema file and the `VerificationOutput` struct must stay synchronized. During development, if we add fields to the struct but forget to update the schema template, our tests will catch this mismatch and fail. This is by design—the tests serve as a contract enforcement mechanism.

## Rationale and alternatives

### Why JSON?

JSON provides universal language support, human readability for debugging, and established tooling without the complexity of binary formats or custom parsers.

### Template-based validation

The schema file (`kani_json_schema.json`) serves as both documentation and validation template. Validation logic reads the schema dynamically rather than hardcoding expected fields.

**Advantage**: Single source of truth; schema changes don't require code changes; prevents drift between documentation and implementation.

**Disadvantage**: More complex validation implementation than hardcoded checks.

## Open questions

- How should we handle breaking schema changes?
- Should users be able to filter which sections are exported?
- For very large projects with hundreds of harnesses, should we support incremental output (e.g., JSONL format)?

## Future possibilities

**Formal JSON Schema**: Generate a formal JSON Schema (Draft 7+) document from our template for automatic client code generation and integration with third-party validators.

**Streaming export**: For projects with hundreds of harnesses, a streaming approach (JSONL format with one harness per line) could reduce memory pressure. Wait for user performance reports before implementing.

**Database integration**: With structured JSON output, users could build tools to store verification results in databases (PostgreSQL) for historical analysis and tracking performance trends. The harness-based organization with `harness_id` keys naturally maps to relational schemas with foreign key relationships.
