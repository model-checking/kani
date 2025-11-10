#!/usr/bin/env python3
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


def validate_structure_recursive(data, schema, path=""):
    """
    Recursively validate data structure against schema template.

    Args:
        data: The JSON data to validate
        schema: The schema template to validate against
        path: Current path in the structure (for error messages)

    Returns:
        (success: bool, errors: list)
    """
    errors = []

    # Handle dict validation
    if isinstance(schema, dict) and isinstance(data, dict):
        # Get optional fields list (fields that may or may not be present)
        optional_fields = schema.get("_optional", [])

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
                data[key], schema[key], current_path
            )[1]
            errors.extend(sub_errors)

    # Handle array validation
    elif isinstance(schema, list) and len(schema) > 0:
        if not isinstance(data, list):
            errors.append(f"Expected array at {path}, got {type(data).__name__}")
        elif len(data) > 0:
            # Validate first item against schema template
            sub_errors = validate_structure_recursive(data[0], schema[0], f"{path}[0]")[
                1
            ]
            errors.extend(sub_errors)

    # Leaf values - no validation needed

    success = len(errors) == 0
    return success, errors


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
    success, all_errors = validate_structure_recursive(data, schema, "")

    if not success or all_errors:
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
            print(f"ERROR: Field path '{field_path}' not found in data")
            return False
        current_data = current_data[part]

        if part not in current_schema:
            print(f"ERROR: Field path '{field_path}' not found in schema template")
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
