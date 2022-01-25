#!/usr/bin/python3
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
from bs4 import BeautifulSoup

def update_path(run, path):
    '''
    Shortens a path referring to an example and adds a link to the file.

    By default, the path to an example follows this pattern:

    `tests/bookrunner/books/<book>/<chapter>/<section>/<subsection>/<line>.rs`

    However, only the first part is shown since these paths are enclosed
    in paragraph markers (`<p>` and `</p>`). So they are often rendered as:

    `tests/bookrunner/books/<book>/<chapter>/...

    This update removes `tests/bookrunner/books/` from the path (common to
    all examples) and transforms them into anchor elements with a link to
    the example, so the path to the example is shown as:

    `<book>/<chapter>/<section>/<subsection>/<line>.rs`
    '''
    orig_path = path.p.string
    new_string = '/'.join(orig_path.split('/')[4:])
    new_tag = run.new_tag('a')
    new_tag.string = new_string
    # Add link to the example
    new_tag['href'] = "artifacts/" + new_string
    path.p.replace_with(new_tag)

def main():
    parser = argparse.ArgumentParser(
        description='Produces an updated HTML report file from the '
                    'contents of an HTML file generated with `litani`')
    parser.add_argument('input')
    parser.add_argument('output')
    args = parser.parse_args()

    with open(args.input) as fp:
        run = BeautifulSoup(fp, 'html.parser')

    # Update pipeline names to link to the example under test
    for row in run.find_all(lambda tag: tag.name == 'div' and
                            tag.get('class') == ['pipeline-row']):
        path = row.find('div', attrs={'class': 'pipeline-name'})
        # Some paths here may be `None` - skip them
        if path.p:
            update_path(run, path)

    # Delete links to empty artifacts folder from progress bars
    for bar in run.find_all('a', attrs={'class': 'stage-artifacts-link fail'}):
        del bar['href']
    for bar in run.find_all('a', attrs={'class': 'stage-artifacts-link success'}):
        del bar['href']

    with open(args.output, "w") as file:
        file.write(str(run))


if __name__ == "__main__":
    main()
