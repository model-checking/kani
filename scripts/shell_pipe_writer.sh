#!/bin/bash

for i in $(seq 1 5); do
   echo "iteration" $i
   sleep 1
done

echo "after iteration"
echo "after the test, making sure this doesn't appear"

for i in $(seq 1 5); do
   echo "number" $i
   sleep 1
done