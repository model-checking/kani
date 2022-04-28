#!/usr/bin/env python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT


"""CBMC JSON Parser

This script allows kani to print to the console the final output displayed to the user
after CBMC gives the response JSON object.

This script accepts JSON files.

This script requires that `colorama` be installed within the Python
environment you are running this script in.

This file can also be imported as a module and contains the following
functions:

    * transform_cbmc_output - returns the formatted string output after parsing
    * main - the main function of the script
"""

import json
import os
import re
import sys

from colorama import Fore, Style
from enum import Enum
from os import path

# Enum to store the style of output that is given by the argument flags
output_style_switcher = {
    'default': 'regular',
    'regular': 'regular',
    'terse': 'terse',
    'old': 'old'
}

class GlobalMessages(str, Enum):
    """
    Enum class to store all the global messages
    """
    CONST_TRACE_MESSAGE = 'Building error trace',
    PROGRAM = 'program'
    RESULT = 'result'
    MESSAGE_TEXT = 'messageText'
    MESSAGE_TYPE = 'messageType'
    SUCCESS = 'SUCCESS'
    FAILED = 'FAILED'
    REACH_CHECK_DESC = "[KANI_REACHABILITY_CHECK]"
    REACH_CHECK_KEY = "reachCheckResult"
    CHECK_ID = "KANI_CHECK_ID"
    ASSERTION_FALSE = "assertion false"
    DEFAULT_ASSERTION = "assertion"
    CHECK_ID_RE = CHECK_ID + r"_.*_([0-9])*"
    UNSUPPORTED_CONSTRUCT_DESC = "is not currently supported by Kani"
    UNWINDING_ASSERT_DESC = "unwinding assertion loop"


def usage_error(msg):
    """ Prints an error message followed by the expected usage. Then exit process. """
    print(f"Error: {msg} Usage:")
    print("cbmc_json_parser.py <cbmc_output.json> <format> [--extra-ptr-check]")
    sys.exit(1)


def main(argv):
    """
    Script main function.
    Usage:
      > cbmc_json_parser.py <cbmc_output.json> <format> [--extra-ptr-check]
    """
    # We expect [3, 4] arguments.
    if len(argv) < 3:
        usage_error("Missing required arguments.")

    max_args = 4
    if len(argv) > max_args:
        usage_error(f"Expected up to {max_args} arguments but found {len(argv)}.")

    output_style = output_style_switcher.get(argv[2], None)
    if not output_style:
        usage_error(f"Invalid output format '{argv[2]}'.")

    extra_ptr_check = False
    if len(argv) == 4:
        if argv[3] == "--extra-ptr-check":
            extra_ptr_check = True
        else:
            usage_error(f"Unexpected argument '{argv[3]}'.")

    # parse the input json file
    with open(argv[1]) as f:
        sample_json_file_parsing = f.read()

    # the main function should take a json file as input
    return_code = transform_cbmc_output(sample_json_file_parsing,
                                        output_style, extra_ptr_check)
    sys.exit(return_code)


class SourceLocation:
    def __init__(self, source_location={}):
        """ Convert a source location dictionary from CBMC json into an object.
           The SOURCE_LOCATION_OBJECT has the following structure:
           {
             'column': '<num>',
             'file': '<file_name>',
             'function': '<fn_name>',
             'line': '<num>'
           }},

           Some fields might be missing.
        """
        self.filename = source_location.get("file", None)
        self.function = source_location.get("function", None)
        self.column = source_location.get("column", None)
        self.line = source_location.get("line", None)

    def filepath(self):
        """ Return a more user friendly path.

        - If the path is inside the current directory, return a relative path.
        - If not but the path is in the ${HOME} directory, return relative to ${HOME}
        - Otherwise, return the path as is.
        """
        if not self.filename:
            return None

        # Reference to C files use relative paths, while rust uses absolute.
        # Normalize both to be absolute first.
        full_path = path.abspath(self.filename)
        cwd = os.getcwd()
        if path.commonpath([full_path, cwd]) == cwd:
            return path.relpath(full_path)

        home_path = path.expanduser("~")
        if path.commonpath([full_path, home_path]) == home_path:
            return "~/{}".format(path.relpath(full_path, home_path))

        return self.filename

    def __str__(self):
        if self.filename:
            s = f"{self.filepath()}"
            if self.line:
                s += f":{self.line}"
                if self.column:
                    s += f":{self.column}"
        else:
            s = "Unknown File"
        if self.function:
            s += f" in function {self.function}"
        return s

    def __bool__(self):
        return bool(self.function) or bool(self.filename)


