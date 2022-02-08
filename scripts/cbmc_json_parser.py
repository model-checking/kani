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
from colorama import Fore, Style
import re
import sys
from enum import Enum

# Enum to store the style of output that is given by the argument flags
output_style_switcher = {
    'default': 'old',
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
    SUCCESS = 'SUCCESS'
    FAILED = 'FAILED'
    REACH_CHECK_DESC = "[KANI_REACHABILITY_CHECK]"
    CHECK_ID = "KANI_CHECK_ID_"
    UNSUPPORTED_CONSTRUCT_DESC = "is not currently supported by Kani"
    UNWINDING_ASSERT_DESC = "unwinding assertion loop"


def main():

    # Check only one json file as input
    if len(sys.argv) < 2:
        print("Json File Input Missing")
        sys.exit(1)

    # parse the input json file
    with open(sys.argv[1]) as f:
        sample_json_file_parsing = f.read()

    # the main function should take a json file as input
    transform_cbmc_output(sample_json_file_parsing, output_style=output_style_switcher["old"])
    return

def transform_cbmc_output(cbmc_response_string, output_style):
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
    is_json_bool, cbmc_json_array = is_json(cbmc_response_string)
    if is_json_bool:

        # Extract property information from the restructured JSON file
        properties, solver_information = extract_solver_information(cbmc_json_array)

        properties, messages = postprocess_results(properties)

        # Using Case Switching to Toggle between various output styles
        # For now, the two options provided are default and terse
        if output_style == output_style_switcher["regular"]:

            # Extract Solver Information from the json file and construct message
            output_message += construct_solver_information_message(solver_information)

            # Extract property messages from the json file
            # Combine both messages to give as final output
            output_message += construct_property_message(properties)

        elif output_style == output_style_switcher["terse"]:

            # Construct only summarized result and display output
            output_message = construct_terse_property_message(properties)

        # Print using an Interface function
        print(output_message)
        print(messages)

    else:
        # DynTrait tests generate a non json output due to "Invariant check failed" error
        # For these cases, we just produce the cbmc output unparsed
        # TODO: Parse these non json outputs from CBMC
        raise Exception("CBMC Crashed - Unable to present Result")

    return

# Check if the blob is in json format for parsing
def is_json(cbmc_output_string):
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

def postprocess_results(properties):
    """
    Check for certain cases, e.g. a reachable unsupported construct or a failed
    unwinding assertion, and update the results of impacted checks accordingly.
    1. Change all "SUCCESS" results to "UNDETERMINED" if the reachability check
    for a Rust construct that is not currently supported by Kani failed, since
    the missing exploration of execution paths through the unsupported construct
    may hide failures
    2. TODO: Change results from "SUCCESS" to "UNDETERMINED" if an unwinding
    assertion failed, since the insufficient unwinding may cause some execution
    paths to be left unexplored (https://github.com/model-checking/kani/issues/746)

    Additionally, print a message at the end of the output that indicates if any
    of the special cases above was hit.
    """

    has_reachable_unsupported_constructs = has_check_failure(properties, GlobalMessages.UNSUPPORTED_CONSTRUCT_DESC)
    has_failed_unwinding_asserts = has_check_failure(properties, GlobalMessages.UNWINDING_ASSERT_DESC)
    properties, reach_checks = filter_reach_checks(properties)
    annotate_properties_with_reach_results(properties, reach_checks)

    for property in properties:
        if has_reachable_unsupported_constructs:
            # Change SUCCESS to UNDETERMINED for all properties
            if property["status"] == "SUCCESS":
                property["status"] = "UNDETERMINED"
        elif "reach_check_result" in property and property["reach_check_result"] == "SUCCESS":
            property["status"] = "UNREACHABLE"
        # TODO: Handle unwinding assertion failure

    messages = ""
    if has_reachable_unsupported_constructs:
        messages += "** WARNING: A Rust construct that is not currently supported " \
                    "by Kani was found to be reachable. Check the results for " \
                    "more details."

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
    filtered_properties = []
    removed_properties = []
    for property in properties:
        if message in property["description"]:
            removed_properties.append(property)
        else:
            filtered_properties.append(property)
    return filtered_properties, removed_properties


def annotate_properties_with_reach_results(properties, reach_checks):
    for reach_check in reach_checks:
        description = reach_check["description"]
        str = GlobalMessages.CHECK_ID + r"([0-9]+)"
        assert_id_obj = re.search(GlobalMessages.CHECK_ID + r"([0-9])*", description)
        if not assert_id_obj:
            raise Exception("Error: failed to extract check ID for reachability check \"" + body + "\"")
        assert_id = assert_id_obj.group(0)
        prop = get_matching_property(properties, assert_id)
        prop["reach_check_result"] = reach_check["status"]
        prop["description"] = prop["description"].replace("[" + assert_id + "] ", "", 1)


def get_matching_property(properties, assert_id):
    for property in properties:
        if assert_id in property["description"]:
            return property
    raise Exception("Error: failed to find matching property for reachability check \"" + body + "\"")


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
        verification_status = Fore.GREEN + "SUCCESSFUL" + Style.RESET_ALL
    else:
        # Go through traces to extract relevant information to be displayed in the summary
        # only in the case of failure
        verification_status = Fore.RED + "FAILED" + Style.RESET_ALL
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

    return output_message

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
            Property 54: calloc.assertion.1
                - Status: SUCCESS
                - Description: "assertion false"
    """

    number_tests_failed = 0
    output_message = ""
    failed_tests = []
    index = 0
    verification_status = GlobalMessages.FAILED

    output_message = "RESULT:\n"

    # for property_instance in properties:
    for index, property_instance in enumerate(properties):
        try:
            name = property_instance["property"]
            status = property_instance["status"]
            description = property_instance["description"]
        except KeyError as e:
            print("Key not present in json property", e)

        if status == "SUCCESS":
            message = Fore.GREEN + f"{status}" + Style.RESET_ALL
        elif status == "UNDETERMINED" or status == "UNREACHABLE":
            message = Fore.YELLOW + f"{status}" + Style.RESET_ALL
        else:
            number_tests_failed += 1
            failed_tests.append(property_instance)
            message = Fore.RED + f"{status}" + Style.RESET_ALL

        """ Ex - Property 54: calloc.assertion.1
         - Status: SUCCESS
         - Description: "assertion false" """
        output_message += f"Check {index+1}: {name}\n\t - Status: " + \
            message + f"\n\t - Description: \"{description}\"\n" + "\n"

    output_message += f"\nSUMMARY: \n ** {number_tests_failed} of {index+1} failed\n"

    # The Verification is successful and the program is verified
    if number_tests_failed == 0:
        verification_status = Fore.GREEN + "SUCCESSFUL" + Style.RESET_ALL
    else:
        verification_status = Fore.RED + "FAILED" + Style.RESET_ALL
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

    return output_message


if __name__ == "__main__":
    main()
