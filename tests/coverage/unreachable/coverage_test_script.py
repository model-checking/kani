import difflib
import subprocess
import json
import os

files_without_unreachable_detected = []
files_with_discrepencies = []
files_with_matches = []

def run_command(command):
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, shell=True)
    output, error = process.communicate()
    return output.decode().strip()

def remove_first_line(string):
    lines = string.split('\n', 1)  # Split string into lines, maximum split = 1
    if len(lines) > 1:
        return '\n'.join(lines[1:])  # Join lines starting from the second line
    else:
        return ''  # If there is only one line or the string is empty, return an empty string

def remove_last_line(string):
    lines = string.split('\n')
    if len(lines) > 1:
        if lines[-2] == ']':
            return '\n'.join(lines[:-1])  # Join lines excluding the last line
        elif lines[-4] == ']':
            return '\n'.join(lines[:-3])
    else:
        return ''  # If there is only one line or the string is empty, return an empty string

def clean_json(string):
    return remove_last_line(remove_first_line(string))


def parse_lines(string):
    """The line numbers denoted by a line number encoding found in coverage data"""

    # string is an encoding of a set of line numbers like
    #   "1,3,6" -> {1, 3, 6}
    #   "1-3,6,20-32" -> {1, 2, 3, 6, 20, 21, 22}

    try:
        lines = set()
        for group in string.split(','):
            bounds = group.split('-')
            if len(bounds) == 1:
                lines.add(int(bounds[0]))
            elif len(bounds) == 2:
                lines.update(range(int(bounds[0]), int(bounds[1])+1))
            else:
                raise ValueError
        return sorted(lines)
    except ValueError:
        # ValueError: invalid literal for int()
        logging.info("Skipping malformed line number encoding: %s", string)
        set_incomplete_coverage()
        return []

def parse_description(description):
    """The source locations in the basic block encoded by a coverage goal description"""

    try:
        # description is "block N (lines BASIC_BLOCK)"
        basic_block = re.match(r'block [0-9]+ \(lines (.*)\)', description).group(1)

        if basic_block is None:
            raise ValueError

        # basic_block is
        #   chunk1;chunk2;chunk3
        # each chunk is
        #   test.c:foo:3,6-10
        # each function name foo from rust may include embedded semicolons like main::foo

        srclocs = []
        for chunk in basic_block.split(';'):
            fyle, func_lines = chunk.split(':', 1)  # fyle:func:lines -> fyle,func:lines
            func, lines = func_lines.rsplit(':', 1) # func:lines -> func,lines
            for line in parse_lines(lines):
                if fyle and func and line:
                    lines.append((fyle, func, line))
                else:
                    logging.info(
                        'Skipping malformed source location in coverage goal description: %s: '
                        'Found file:%s function:%s line:%s', description, fyle, func, line
                    )
                    set_incomplete_coverage()
        return srclocs
    except ValueError:
        # ValueError after after split()/rsplit(): not enough values to unpack
        logging.info('Skipping malformed coverage goal description: %s', description)
        set_incomplete_coverage()
        return []

def parse_basicBlockLines(basicBlockLines): # pylint: disable=invalid-name
    """The source locations in a basic block of a json coverage goal"""

    # basicBlockLines is json
    #   "basicBlockLines": {
    #     "test.c": {
    #       "main": "16,17"
    #     }
    #   }

    if basicBlockLines is None:
        return []

    srclocs = []
    for fyle, fyle_data in basicBlockLines.items():
        for func, lines in fyle_data.items():
            for line in parse_lines(lines):
                if fyle and func and line:
                    srclocs.append((fyle, func, line))
                else:
                    logging.info('Skipping malformed source location in coverage goal: '
                                 'Found file:%s function:%s line:%s', fyle, func, line)
                    set_incomplete_coverage()
    return srclocs

def json_srcloc_wkdir(cbmc_srcloc):
    """Extract working directory from a json source location"""
    return cbmc_srcloc.get('workingDirectory')

def get_rust_files(root_directory):
    # Traverse through the directory and its subdirectories
    rust_files = []
    for root, dirs, files in os.walk(root_directory):
        for filename in files:
            if filename.endswith(".rs"):
                file_path = os.path.join(root, filename)
                rust_files.append(file_path)

    return rust_files

def clean_lines(lines, file_path):
    lines_in_file = []
    for line in lines:
        fyle, func, line_no = line
        if file_path in fyle:
            lines_in_file.append(line)
        else:
            pass

    return lines_in_file