def transform_cbmc_output(cbmc_response_string, output_style, extra_ptr_check):
    """
    Take Unstructured CBMC Response object, parse the blob and gives structured
    and formatted output depending on User Provided Output Style

    Parameters -
        cbmc_response_string : str
            A response blob that is given to the function from CBMC
        output_style : int
            An index to tell the script which style of output is requested by the User.

    Returns -
        None
            Prints the final output string to the user
    """

    # Output Message is the final output that is printed to the user
    output_message = ""

    # Check if the output given by CBMC is in Json format or not
    is_json_bool, cbmc_json_array = parse_json(cbmc_response_string)
    if is_json_bool:

        # Extract property information from the restructured JSON file
        properties, solver_information = extract_solver_information(cbmc_json_array)

        # Check if there were any errors
        errors = extract_errors(solver_information)
        if errors:
            print('\n'.join(errors))
            return 1

        properties, messages = postprocess_results(properties, extra_ptr_check)

        # Using Case Switching to Toggle between various output styles
        # For now, the two options provided are default and terse
        if output_style == output_style_switcher["regular"]:

            # Extract Solver Information from the json file and construct message
            output_message += construct_solver_information_message(solver_information)

            # Extract property messages from the json file
            property_message, num_failed = construct_property_message(properties)
            # Combine both messages to give as final output
            output_message += property_message

        elif output_style == output_style_switcher["terse"]:

            # Construct only summarized result and display output
            output_message, num_failed = construct_terse_property_message(properties)

        # Print using an Interface function
        print(output_message)
        print(messages)

    else:
        # When CBMC crashes or does not produce json output, print the response
        # string to allow us to debug
        print(cbmc_response_string)
        raise Exception("CBMC Crashed - Unable to present Result")

    return num_failed > 0

# Check if the blob is in json format for parsing
def parse_json(cbmc_output_string):
    try:
        cbmc_json_array = json.loads(cbmc_output_string)
    except ValueError:
        return False, None
    return True, cbmc_json_array

def extract_solver_information(cbmc_response_json_array):
    """
    Takes the CBMC Response, now in JSON Array format and extracts solver and property information
    and splits them into two seperate lists.

    Parameters -
        cbmc_response_json_array : JSON Array
            A response JSON Array that contains the Result Object from CBMC and
            Solver Information

            Input Example -
                [{'program': 'CBMC 5.44.0 (cbmc-5.44.0)'},
                {'messageText': 'CBMC version 5.44.0 (cbmc-5.44.0) 64-bit x86_64 linux', 'messageType': 'STATUS-MESSAGE'},
                {'messageText': 'Reading GOTO program from file', 'messageType': 'STATUS-MESSAGE'},
                ...
                {'result': [{'description': 'assertion failed: 2 == 4', 'property': 'main.assertion.1', 'status': 'FAILURE', '
                trace': [{'function': {'displayName': '__CPROVER_initialize', 'identifier': '__CPROVER_initialize',
                'sourceLocation': {'file': '<built-in-additions>', 'line': '40', 'workingDirectory': '/home/ubuntu'}},
                ...'thread': 0}]}

    Returns -
        properties : List
            Contains the list of properties that is obtained from the "result" object from CBMC.

        solver_information : List
            Contains the list of message texts which collectively contain information about the solver.

    """

    # Solver information is all the fields in the json object which are not related to the results
    solver_information = []
    properties = None

    # Parse each object and extract out the "result" object to be returned seperately
    for response_object in cbmc_response_json_array:
        """Example response object -
        1)  {'program': 'CBMC 5.44.0 (cbmc-5.44.0)'},
        2)  {'result': [{'description': 'assertion failed: 2 == 4',..}
        """
        if GlobalMessages.RESULT in response_object.keys():
            properties = response_object["result"]
        else:
            solver_information.append(response_object)

    return properties, solver_information

