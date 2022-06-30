#!/bin/bash

SELF_SCRIPT=$0
SELF_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
NPROC=$(nproc 2> /dev/null || sysctl -n hw.ncpu 2> /dev/null || echo 4)  # Linux or Mac or hard-coded default of 4
WORK_DIRECTORY_PREFIX="$SELF_DIR/../target/top-100"
HARD_CODED_TOP_100_CRATES_AS_OF_2022_6_17='0,https://github.com/Amanieu/parking_lot
1,https://github.com/Amanieu/thread_local-rs
2,https://github.com/BurntSushi/aho-corasick
3,https://github.com/BurntSushi/byteorder
4,https://github.com/BurntSushi/memchr
5,https://github.com/BurntSushi/termcolor
6,https://github.com/Frommi/miniz_oxide
7,https://github.com/Gilnaa/memoffset
8,https://github.com/Kimundi/rustc-version-rs
9,https://github.com/RustCrypto/traits
10,https://github.com/RustCrypto/utils
11,https://github.com/SergioBenitez/version_check
12,https://github.com/SimonSapin/rust-std-candidates
13,https://github.com/alexcrichton/cc-rs
14,https://github.com/alexcrichton/cfg-if
15,https://github.com/alexcrichton/toml-rs
16,https://github.com/bitflags/bitflags
17,https://github.com/bluss/arrayvec
18,https://github.com/bluss/either
19,https://github.com/bluss/indexmap
20,https://github.com/bluss/scopeguard
21,https://github.com/chronotope/chrono
22,https://github.com/clap-rs/clap
23,https://github.com/contain-rs/vec-map
24,https://github.com/crossbeam-rs/crossbeam
25,https://github.com/cryptocorrosion/cryptocorrosion
26,https://github.com/cuviper/autocfg
27,https://github.com/dguo/strsim-rs
28,https://github.com/dtolnay/anyhow
29,https://github.com/dtolnay/itoa
30,https://github.com/dtolnay/proc-macro-hack
31,https://github.com/dtolnay/proc-macro2
32,https://github.com/dtolnay/quote
33,https://github.com/dtolnay/ryu
34,https://github.com/dtolnay/semver
35,https://github.com/dtolnay/syn
36,https://github.com/dtolnay/thiserror
37,https://github.com/env-logger-rs/env_logger
38,https://github.com/fizyk20/generic-array.git
39,https://github.com/hyperium/h2
40,https://github.com/hyperium/http
41,https://github.com/hyperium/hyper
42,https://github.com/marshallpierce/rust-base64
43,https://github.com/matklad/once_cell
44,https://github.com/mgeisler/textwrap
45,https://github.com/ogham/rust-ansi-term
46,https://github.com/paholg/typenum
47,https://github.com/retep998/winapi-rs
48,https://github.com/rust-itertools/itertools
49,https://github.com/rust-lang-nursery/lazy-static.rs
50,https://github.com/rust-lang/backtrace-rs
51,https://github.com/rust-lang/futures-rs
52,https://github.com/rust-lang/hashbrown
53,https://github.com/rust-lang/libc
54,https://github.com/rust-lang/log
55,https://github.com/rust-lang/pkg-config-rs
56,https://github.com/rust-lang/regex
57,https://github.com/rust-lang/socket2
58,https://github.com/rust-num/num-integer
59,https://github.com/rust-num/num-traits
60,https://github.com/rust-random/getrandom
61,https://github.com/rust-random/rand
62,https://github.com/seanmonstar/httparse
63,https://github.com/seanmonstar/num_cpus
64,https://github.com/serde-rs/json
65,https://github.com/serde-rs/serde
66,https://github.com/servo/rust-fnv
67,https://github.com/servo/rust-smallvec
68,https://github.com/servo/rust-url
70,https://github.com/servo/unicode-bidi
71,https://github.com/softprops/atty
72,https://github.com/steveklabnik/semver-parser
73,https://github.com/taiki-e/pin-project-lite
74,https://github.com/time-rs/time
75,https://github.com/tokio-rs/bytes
76,https://github.com/tokio-rs/mio
77,https://github.com/tokio-rs/slab
78,https://github.com/tokio-rs/tokio
79,https://github.com/unicode-rs/unicode-normalization
80,https://github.com/unicode-rs/unicode-segmentation
81,https://github.com/unicode-rs/unicode-width
82,https://github.com/unicode-rs/unicode-xid
83,https://github.com/withoutboats/heck'

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
# printing function that greps the error logs signal
function print_errors_for_each_repo_result {
    DIRECTORY=$1
    IS_FAIL='0'

    error_code="$(cat $DIRECTORY/$EXIT_CODE_SUFFIX)"
    if [ "$error_code" -ne "0" ]; then
	echo -e "Error exit: code $error_code\n"
	IS_FAIL='1'
    fi

    STDERR_GREP=$(grep -A3 -n $TARGET_ERROR_REGEX $DIRECTORY/$STDERR_SUFFIX 2> /dev/null && echo 'STDERR has warnings')
    if [[ "$STDERR_GREP" =~ [a-zA-Z0-9] ]]; then
	echo -e "------ STDERR Warnings (Plus 3 lines after) -----\n$STDERR_GREP"
	IS_FAIL='1'
    fi

    STDOUT_GREP=$(grep -A3 -n $TARGET_ERROR_REGEX $DIRECTORY/$STDOUT_SUFFIX 2> /dev/null && echo 'STDOUT has warnings')
    if [[ "$STDOUT_GREP" =~ [a-zA-Z0-9] ]] && [ "$PRINT_STDOUT" -eq "1" ]; then
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
    echo $HARD_CODED_TOP_100_CRATES_AS_OF_2022_6_17 | xargs -d ' ' -I {} -P $NPROC bash -c "$SELF_SCRIPT {}"
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
