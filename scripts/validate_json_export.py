#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
"""
JSON Export Validation Script for Kani Integration Tests

Validates JSON exports against the kani_json_schema.json template.
"""

import json
import sys
import os
from pathlib import Path


def load_schema_template():
    """Load the JSON schema template"""
    # Find schema template in tests/json-handler/schema-validation directory
    script_dir = Path(__file__).parent
    schema_path = (
        script_dir.parent
        / "tests"
        / "json-handler"
        / "schema-validation"
        / "kani_json_schema.json"
    )

    if not schema_path.exists():
        print(f"ERROR: Schema template not found at {schema_path}")
        return None

    with open(schema_path, "r") as f:
        return json.load(f)


def validate_structure_recursive(data, schema, path="", nullable=False):
    """
    Recursively validate data structure against schema template.

    Args:
        data: The JSON data to validate
        schema: The schema template to validate against
        path: Current path in the structure (for error messages)
        nullable: Whether the parent declared this value may be null, via its
            `_nullable` list. Only such values are allowed to be null where the
            template expects an object or an array.

    Returns:
        (success: bool, errors: list)
    """
    errors = []

    # A null where the template expects a structure is only acceptable if the parent
    # declared it nullable; otherwise it is reported below as a type mismatch.
    if data is None and nullable:
        return True, errors

    # Handle dict validation
    if isinstance(schema, dict):
        if not isinstance(data, dict):
            # This used to fall through to the leaf case and report success, so an export
            # with `"metadata": null` bypassed every field required beneath it.
            errors.append(
                f"Expected object at {path or '<root>'}, "
                f"got {type(data).__name__}"
            )
            return False, errors

        # Get optional fields list (fields that may or may not be present)
        optional_fields = schema.get("_optional", [])
        # Fields whose value may legitimately be null instead of a structure.
        nullable_fields = schema.get("_nullable", [])

        # All fields in schema are required except metadata fields (starting with _) and optional fields
        schema_keys = [k for k in schema.keys() if not k.startswith("_")]

        # Check each key in schema
        for key in schema_keys:
            # Check if field exists in data
            if key not in data:
                # Only report error if field is required (not in optional list)
                if key not in optional_fields:
                    current_path = f"{path}.{key}" if path else key
                    errors.append(f"Missing required field: {current_path}")
                continue

            # Recursively validate nested structure
            current_path = f"{path}.{key}" if path else key
            sub_errors = validate_structure_recursive(
                data[key], schema[key], current_path, key in nullable_fields
            )[1]
            errors.extend(sub_errors)

    # Handle array validation
    elif isinstance(schema, list) and len(schema) > 0:
        if not isinstance(data, list):
            errors.append(f"Expected array at {path}, got {type(data).__name__}")
        else:
            # Validate every item against the schema template. Checking only the first
            # element let malformed data in every later harness pass validation, which
            # defeats the purpose on exactly the multi-harness runs this validates.
            #
            # An empty array is *not* treated as an error here: some arrays are legitimately
            # empty (e.g. `checks` for a harness with no properties). What used to make an empty
            # array a vacuous pass was that it skips the loop below with zero iterations and
            # zero errors -- exactly like a real, correctly-validated array. The distinction
            # this function draws is between "validated everything present" (this) and
            # "nothing was present to check" (a caller-level semantic question, e.g. whether
            # `results` should be empty -- see `validate_semantic_checks`).
            for index, item in enumerate(data):
                sub_errors = validate_structure_recursive(
                    item, schema[0], f"{path}[{index}]"
                )[1]
                errors.extend(sub_errors)

    # Leaf values. A `None` schema leaf means the template does not commit to a type at this
    # position (e.g. `contract.contracted_function_name`, which is only ever populated from real
    # data) -- there's nothing to check against, so it is left unvalidated exactly as before.
    # A non-`None` schema leaf, on the other hand, is a definite type (bool / number / string),
    # and the data must match it. Before this, `"failed": -100`, `"successful": "yes"`, or any
    # other leaf of the wrong type validated as OK, because leaves were never inspected at all.
    elif schema is not None:
        if isinstance(schema, bool):
            # Must come before the `int`/`float` branch: in Python, `bool` is a subclass of
            # `int`, so `isinstance(True, int)` is true and would otherwise let a bool through
            # a numeric check (and vice versa below).
            if not isinstance(data, bool):
                errors.append(
                    f"Type mismatch at {path}: expected bool, got {type(data).__name__}"
                )
        elif isinstance(schema, (int, float)):
            if isinstance(data, bool) or not isinstance(data, (int, float)):
                errors.append(
                    f"Type mismatch at {path}: expected a number, got {type(data).__name__}"
                )
        elif isinstance(schema, str):
            if not isinstance(data, str):
                errors.append(
                    f"Type mismatch at {path}: expected str, got {type(data).__name__}"
                )

    success = len(errors) == 0
    return success, errors