def resolve_unknown_location_checks(properties):
    """
    1. Searches for any check which has unknown file location or missing defition and replaces description
    2. If any of these checks has a failure status, then we turn all the sucesses into undetermined.
    """
    has_unknown_location_checks = False
    for property in properties:
        if GlobalMessages.ASSERTION_FALSE in property["description"] and extract_property_class(property) == GlobalMessages.DEFAULT_ASSERTION and not hasattr(property["sourceLocation"], "file"):
            property["description"] = "Function with missing definition is unreachable"
            if property["status"] == "FAILURE":
                has_unknown_location_checks = True
    return has_unknown_location_checks

def extract_errors(solver_information):
    """
    Extract errors from the CBMC output, which are messages that have the
    message type 'ERROR'
    """
    errors = []
    for message in solver_information:
        if GlobalMessages.MESSAGE_TYPE in message and message[GlobalMessages.MESSAGE_TYPE] == 'ERROR':
            error_message = message[GlobalMessages.MESSAGE_TEXT]
            # Replace "--object bits n" with "--enable-unstable --cbmc-args
            # --object bits n" in the message
            if 'use the `--object-bits n` option' in error_message:
                error_message = error_message.replace("--object-bits ", "--enable-unstable --cbmc-args --object-bits ")
            errors.append(error_message)
    return errors

def postprocess_results(properties, extra_ptr_check):
    """
    Check for certain cases, e.g. a reachable unsupported construct or a failed
    unwinding assertion, and update the results of impacted checks accordingly.
    1. Change all "SUCCESS" results to "UNDETERMINED" if the reachability check
    for a Rust construct that is not currently supported by Kani failed, since
    the missing exploration of execution paths through the unsupported construct
    may hide failures
    2. Change a check's result from "SUCCESS" to "UNREACHABLE" if its
    reachability check's result was "SUCCESS"
    3. TODO: Change results from "SUCCESS" to "UNDETERMINED" if an unwinding
    assertion failed, since the insufficient unwinding may cause some execution
    paths to be left unexplored (https://github.com/model-checking/kani/issues/746)

    Additionally, print a message at the end of the output that indicates if any
    of the special cases above was hit.
    """

    has_reachable_unsupported_constructs = has_check_failure(properties, GlobalMessages.UNSUPPORTED_CONSTRUCT_DESC)
    has_failed_unwinding_asserts = has_check_failure(properties, GlobalMessages.UNWINDING_ASSERT_DESC)
    has_unknown_location_asserts = resolve_unknown_location_checks(properties)
    properties, reach_checks = filter_reach_checks(properties)
    annotate_properties_with_reach_results(properties, reach_checks)
    remove_check_ids_from_description(properties)

    if not extra_ptr_check:
        properties = filter_ptr_checks(properties)

    for property in properties:
        property["description"] = get_readable_description(property)
        if has_reachable_unsupported_constructs or has_failed_unwinding_asserts or has_unknown_location_asserts:
            # Change SUCCESS to UNDETERMINED for all properties
            if property["status"] == "SUCCESS":
                property["status"] = "UNDETERMINED"
        elif GlobalMessages.REACH_CHECK_KEY in property and property[GlobalMessages.REACH_CHECK_KEY] == "SUCCESS":
            # Change SUCCESS to UNREACHABLE
            description = property["description"]
            assert property[
                "status"] == "SUCCESS", f"** ERROR: Expecting the unreachable property \"{description}\" to have a status of \"SUCCESS\""
            property["status"] = "UNREACHABLE"

    messages = ""
    if has_reachable_unsupported_constructs:
        messages += "** WARNING: A Rust construct that is not currently supported " \
                    "by Kani was found to be reachable. Check the results for " \
                    "more details."
    if has_failed_unwinding_asserts:
        messages += "[Kani] info: Verification output shows one or more unwinding failures.\n" \
                    "[Kani] tip: Consider increasing the unwinding value or disabling `--unwinding-assertions`.\n"

    return properties, messages


