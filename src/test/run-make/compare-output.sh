#!/bin/bash

LINES=$1
FILE=$2

while IFS= read -r line; do
    if ! grep -q -F "$line" "$FILE"; then
        echo "This line doesn't exist: $line"
        exit 1
    fi
done < "$LINES"
