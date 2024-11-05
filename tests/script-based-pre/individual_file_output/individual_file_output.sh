#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

OUT_DIR=tmp_sample_crate

# Ensure output folder is clean
rm -rf ${OUT_DIR}

# Move the original source to the output folder since it will be modified
cp -r sample_crate ${OUT_DIR}
pushd $OUT_DIR

echo "Run verification..."
cargo kani -Z unstable-options --output-into-files

OUTPUT_DIR="result_output_dir" 

# Check if the output directory exists
if [ ! -d "$OUTPUT_DIR" ]; then
    echo "Output directory $OUT_DIR/$OUTPUT_DIR does not exist. Verification failed."
    exit 1
fi

# Check if there are any files in the output directory
output_files=("$OUTPUT_DIR"/*)

if [ ${#output_files[@]} -eq 0 ]; then
    echo "No files found in the output directory. Verification failed."
    exit 1
fi

# Check if each file contains text
for file in "${output_files[@]}"; do
    if [ ! -s "$file" ]; then
        echo "File $file is empty. Verification failed."
        exit 1
    else
        echo "File $file is present and contains text."
    fi
done

popd
rm -rf ${OUT_DIR}