def has_check_failure(properties, message):
    """
    Search in properties for a failed property with the given message
    """
    for property in properties:
        if message in property["description"] and property["status"] == "FAILURE":
            return True
    return False


def filter_reach_checks(properties):
    return filter_properties(properties, GlobalMessages.REACH_CHECK_DESC)


def filter_properties(properties, message):
    """
    Move properties that have "message" in their description out of "properties"
    into "removed_properties"
    """
    filtered_properties = []
    removed_properties = []
    for property in properties:
        if message in property["description"]:
            removed_properties.append(property)
        else:
            filtered_properties.append(property)
    return filtered_properties, removed_properties

class CProverCheck:
    """ Represents a CProverCheck and provides methods to replace the check.

        Objects of this class are used to represent specific types of CBMC's
        check messages. It allow us to identify and to replace them by a more
        user friendly message.

        That includes rewriting them and removing information that don't
        make sense in the ust context. E.g.:
        - Original CBMC message: "dead object in OBJECT_SIZE(&temp_0)"
                     Not in the original code -> ^^^^^^^^^^^^^^^^^^^^
        - New message: "pointer to dead object"
    """

    def __init__(self, msg, new_msg=None):
        self.original = msg
        self.kani_msg = new_msg if new_msg else msg

    def matches(self, msg):
        return self.original in msg

    def replace(self, msg):
        return self.kani_msg


