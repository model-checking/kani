# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT


"""CBMC JSON Parser

This script allows rmc to print to the console the final output displayed to the user
after CBMC gives the response JSON object.  

This script accepts JSON files. 

This script requires that `coloroma` be installed within the Python
environment you are running this script in.

This file can also be imported as a module and contains the following
functions:

    * transform_cbmc_output - returns the formatted string output after parsing
    * main - the main function of the script
"""

import json
from json.decoder import JSONDecodeError
import subprocess
import os
from colorama import Fore, Back, Style
import random

def main():
    
    # parser = argparse.ArgumentParser()
    # Sample command run
    command = 'rmc /home/ubuntu/rmc/src/test/rmc/Assume/main.rs'.split()
    result = subprocess.run(command, capture_output=True,universal_newlines=True)
    json_file = json.loads(result.stdout)
    print(json_file)
    return

def transform_cbmc_output(log_file, output_style):
    """
    Take Unstructured CBMC Response object, parse the blob and gives structured 
    and formatted output depending on User Inputted Output Style

    Parameters -
        log_file : str
            A response blob that is given to the function from CBMC
        output_style : int
            An index to tell the script which style of output is requested by the User.

    Returns - 
        None
            Prints the final output string to the user
    """

    # Enum to store the style of output that is given by the argument flags
    output_style_switcher = {
        '0' : 'default',
        '1' : 'terse',
        '2' : 'old'
    }

    # Output Message is the final output that is printed to the user
    output_message = ""

    # Check if the output given by CBMC is in Json format or not
    if is_json(log_file):
        json_file = json.loads(log_file)
        
        # Write cbmc output to a json file in scripts/temp_json for parallel I/O
        json_file = save_json_file(json_file)
        
        # Read from the saved json file and restructure it to make it parseable
        parseable_json_dictionary = restructure_json_file(json_file)
        
        # Extract property information from the restructured JSON file 
        properties = extract_properties(parseable_json_dictionary)
        
        # Using Case Switching to Toggle between various output styles
        # For now, the two options provided are default and terse
        if output_style_switcher[output_style] == 'default':
            
            # Extract Solver Information from the json file and construct message 
            solver_information = extract_solver_information(parseable_json_dictionary)
            output_message += construct_solver_information_message(solver_information)

            # Extract property messages from the json file
            # Combine both messages to give as final output 
            output_message += construct_property_message(properties)
        
        elif output_style_switcher[output_style] == 'terse':
            
            # Construct only summarized result and display output 
            output_message = construct_terse_property_message(properties)
        
        # Print using an Interfact function
        print_to_terminal(output_message)

        # Delete temp Json file after displaying result
        clear_json_file(json_file)
    else:
        # DynTrait tests generate a non json output due to "Invariant check failed" error
        # For these cases, we just produce the cbmc output unparsed
        # TODO: Parse these non json outputs from CBMC
        non_json_cbmc_output_handler(log_file)
    return

# Write the json file to disk for easy parsing
def save_json_file(log_file):

    # TODO : Change to use the source file name as the temp file name too
    # Create a temp json file for every source file as testing can be parallelized
    temp_file_name = str(random.randint(10,1000000))
    saved_json = "/home/ubuntu/rmc/scripts/temp_json/" + temp_file_name + "_temp.json"
    try:
        with open(saved_json, "w", encoding='utf-8') as outfile:
            json.dump(log_file, outfile)
    except FileNotFoundError as e:
        print("Error in writing JSON file", e)
    return saved_json

# Check if the blob is in json format for parsing
def is_json(myjson):
    try:
        json.loads(myjson)
    except ValueError:
        return False
    return True

# Function for printing temporary json objects for debugging purposes
def print_json_file(input_json_file):
    with open(input_json_file) as infile:
        data = json.load(infile)
        print(data)
    return

# Function to reformat CBMC blob into a parseable format
def restructure_json_file(log_file):
    """
    CBMC's response blob is difficult to parse by itself, so it needs to be re-fit into
    another temporary dictionary structure and 

    Parameters -
        log_file - log_file is the initial cbmc blob object which even in JSON format, 
                    is difficult to parse and in some cases, is not in JSON format.

    Returns - 
        temp_dict - The restructured dictionary which is easy to parse

    """
    
    # The restructured dictionary which is easy to parse
    temp_dict = {}
    try:
        with open(log_file) as infile:
            data = json.load(infile)
            temp_dict["response"] = data
    except FileNotFoundError:
        print("CBMC File not found")
        return {}
    except JSONDecodeError:
        # print out specific information about the extra data (like the lines before it)
        print("JSON File not being parsed", )
        return {}

    # TODO: Check if passing dictionaries around is optimal or if there are alternate ways to 
    # restructure and parse cbmc blob.
    return temp_dict

