#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
"""
JSON Export Validation Script for Kani Integration Tests

Validates a terminal `--export-json` document (RFC 0015) in two steps:

1. Structure, against the schema template `kani_json_schema.json`: every required
   field is present, every scalar has the template's type, and every array element
   has the template's element shape (also when the document's array is empty in
   the template's example).
2. Presence by outcome: the fields RFC 0015 makes depend on a harness's
   `outcome.kind`.

Unknown fields are ignored, as the RFC requires of consumers.

Template syntax:
    "string" | "integer" | "number" | "boolean"   a scalar of that type
    "<type>?"                                     the same, or null
    "enum:A|B"                                    one of the listed strings
    "pattern:<regex>"                             a string matching the whole regex
    "named:<name>"                                a check in NAMED_CHECKS
    "ref:<name>"                                  the template in `_defs`
    {...}                                         an object; `_optional` lists keys that
                                                  may be absent, `_nullable` keys that
                                                  may be null
    [<template>]                                  an array whose elements all match
"""

import json
import re
import sys
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


def type_name(value):
    return "null" if value is None else type(value).__name__


def check_solver(value):
    """`attributes.solver`: a solver name, or {"Binary": "<path>"}. Names are an open set."""
    if isinstance(value, str):
        return None
    if isinstance(value, dict) and isinstance(value.get("Binary"), str):
        return None
    return f'expected a solver name or {{"Binary": <string>}}, got {value!r}'


def check_attribute_kind(value):
    """`attributes.kind`: "Proof", "Test" or {"ProofForContract": {"target_fn": <string>}}."""
    if value in ("Proof", "Test"):
        return None
    if (
        isinstance(value, dict)
        and isinstance(value.get("ProofForContract"), dict)
        and isinstance(value["ProofForContract"].get("target_fn"), str)
    ):
        return None
    return f"expected Proof, Test or a ProofForContract object, got {value!r}"


NAMED_CHECKS = {"solver": check_solver, "attribute_kind": check_attribute_kind}


def check_scalar(value, spec, path, defs, errors):
    if spec.startswith("ref:"):
        check_value(value, defs[spec[len("ref:"):]], path, defs, errors)
        return
    nullable = spec.endswith("?")
    spec = spec[:-1] if nullable else spec
    if value is None:
        if not nullable:
            errors.append(f"{path}: null is not allowed (expected {spec})")
        return
    kind, _, argument = spec.partition(":")
    problem = None
    if kind == "string":
        ok = isinstance(value, str)
    elif kind == "integer":
        ok = isinstance(value, int) and not isinstance(value, bool)
    elif kind == "number":
        ok = isinstance(value, (int, float)) and not isinstance(value, bool)
    elif kind == "boolean":
        ok = isinstance(value, bool)
    elif kind == "enum":
        ok = isinstance(value, str) and value in argument.split("|")
    elif kind == "pattern":
        ok = isinstance(value, str) and re.fullmatch(argument, value) is not None
    elif kind == "named":
        problem = NAMED_CHECKS[argument](value)
        ok = problem is None
    else:
        raise ValueError(f"unknown template type {spec!r} at {path}")
    if not ok:
        problem = problem or f"expected {spec}, got {type_name(value)} {value!r}"
        errors.append(f"{path}: {problem}")


def check_value(value, spec, path, defs, errors):
    """Append every mismatch between `value` and the template `spec` to `errors`."""
    if isinstance(spec, str):
        check_scalar(value, spec, path, defs, errors)
    elif isinstance(spec, dict):
        if not isinstance(value, dict):
            errors.append(
                f"{path or '<root>'}: expected object, got {type_name(value)}")
            return
        optional = spec.get("_optional", [])
        nullable = spec.get("_nullable", [])
        for key, sub_spec in spec.items():
            if key.startswith("_"):
                continue
            sub_path = f"{path}.{key}" if path else key
            if key not in value:
                if key not in optional:
                    errors.append(f"Missing required field: {sub_path}")
            elif value[key] is not None or key not in nullable:
                check_value(value[key], sub_spec, sub_path, defs, errors)
    elif isinstance(spec, list):
        if not isinstance(value, list):
            errors.append(f"{path}: expected array, got {type_name(value)}")
            return
        for index, item in enumerate(value):
            check_value(item, spec[0], f"{path}[{index}]", defs, errors)