CBMC_DESCRIPTIONS = {
    "error_label": [],
    "division-by-zero": [CProverCheck("division by zero")],
    "enum-range-check": [CProverCheck("enum range check")],
    "undefined-shift": [CProverCheck("shift distance is negative"),
                        CProverCheck("shift distance too large"),
                        CProverCheck("shift operand is negative"),
                        CProverCheck("shift of non-integer type")],
    "overflow": [
        CProverCheck("result of signed mod is not representable"),
        CProverCheck("arithmetic overflow on signed type conversion"),
        CProverCheck("arithmetic overflow on signed division"),
        CProverCheck("arithmetic overflow on signed unary minus"),
        CProverCheck("arithmetic overflow on signed shl"),
        CProverCheck("arithmetic overflow on unsigned unary minus"),
        CProverCheck("arithmetic overflow on signed +", "arithmetic overflow on signed addition"),
        CProverCheck("arithmetic overflow on signed -", "arithmetic overflow on signed subtraction"),
        CProverCheck("arithmetic overflow on signed *", "arithmetic overflow on signed multiplication"),
        CProverCheck("arithmetic overflow on unsigned +", "arithmetic overflow on unsigned addition"),
        CProverCheck("arithmetic overflow on unsigned -", "arithmetic overflow on unsigned subtraction"),
        CProverCheck("arithmetic overflow on unsigned *", "arithmetic overflow on unsigned multiplication"),
        CProverCheck("arithmetic overflow on floating-point typecast"),
        CProverCheck("arithmetic overflow on floating-point division"),
        CProverCheck("arithmetic overflow on floating-point addition"),
        CProverCheck("arithmetic overflow on floating-point subtraction"),
        CProverCheck("arithmetic overflow on floating-point multiplication"),
        CProverCheck("arithmetic overflow on unsigned to signed type conversion"),
        CProverCheck("arithmetic overflow on float to signed integer type conversion"),
        CProverCheck("arithmetic overflow on signed to unsigned type conversion"),
        CProverCheck("arithmetic overflow on unsigned to unsigned type conversion"),
        CProverCheck("arithmetic overflow on float to unsigned integer type conversion")],
    "NaN": [
        CProverCheck("NaN on +", "NaN on addition"),
        CProverCheck("NaN on -", "NaN on subtraction"),
        CProverCheck("NaN on /", "NaN on division"),
        CProverCheck("NaN on *", "NaN on multiplication")],
    "pointer": [
        CProverCheck("same object violation")],
    "pointer_arithmetic": [
        CProverCheck("pointer relation: deallocated dynamic object"),
        CProverCheck("pointer relation: dead object"),
        CProverCheck("pointer relation: pointer NULL"),
        CProverCheck("pointer relation: pointer invalid"),
        CProverCheck("pointer relation: pointer outside dynamic object bounds"),
        CProverCheck("pointer relation: pointer outside object bounds"),
        CProverCheck("pointer relation: invalid integer address"),
        CProverCheck("pointer arithmetic: deallocated dynamic object"),
        CProverCheck("pointer arithmetic: dead object"),
        CProverCheck("pointer arithmetic: pointer NULL"),
        CProverCheck("pointer arithmetic: pointer invalid"),
        CProverCheck("pointer arithmetic: pointer outside dynamic object bounds"),
        CProverCheck("pointer arithmetic: pointer outside object bounds"),
        CProverCheck("pointer arithmetic: invalid integer address")],
    "pointer_dereference": [
        CProverCheck("dereferenced function pointer must be", "dereference failure: invalid function pointer"),
        CProverCheck("dereference failure: pointer NULL"),
        CProverCheck("dereference failure: pointer invalid"),
        CProverCheck("dereference failure: deallocated dynamic object"),
        CProverCheck("dereference failure: dead object"),
        CProverCheck("dereference failure: pointer outside dynamic object bounds"),
        CProverCheck("dereference failure: pointer outside object bounds"),
        CProverCheck("dereference failure: invalid integer address")],
    "pointer_primitives": [
        # These are very hard to understand without more context.
        CProverCheck("pointer invalid"),
        CProverCheck("deallocated dynamic object", "pointer to deallocated dynamic object"),
        CProverCheck("dead object", "pointer to dead object"),
        CProverCheck("pointer outside dynamic object bounds"),
        CProverCheck("pointer outside object bounds"),
        CProverCheck("invalid integer address")
    ],
    "array_bounds": [
        CProverCheck("lower bound", "index out of bounds"),  # Irrelevant check. Only usize allowed as index.
        # This one is redundant:
        # CProverCheck("dynamic object upper bound", "access out of bounds"),
        CProverCheck("upper bound", "index out of bounds: the length is less than or equal to the given index"), ],
    "bit_count": [
        CProverCheck("count trailing zeros is undefined for value zero"),
        CProverCheck("count leading zeros is undefined for value zero")],
    "memory-leak": [
        CProverCheck("dynamically allocated memory never freed")],
    # These pre-conditions should not print temporary variables since they are embedded in the libc implementation.
    # They are added via __CPROVER_precondition.
    # "precondition_instance": [],
}


def extract_property_class(prop):
    """
    This function extracts the property class from the property string.
    Property strings have the format of -([<function>.]<property_class_id>.<counter>)
    """
    prop_class = prop["property"].rsplit(".", 3)
    # Do nothing if prop_class is diff than cbmc's convention
    class_id = prop_class[-2] if len(prop_class) > 1 else None
    return class_id