# Get Information about the Solver from the Json file written
def extract_solver_information(json_object):
    
    # Solver information is all the fields in the json object which are not related to the results
    solver_information = []
    try:
        responses = json_object["response"]
    except:
        print("No such Value called \"response\"")

    # Check for the objects which are not related to the result object
    for response_object in responses:
        if "result" not in response_object.keys():
            solver_information.append(response_object)
        else:
            return solver_information

    return solver_information

# From the extracted information, construct a message and append to the final Output
def construct_solver_information_message(solver_information):
    """
    Construct a message and append to the final Output

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
            if "program" in message_object.keys():
                solver_information_message += message_object['program']
            # Check message texts for valuable information and append only relevant messages
            elif "messageText" in message_object.keys():
                if message_object['messageText'] != 'Building error trace':
                    solver_information_message += message_object['messageText']
                else:
                    pass
            else:
                solver_information_message += ''
        except KeyError as e:
            print("Key Error, Missing Properties in reading Solver Information from JSON")
        solver_information_message += '\n'
    return solver_information_message

# Get property tests and results from the Json file written
def extract_properties(json_object):
    try:
        responses = json_object["response"]
    except:
        print("No such field called \"response\"")
    
    for response_object in responses:
        if "result" in response_object.keys():
            properties = response_object["result"]
        else:
            #TODO: Handle cases when result field not present not in response object
            pass

    return properties

# Get property tests and results from the Json file written
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
            Failed Tests: assertion failed: 2 == 4
            File: "/home/ubuntu/test.rs", line 3, in main
            VERIFICATION:- FAILED
    """
    number_tests_failed = 0
    output_message = ""
    failed_tests = []
    index = 0
    verification_status = "FAILED"

    # Parse each property instance in properties
    for index, property_instance in enumerate(properties):
        status = property_instance["status"]
        if status == "FAILURE":
            number_tests_failed += 1
            failed_tests.append(property_instance)
        else:
            pass

    # Ex - SUMMARY: ** 1 of 54 failed
    output_message += f"SUMMARY: \n ** {number_tests_failed} of {index+1} failed\n"
    
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
                output_message += f"Failed Tests: {failure_message}\n File: \"{failure_message_path}\", line {failure_line_number}, in {failure_function_name}"
            except KeyError:
                failure_source = "None"
                output_message += f"Failed Tests: {failure_message}\n"

    #TODO: Get final status from the cprover status
    output_message += f"\nVERIFICATION:- {verification_status}\n"

    return output_message

# Construct the final display output from the list of properties
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
    verification_status = "FAILED"

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
        else:
            number_tests_failed += 1
            failed_tests.append(property_instance)
            message = Fore.RED + f"{status}" + Style.RESET_ALL

        """ Ex - Property 54: calloc.assertion.1
         - Status: SUCCESS
         - Description: "assertion false" """
        output_message += f"Property {index+1}: {name}\n\t - Status: " + message + f"\n\t - Description: \"{description}\"\n"

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
                output_message += f"Failed Tests: {failure_message}\n File: \"{failure_message_path}\", line {failure_line_number}, in {failure_function_name}"
            except KeyError:
                failure_source = "None"
                output_message += f"Failed Tests: {failure_message}\n"

    #TODO: Change this to cProver status
    #TODO: Extract information from CBMC about iterations
    output_message += f"\nVERIFICATION:- {verification_status}\n"

    return output_message

# Method provides a handler for non json cbmc outputs, for example
# with DynTrait tests
def non_json_cbmc_output_handler(logfile):
    # Basic handling is just printing the output provided by CBMC
    print(logfile)

# Method provides an Interface to printing, can be expanded upon
def print_to_terminal(output_message):
    print(output_message)
    return

# Deleting Temp JSON files created for parsing
def clear_json_file(saved_file):
    try:
        os.remove(saved_file)
    except :
        pass
    return

if __name__ == "__main__":
    main()