def validate_semantic_checks(data):
    """
    A small set of semantic checks on `verification_results.summary` beyond structure and leaf
    types: a document can be perfectly well-typed and still lie about what happened, e.g.
    `executed` disagreeing with the number of results actually reported, or a negative count that
    is well-typed (a valid JSON integer) but meaningless. Kept deliberately narrow -- this is not
    a general semantic validator, just the reconciliation checks a mutated count can defeat.

    Returns a list of error strings; an empty list means the semantic checks found nothing wrong.
    """
    errors = []

    verification_results = data.get("verification_results")
    if not isinstance(verification_results, dict):
        # Already reported (or not applicable) by the structural check; nothing more to say here.
        return errors

    summary = verification_results.get("summary")
    results = verification_results.get("results")
    if not isinstance(summary, dict) or not isinstance(results, list):
        return errors

    def as_int(value):
        # Excludes `bool`: a `True`/`False` count is a type error the structural check already
        # reports, not a value this function should reason about numerically.
        return value if isinstance(value, int) and not isinstance(value, bool) else None

    executed = as_int(summary.get("executed"))
    successful = as_int(summary.get("successful"))
    failed = as_int(summary.get("failed"))

    for field in ("total_harnesses", "executed", "successful", "failed", "duration_ms"):
        value = as_int(summary.get(field))
        if value is not None and value < 0:
            errors.append(
                f"verification_results.summary.{field} is negative: {value}"
            )

    if executed is not None and executed != len(results):
        errors.append(
            f"verification_results.summary.executed ({executed}) does not match "
            f"the number of entries in verification_results.results ({len(results)})"
        )

    if executed is not None and successful is not None and failed is not None:
        if successful + failed != executed:
            errors.append(
                f"verification_results.summary.successful ({successful}) + "
                f"failed ({failed}) does not equal executed ({executed})"
            )

    # `run_state` is the field a consumer is meant to trust as the run's bottom line, so its
    # value must both be one of the states the exporter actually emits, and must agree with what
    # the rest of the document says happened -- otherwise a mislabeled `run_state` is a more
    # dangerous lie than no `run_state` at all.
    run_state = data.get("run_state")
    if run_state is not None:
        valid_run_states = {"incomplete", "no_harnesses_selected", "partial", "complete"}
        if run_state not in valid_run_states:
            errors.append(
                f"run_state has an unrecognized value: {run_state!r} "
                f"(expected one of {sorted(valid_run_states)})"
            )
        elif run_state == "complete":
            if len(results) == 0:
                errors.append(
                    'run_state is "complete" but verification_results.results is empty'
                )
            # `executed == len(results)` is already enforced unconditionally above, so a
            # "complete" run that disagrees is already reported there.
        elif run_state == "no_harnesses_selected":
            if executed is not None and executed != 0:
                errors.append(
                    'run_state is "no_harnesses_selected" but '
                    f"verification_results.summary.executed is {executed}, expected 0"
                )
            if len(results) != 0:
                errors.append(
                    'run_state is "no_harnesses_selected" but '
                    f"verification_results.results is not empty (len={len(results)})"
                )

    return errors