def filter_ptr_checks(props):
    """This function will filter out extra pointer checks.

        Our support to primitives and overflow pointer checks is unstable and
        can result in lots of spurious failures. By default, we filter them out.
    """
    def not_extra_check(prop):
        return extract_property_class(prop) not in ["pointer_arithmetic", "pointer_primitives"]

    return list(filter(not_extra_check, props))


def get_readable_description(prop):
    """This function will return a user friendly property description.

       For CBMC checks, it will ensure that the failure does not include any
       temporary variable.
    """
    original = prop["description"]
    class_id = extract_property_class(prop)
    if class_id in CBMC_DESCRIPTIONS:
        # Contains a list for potential message translation [String].
        prop_type = [check.replace(original) for check in CBMC_DESCRIPTIONS[class_id] if check.matches(original)]
        if len(prop_type) != 1:
            if "KANI_FAIL_ON_UNEXPECTED_DESCRIPTION" in os.environ:
                print(f"Unexpected description: {original}\n"
                      f"  - class_id: {class_id}\n"
                      f"  - matches: {prop_type}\n")
                exit(1)
            else:
                return original
        else:
            return prop_type[0]
    return original

def annotate_properties_with_reach_results(properties, reach_checks):
    """
    When assertion reachability checks are turned on, kani prefixes each
    assert's description with an "ID" of the following form:
    [KANI_CHECK_ID_<crate-name>_<index-of-check>]
    e.g.:
    [KANI_CHECK_ID_foo.6875c808::foo_0] assertion failed: x % 2 == 0
    In addition, the description of each reachability check that it generates
    includes the ID of the assert that we want to check its reachability. The
    description of a reachability check uses the following template:
    [KANI_REACHABILITY_CHECK] <ID of original assert>
    e.g.:
    [KANI_REACHABILITY_CHECK] KANI_CHECK_ID_foo.6875c808::foo_0
    This function iterates over the reachability checks, and for each:
    1. It finds the corresponding assert through matching the ID
    2. It annotates the assert's data with the result of the reachability check
    under the GlobalMessages.REACH_CHECK_KEY key
    """
    for reach_check in reach_checks:
        description = reach_check["description"]
        # Extract the ID of the assert from the description
        match_obj = re.search(GlobalMessages.CHECK_ID_RE, description)
        if not match_obj:
            raise Exception("Error: failed to extract check ID for reachability check \"" + description + "\"")
        check_id = match_obj.group(0)
        prop = get_matching_property(properties, check_id)
        # Attach the result of the reachability check to this property
        prop[GlobalMessages.REACH_CHECK_KEY] = reach_check["status"]


def get_matching_property(properties, check_id):
    """
    Find the property with the given ID
    """
    for property in properties:
        description = property["description"]
        match_obj = re.search("\\[" + GlobalMessages.CHECK_ID_RE + "\\]", description)
        # Currently, not all properties have a check ID
        if match_obj:
            prop_check_id = match_obj.group(0)
            if prop_check_id == "[" + check_id + "]":
                return property
    raise Exception("Error: failed to find a property with ID \"" + check_id + "\"")


def remove_check_ids_from_description(properties):
    """
    Some asserts generated by Kani have a unique ID in their description that is
    of the form:

    [KANI_CHECK_ID_<crate-fn-name>_<index>]

    e.g.:

    [KANI_CHECK_ID_foo.6875c808::foo_0] assertion failed: x % 2 == 0

    This function removes those IDs from the property's description so that
    they're not shown to the user. The removal of the IDs should only be done
    after all ID-based post-processing is done.
    """
    check_id_pattern = re.compile(r"\[" + GlobalMessages.CHECK_ID_RE + r"\] ")
    for property in properties:
        property["description"] = re.sub(check_id_pattern, "", property["description"])


