#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


DOCUMENTATION=\
'kani-top-100.sh -- script to clone and compile the top 100 crates with Kani.

USAGE:
./scripts/kani-top-100.sh

Download the top 100 crates and runs kani on them. Prints out the
errors and warning when done. Xargs is required for this script to
work.

ENV:
- PRINT_STDOUT=1 forces this script to search for warning in
  STDOUT in addition to STDERR

EDITING:

- To modify the list of crates to crawl, modify
  `HARD_CODED_TOP_100_CRATES_AS_OF_2022_6_17`.
- To adjust the git clone or kani args, modify the function
  `clone_and_run_kani`.
- To adjust the errors this script searches for, edit the function
  `print_errors_for_each_repo_result`

Copyright Kani Contributors
SPDX-License-Identifier: Apache-2.0 OR MIT'


SELF_SCRIPT=$0
SELF_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
NPROC=$(nproc 2> /dev/null || sysctl -n hw.ncpu 2> /dev/null || echo 4)  # Linux or Mac or hard-coded default of 4
WORK_DIRECTORY_PREFIX="$SELF_DIR/../target/top-100"
HARD_CODED_TOP_100_CRATES_AS_OF_2022_6_17=$(cat $SELF_DIR/../tests/large-crawl/crates-top-100.txt)

STDOUT_SUFFIX='stdout.cargo-kani'
STDERR_SUFFIX='stderr.cargo-kani'
EXIT_CODE_SUFFIX='exit-code.cargo-kani'
# worker function that clones target repos and runs kani over
# them. This functions is called in parallel by
# parallel_clone_and_run, and should not be run explicitly
function clone_and_run_kani {
    WORK_NUMBER_ID=$(echo $1 | awk -F ',' '{ print $1}')
    REPOSITORY_URL=$(echo $1 | awk -F ',' '{ print $2}')
    REPO_DIRECTORY="$WORK_DIRECTORY_PREFIX/$WORK_NUMBER_ID"
    echo "work# $WORK_NUMBER_ID -- $REPOSITORY_URL"

    # clone or update repository
    (git clone $REPOSITORY_URL $REPO_DIRECTORY 2> /dev/null || git -C $REPO_DIRECTORY pull)

    # run cargo kani compile on repo. save results to file.
    PATH=$PATH:$SELF_DIR
    (cd $REPO_DIRECTORY; nice -n15 cargo kani --only-codegen) \
         1> $REPO_DIRECTORY/$STDOUT_SUFFIX \
         2> $REPO_DIRECTORY/$STDERR_SUFFIX
    echo $? > $REPO_DIRECTORY/$EXIT_CODE_SUFFIX
}

OVERALL_EXIT_CODE='0'
TARGET_ERROR_REGEX='warning:\sFound\sthe\sfollowing\sunsupported\sconstructs:\|WARN'
# printing function that greps the error logs and exit code.
function print_errors_for_each_repo_result {
    DIRECTORY=$1
    IS_FAIL='0'

    error_code="$(cat $DIRECTORY/$EXIT_CODE_SUFFIX)"
    if ! [ "$error_code" = "0" ]; then
        echo -e "Error exit: code $error_code\n"
        IS_FAIL='1'
    fi

    STDERR_GREP=$(grep -A3 -n $TARGET_ERROR_REGEX $DIRECTORY/$STDERR_SUFFIX 2> /dev/null && echo 'STDERR has warnings')
    if [[ "$STDERR_GREP" =~ [a-zA-Z0-9] ]]; then
        echo -e "------ STDERR Warnings (Plus 3 lines after) -----\n$STDERR_GREP"
        IS_FAIL='1'
    fi

    STDOUT_GREP=$(grep -A3 -n $TARGET_ERROR_REGEX $DIRECTORY/$STDOUT_SUFFIX 2> /dev/null && echo 'STDOUT has warnings')
    if [[ "$STDOUT_GREP" =~ [a-zA-Z0-9] ]] && [ "$PRINT_STDOUT" = '1' ]; then
        echo -e "------ STDOUT Warnings (Plus 3 lines after) -----\n$STDOUT_GREP"
        IS_FAIL='1'
    fi

    if [ "$IS_FAIL" -eq "0" ]; then
        echo 'Ok'
    fi
}

if ! which xargs 1>&2 1> /dev/null; then
    echo "Need to have xargs installed. Please install with `apt-get install -y xargs`"
    exit -1
elif [[ "$*" == *"--help"* ]]; then
    echo -e "$DOCUMENTATION"
elif [ "$#" -eq "0" ]; then
    # top level logic that runs clone_and_run_kani in parallel with xargs.
    mkdir -p $WORK_DIRECTORY_PREFIX
    echo -e "$HARD_CODED_TOP_100_CRATES_AS_OF_2022_6_17" | \
        awk -F '\n' 'BEGIN{ a=0 }{ print a++ "," $1  }' | \
        xargs -n1 -I {} -P $NPROC bash -c "$SELF_SCRIPT {}"

    # serially print out the ones that failed.
    for directory in $(ls $WORK_DIRECTORY_PREFIX); do
        REPOSITORY=$(git -C $WORK_DIRECTORY_PREFIX/$directory remote -v | awk '{ print $2 }' | head -1)
        echo "repository: $REPOSITORY"

        ERROR_OUTPUTS=$(print_errors_for_each_repo_result $WORK_DIRECTORY_PREFIX/$directory)
        if [[ ! "$ERROR_OUTPUTS" =~ 'STD... has warnings' ]]; then
            OVERALL_EXIT_CODE='1'
        fi

        echo -e "$ERROR_OUTPUTS" | sed 's/^/    /'
    done
else
    (clone_and_run_kani $1 $2)
fi

exit $OVERALL_EXIT_CODE