def validate_json_structure(json_file, schema=None):
    """
    Validate that JSON export matches the schema template structure.
    """
    try:
        with open(json_file, "r") as f:
            data = json.load(f)
    except FileNotFoundError:
        print(f"ERROR: JSON file {json_file} not found")
        return False
    except json.JSONDecodeError as e:
        print(f"ERROR: Invalid JSON in {json_file}: {e}")
        return False

    # Load schema if not provided
    if schema is None:
        schema = load_schema_template()
        if schema is None:
            return False

    # All schema fields are required - validate structure recursively
    # The recursive validator will catch any missing required fields
    _, structural_errors = validate_structure_recursive(data, schema, "")

    # Structural/type validation and semantic validation are independent: run both and report
    # everything found, rather than short-circuiting on the first kind of failure.
    all_errors = structural_errors + validate_semantic_checks(data)

    if all_errors:
        print(f"ERROR: Validation failed for {json_file}:")
        for error in all_errors:
            print(f"  - {error}")
        return False

    print(f"JSON structure validation passed for {json_file}")
    return True


def validate_field_path(json_file, field_path, schema=None):
    """
    Validate specific fields at a given path.

    Args:
        json_file: Path to JSON file
        field_path: Dot-separated path (e.g., 'metadata', 'verification_results.summary')
        schema: Optional pre-loaded schema
    """
    try:
        with open(json_file, "r") as f:
            data = json.load(f)
    except Exception as e:
        print(f"ERROR: Failed to load {json_file}: {e}")
        return False

    # Load schema if not provided
    if schema is None:
        schema = load_schema_template()
        if schema is None:
            return False

    # Navigate to the field in both data and schema
    parts = field_path.split(".")
    current_data = data
    current_schema = schema

    for part in parts:
        if part not in current_data:
            print(
                f"ERROR: Field path '{field_path}' not found in data. Missing part: '{part}'"
            )
            return False
        current_data = current_data[part]

        if part not in current_schema:
            print(
                f"ERROR: Field path '{field_path}' not found in schema template. Missing part: '{part}'"
            )
            return False
        current_schema = current_schema[part]

        # Handle arrays - check first item
        if isinstance(current_schema, list) and len(current_schema) > 0:
            current_schema = current_schema[0]
            if isinstance(current_data, list) and len(current_data) > 0:
                current_data = current_data[0]

    # Validate structure at this path
    success, errors = validate_structure_recursive(
        current_data, current_schema, field_path
    )

    if not success:
        print(f"ERROR: Validation failed for {field_path}:")
        for error in errors:
            print(f"  - {error}")
        return False

    print(f"Field validation passed for {field_path}")
    return True


def main():
    if len(sys.argv) < 2:
        print(
            "Usage: python3 validate_json_export.py <json_file> [--field-path <path>]"
        )
        sys.exit(1)

    json_file = sys.argv[1]

    # Check if specific field validation requested
    if len(sys.argv) > 2 and sys.argv[2] == "--field-path":
        if len(sys.argv) < 4:
            print("ERROR: --field-path requires a path argument")
            sys.exit(1)

        field_path = sys.argv[3]
        if validate_field_path(json_file, field_path):
            sys.exit(0)
        else:
            sys.exit(1)

    # Load schema once
    schema = load_schema_template()
    if schema is None:
        print("ERROR: Could not load schema template")
        sys.exit(1)

    # Run full validation
    if validate_json_structure(json_file, schema):
        print(f"\nAll validations passed for {json_file}")
        sys.exit(0)
    else:
        print(f"\nValidation failed for {json_file}")
        sys.exit(1)


if __name__ == "__main__":
    main()