def construct_solver_information_message(solver_information):
    """
    From the extracted information, construct a message and append to the final Output

    Sample Output -
        CBMC 5.36.0 (cbmc-5.36.0)
        CBMC version 5.36.0 (cbmc-5.36.0) 64-bit x86_64 linux
        Reading GOTO program from file
        ...
    """

    solver_information_message = ""
    for message_object in solver_information:
        # 'Program' and 'messageText' fields give us the information about the solver
        try:
            # Objects with the key "program" give information about CBMC's version
            # Example - {'program': 'CBMC 5.44.0 (cbmc-5.44.0)'}
            if GlobalMessages.PROGRAM in message_object.keys():
                solver_information_message += message_object['program']

            # Check message texts - objects with the key 'messageText'
            # Example - {'messageText': 'CBMC version 5.44.0 (cbmc-5.44.0) 64-bit x86_64 linux', 'messageType': 'STATUS-MESSAGE'},
            # {'messageText': 'Reading GOTO program from file', 'messageType': 'STATUS-MESSAGE'}
            elif GlobalMessages.MESSAGE_TEXT in message_object.keys():
                # Remove certain messageTexts which do not contain important information, like "Building error trace"
                if message_object['messageText'] != GlobalMessages.CONST_TRACE_MESSAGE:
                    solver_information_message += message_object['messageText']
                else:
                    solver_information_message += '\n'
                    break
            else:
                pass
        except KeyError as e:
            print("Key Error, Missing Properties in reading Solver Information from JSON")
        solver_information_message += '\n'
    return solver_information_message

def construct_terse_property_message(properties):
    """
    Get property tests and results from the Json file written and construct
    a terse final output displaying only final results and summary of tests

    input -
        list - properties is a list of small json objects containing each test , description and result
        Ex -
            {'description': 'assertion false', 'property': 'fmaf.assertion.1', 'status': 'SUCCESS'}
            {'description': 'assertion failed: 2 == 4', 'property': 'main.assertion.1', 'status': 'FAILURE', 'trace': [{'function': {'displayName' ..

    output -
        str - Final string output which is a summary of the property tests
        Ex - SUMMARY:
            ** 1 of 54 failed
            Failed Checks: assertion failed: 2 == 4
            File: "/home/ubuntu/test.rs", line 3, in main
            VERIFICATION:- FAILED
    """
    number_tests_failed = 0
    output_message = ""
    failed_tests = []
    index = 0
    verification_status = GlobalMessages.FAILED

    # Parse each property instance in properties
    for index, property_instance in enumerate(properties):
        status = property_instance["status"]
        if status == "FAILURE":
            number_tests_failed += 1
            failed_tests.append(property_instance)
        else:
            pass

    # Ex - OUTPUT: ** 1 of 54 failed
    output_message += f"VERIFICATION RESULT: \n ** {number_tests_failed} of {index+1} failed\n"

    # The Verification is successful and the program is verified
    if number_tests_failed == 0:
        verification_status = colored_text(Fore.GREEN, "SUCCESSFUL")
    else:
        # Go through traces to extract relevant information to be displayed in the summary
        # only in the case of failure
        verification_status = colored_text(Fore.RED, "FAILED")
        for failed_test in failed_tests:
            try:
                failure_message = failed_test["description"]
                failure_source = failed_test["trace"][-1]['sourceLocation']
                failure_message_path = failure_source['file']
                failure_function_name = failure_source['function']
                failure_line_number = failure_source['line']
                output_message += f"Failed Checks: {failure_message}\n File: \"{failure_message_path}\", line {failure_line_number}, in {failure_function_name}\n"
            except KeyError:
                failure_source = "None"
                output_message += f"Failed Checks: {failure_message}\n"

    # TODO: Get final status from the cprover status
    output_message += f"\nVERIFICATION:- {verification_status}\n"

    return output_message, number_tests_failed