def check_harness_presence(harness, template, path, errors):
    """The RFC 0015 presence matrix for one harness, by `outcome.kind`."""
    outcome = harness["outcome"]
    kind = outcome["kind"]
    completed = kind == "COMPLETED"
    crashed = kind == "CRASHED"

    def require(condition, message):
        if not condition:
            errors.append(f"{path}: {message}")

    require(("verdict" in outcome) == completed,
            f"outcome.verdict must be present exactly on COMPLETED (kind {kind})")
    require(("failure_kind" in harness) == completed,
            f"failure_kind must be present exactly on COMPLETED (kind {kind})")
    require(("code" in outcome) == crashed,
            f"outcome.code must be present exactly on CRASHED (kind {kind})")
    require(("message" in outcome) == crashed,
            f"outcome.message must be present exactly on CRASHED (kind {kind})")

    totals = {
        "n_properties": harness["n_properties"],
        "n_failed": harness["n_failed"],
        "checks.total": harness["checks"]["total"],
        "checks.success": harness["checks"]["success"],
        "covers.total": harness["covers"]["total"],
    }
    for name, value in totals.items():
        if completed:
            require(value is not None, f"{name} must be an integer on COMPLETED")
        else:
            require(value is None, f"{name} must be null on {kind}, got {value!r}")

    if not completed:
        lists = {
            "failed_properties": harness["failed_properties"],
            "unsupported_constructs": harness["unsupported_constructs"],
        }
        for bucket in ("checks", "covers"):
            for name, spec in template[bucket].items():
                if isinstance(spec, list):
                    lists[f"{bucket}.{name}"] = harness[bucket][name]
        for name, value in lists.items():
            require(value == [], f"{name} must be [] on {kind}, got {value!r}")
    elif "verdict" in outcome and "failure_kind" in harness:
        # RFC 0015, "Value domains": the verdict follows from `should_panic` and
        # `failure_kind`.
        should_panic = harness["attributes"]["should_panic"]
        passing_kind = "PANICS_ONLY" if should_panic else "NONE"
        expected = "SUCCESS" if harness["failure_kind"] == passing_kind else "FAILURE"
        require(
            outcome["verdict"] == expected,
            f"outcome.verdict {outcome['verdict']} does not follow from "
            f"should_panic={should_panic} and failure_kind={harness['failure_kind']}",
        )


def validate_document(data, schema):
    """Return the list of problems with a terminal export document (empty if valid)."""
    errors = []
    check_value(data, schema, "", schema.get("_defs", {}), errors)
    if errors:
        return errors
    for key in ("verdict", "code", "message"):
        if key in data["outcome"]:
            errors.append(f"outcome.{key}: the run-level outcome carries only a kind")
    for index, harness in enumerate(data["harnesses"]):
        check_harness_presence(
            harness, schema["harnesses"][0], f"harnesses[{index}]", errors)
    return errors


def validate_json_structure(json_file, schema=None):
    """
    Validate that JSON export matches the schema template and the presence rules.
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

    all_errors = validate_document(data, schema)
    if all_errors:
        print(f"ERROR: Validation failed for {json_file}:")
        for error in all_errors:
            print(f"  - {error}")
        return False

    print(f"JSON structure validation passed for {json_file}")
    return True


def validate_field_path(json_file, field_path, schema=None):
    """
    Validate the structure of the fields at a given path.

    Args:
        json_file: Path to JSON file
        field_path: Dot-separated path (e.g., 'tools', 'summary')
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

    errors = []
    defs = schema.get("_defs", {})
    check_value(current_data, current_schema, field_path, defs, errors)
    if errors:
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
