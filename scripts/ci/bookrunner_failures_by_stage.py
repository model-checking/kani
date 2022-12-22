#!/usr/bin/python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
from bs4 import BeautifulSoup

def main():
    parser = argparse.ArgumentParser(
        description='Scans an HTML dashboard file and prints'
                    'the number of failures grouped by stage')
    parser.add_argument('input')
    args = parser.parse_args()

    with open(args.input) as fp:
        run = BeautifulSoup(fp, 'html.parser')

    failures = {}
    failures[0] = 0
    failures[1] = 0
    failures[2] = 0
    
    for row in run.find_all('div', attrs={'class': 'pipeline-row'}):
        stages = row.find_all('div', attrs={'class': 'pipeline-stage'})
        i = 0
        for stage in stages:
            if stage.a['class'][1] == 'fail':
                failures[i] += 1
                break
            i += 1

    print('bookrunner failures grouped by stage:')
    print(' * rustc-compilation: ' + str(failures[0]))
    print(' * kani-codegen: ' + str(failures[1]))
    print(' * cbmc-verification: ' + str(failures[2]))

if __name__ == "__main__":
    main()