def construct_property_message(properties):
    """
    Get property tests and results from the Json file written and construct
    a verbose final output displaying each property test results and summary of tests

    input -
        list - properties is a list of small json objects containing each test , description and result
        Ex -
            {'description': 'assertion false', 'property': 'fmaf.assertion.1', 'status': 'SUCCESS'}
            {'description': 'assertion failed: 2 == 4', 'property': 'main.assertion.1', 'status': 'FAILURE', 'trace': [{'function': {'displayName' ..

    output -
        str - Final string output which is a detailed output displaying all the tests and the results
        Ex - SUMMARY:
            Property 53: sinf.assertion.1
                - Status: SUCCESS
                - Description: "assertion false"
                - Location: file/path.rs:10:8 in function harness
            Property 54: calloc.assertion.1
                - Status: SUCCESS
                - Description: "assertion false"

        Note: Location is missing on CBMC checks. In those cases, we omit the Location line.
    """

    number_tests_failed = 0
    number_tests_unreachable = 0
    number_tests_undetermined = 0
    output_message = ""
    failed_tests = []
    index = 0
    verification_status = GlobalMessages.FAILED

    output_message = "RESULTS:\n"

    # for property_instance in properties:
    for index, property_instance in enumerate(properties):
        try:
            name = property_instance["property"]
            status = property_instance["status"]
            description = property_instance["description"]
            location = SourceLocation(property_instance.get("sourceLocation", {}))
        except KeyError as e:
            print("Key not present in json property", e)

        if status == "SUCCESS":
            message = colored_text(Fore.GREEN, f"{status}")
        elif status == "UNDETERMINED":
            message = colored_text(Fore.YELLOW, f"{status}")
            number_tests_undetermined += 1
        elif status == "UNREACHABLE":
            message = colored_text(Fore.YELLOW, f"{status}")
            number_tests_unreachable += 1
        else:
            number_tests_failed += 1
            failed_tests.append(property_instance)
            message = colored_text(Fore.RED, f"{status}")

        """ Ex - Property 54: calloc.assertion.1
         - Status: SUCCESS
         - Description: "assertion false" """
        output_message += f"Check {index+1}: {name}\n" \
            f"\t - Status: {message}\n" \
            f"\t - Description: \"{description}\"\n"
        if location:
            output_message += f"\t - Location: {location}\n"

        output_message += "\n"

    output_message += f"\nSUMMARY: \n ** {number_tests_failed} of {index+1} failed"
    other_status = []
    if number_tests_undetermined > 0:
        other_status.append(f"{number_tests_undetermined} undetermined")
    if number_tests_unreachable > 0:
        other_status.append(f"{number_tests_unreachable} unreachable")
    if other_status:
        output_message += " ("
        output_message += ",".join(other_status)
        output_message += ")"
    output_message += "\n"

    # The Verification is successful and the program is verified
    if number_tests_failed == 0:
        verification_status = colored_text(Fore.GREEN, "SUCCESSFUL")
    else:
        verification_status = colored_text(Fore.RED, "FAILED")
        for failed_test in failed_tests:
            # Go through traces to extract relevant information to be displayed in the summary
            # only in the case of failure
            try:
                failure_message = failed_test["description"]
                failure_source = failed_test["trace"][-1]['sourceLocation']
                failure_message_path = failure_source['file']
                failure_function_name = failure_source['function']
                failure_line_number = failure_source['line']
                output_message += f"Failed Checks: {failure_message}\n File: \"{failure_message_path}\", line {failure_line_number}, in {failure_function_name}\n"
            except KeyError:
                failure_source = "None"
                output_message += f"Failed Checks: {failure_message}\n"

    # TODO: Change this to cProver status
    # TODO: Extract information from CBMC about iterations
    output_message += f"\nVERIFICATION:- {verification_status}\n"

    return output_message, number_tests_failed

def colored_text(color, text):
    """
    Only use colored text if running in a terminal to avoid dumping escape
    characters
    """
    if sys.stdout.isatty():
        return color + text + Style.RESET_ALL
    else:
        return text


if __name__ == "__main__":
    main(sys.argv)
