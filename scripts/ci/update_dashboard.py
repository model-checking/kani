#!/usr/bin/python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import sys
from bs4 import BeautifulSoup

def update_name(run, path):
    """Shorten name/path and add link to example"""
    orig_path = path.p.string
    new_string = '/'.join(orig_path.split('/')[4:])
    new_tag = run.new_tag('a')
    new_tag.string = new_string
    # Add link to the example
    new_tag['href'] = "artifacts/" + new_string
    path.p.replace_with(new_tag)

with open(sys.argv[1]) as fp:
    run = BeautifulSoup(fp, 'html.parser')

    # Update pipeline names to link to the example under test
    for row in run.find_all('div', attrs={'class': 'pipeline-row'}):
        path = row.find('div', attrs={'class': 'pipeline-name'})
        # Some paths here may be `None` - skip them
        if path.p:
            update_name(run, path)

    # Delete links to empty artifacts folder from progress bars
    for bar in run.find_all('a', attrs={'class': 'stage-artifacts-link fail'}):
        del bar['href']
    for bar in run.find_all('a', attrs={'class': 'stage-artifacts-link success'}):
        del bar['href']
    
    with open("new_index.html", "w") as file:
        file.write(str(run))