def load_cbmc_json(json_data, file_path):
    """Load json file produced by cbmc --cover location --json-ui."""

    try:
        goals_list = [entry for entry in json_data if 'goals' in entry]
        assert len(goals_list) == 1
        goals = goals_list[0]['goals']
    except AssertionError as error:
        raise UserWarning(
            f"Failed to locate coverage goals in json coverage data: {json_file}") from error

    # coverage is of type list(tuple(list(tuple), string))
    coverage = []
    for goal in goals:
        lines = (parse_basicBlockLines(goal.get("basicBlockLines")) or
                 parse_description(goal.get("description")))
        cleaned_lines = clean_lines(lines, file_path)
        status = goal["status"]
        if len(cleaned_lines) != 0:
            coverage.append((cleaned_lines, status))
        else:
            pass

    return coverage

def find_line_no_file(json_data_file, line_no):
    """In the goal list, find the results for the given line"""

    status_for_line = []

    for block in json_data_file:
        lines, status = block
        for line in lines:
            fyle, func, line_number_from_json = line
            if int(line_number_from_json) == int(line_no):
                status_for_line.append(status)

    return status_for_line

def run_commands(file_path):
    # Define the commands to run
    command1 = f"kani --enable-unstable --no-unwinding-checks --output-format=old {file_path} --cbmc-args --cover location --json-ui"
    command2 = f"kani --enable-unstable --no-unwinding-checks --output-format=old {file_path} --cbmc-args --json-ui"

    # Run the commands and get their outputs
    output1 = run_command(command1)
    output2 = run_command(command2)

    output1_cleaned = clean_json(output1)
    output2_cleaned = clean_json(output2)

    # Parse the outputs as JSON
    try:
        json_output1 = json.loads(output1_cleaned)
        json_output2 = json.loads(output2_cleaned)

        coverage_data_for_file = load_cbmc_json(json_output1, file_path)

        # Find reachability checks in output2
        found_reachability_check = []
        for item in json_output2:
            if 'result' in str(item):
                for prop in item['result']:
                    if 'reachability_check' in str(prop):
                        found_reachability_check.append((prop['sourceLocation'],prop['status']))

        unreachable_line_numbers = []
        for check in found_reachability_check:
            if check[1] == 'SUCCESS':
                if file_path in check[0]['file']:
                    unreachable_line_numbers.append(check[0]['line'])

        if len(unreachable_line_numbers) == 0:
            files_without_unreachable_detected.append(file_path)
            return

        # Find reachability checks in output1
        found_reachability_check_output1 = []
        found_match = False
        for line_number in unreachable_line_numbers:
            statuses = find_line_no_file(coverage_data_for_file, line_number)
            if len(statuses) != 0:
                if "failed" in statuses:
                    found_match = True
                    print(file_path, line_number, statuses)
                    files_with_matches.append(file_path)
                else:
                    print(file_path, line_number, statuses)
                    files_with_discrepencies.append(file_path)
            else:
                files_with_discrepencies.append(file_path)
    except:
        pass


def run_on_no_unreachable(file_path):

    # Define the commands to run
    command1 = f"kani --enable-unstable --no-unwinding-checks --output-format=old {file_path} --cbmc-args --cover location --json-ui"
    command2 = f"kani --enable-unstable --no-unwinding-checks --output-format=old {file_path} --cbmc-args --json-ui"

    # Run the commands and get their outputs
    output1 = run_command(command1)

    output1_cleaned = clean_json(output1)

    # Parse the outputs as JSON
    try:
        json_output1 = json.loads(output1_cleaned)

        coverage_data_for_file = load_cbmc_json(json_output1, file_path)

        for block in coverage_data_for_file:
            lines, status = block
            if status == "failed":
                for line in lines:
                    fyle, func, line_no = line
                    print(f"The file {fyle} and line no: {line_no} was found unreachable with --cover")
    except:
        pass

    return


# Define the directory to parse for Rust files (including subdirectories)
reachable_directory = "tests/coverage/reachable"
unreachable_directory = "tests/coverage/unreachable"

# run_commands("tests/coverage/unreachable/wrong_coverage_2/main.rs")

for file in get_rust_files(unreachable_directory):
    run_commands(file)

print("\n")
print("With --cover, these files have unreachable line detectable. But with --cover, the same lines could not be detected")
print(files_without_unreachable_detected, "\n")
print("files with discrepencies in the two outputs\n")
print(files_with_discrepencies, "\n")
print("Files that match")
print(files_with_matches, "\n")

print("\n")
print("With --cover, these files have unreachable line detectable. But with --cover, the same lines could not be detected")
for file in files_without_unreachable_detected:
    run_on_no_unreachable(file)
