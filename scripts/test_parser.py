# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT
import unittest
import os
import tempfile
from cbmc_json_parser import SourceLocation


def source_json(filename=None, function=None, line=None, column=None):
    result = dict()
    if filename:
        result["file"] = filename
    if function:
        result["function"] = function
    if column:
        result["column"] = column
    if line:
        result["line"] = line
    return result


class SourceLocationTest(unittest.TestCase):
    """ Unit tests for SourceLocation """

    def test_source_location_valid_path(self):
        """Path returned by filepath() works for valid paths"""
        path = tempfile.gettempdir()
        json = source_json(path)
        src_loc = SourceLocation(json)
        self.assertEqual(src_loc.filepath(), path)

    def test_source_location_invalid_path(self):
        """Path returned by filepath() returns the input path if it doesn't exist"""
        path = "/rust/made/up/path/lib.rs"
        json = source_json(path)
        src_loc = SourceLocation(json)
        self.assertEqual(src_loc.filepath(), path)

    def test_source_location_relative_path(self):
        """Path returned by filepath() is relative if the input is also relative"""
        relpath = "dummy/target.rs"
        json = source_json(relpath)
        src_loc = SourceLocation(json)
        self.assertEqual(src_loc.filepath(), relpath)

    def test_source_location_absolute_cwd_path(self):
        """Path returned by filepath() is relative to current directory

            Note that the file may not exist that this should still hold.
        """
        relpath = "dummy/target.rs"
        path = os.path.join(os.getcwd(), relpath)
        self.assertNotEqual(relpath, path)

        json = source_json(path)
        src_loc = SourceLocation(json)
        self.assertEqual(src_loc.filepath(), relpath)

    def test_source_location_absolute_user_path(self):
        """Path returned by filepath() is relative to current directory

            Note that the file may not exist that this should still hold.
        """
        relpath = "~/dummy/target.rs"
        path = os.path.expanduser(relpath)
        self.assertNotEqual(relpath, path)

        json = source_json(path)
        src_loc = SourceLocation(json)
        self.assertEqual(src_loc.filepath(), relpath)

    def test_source_location_relative_user_path(self):
        """Path returned by filepath() is relative to current directory

            Note that the file may not exist that this should still hold.
        """
        relpath = "~/dummy/target.rs"
        json = source_json(relpath)
        src_loc = SourceLocation(json)
        self.assertEqual(src_loc.filepath(), relpath)

    def test_source_location_with_no_path(self):
        """Function filepath() is None if no file is given in the input"""
        json = source_json(function="main")
        src_loc = SourceLocation(json)
        self.assertIsNone(src_loc.filepath())

    def test_source_location_output_format(self):
        """Check that output includes all the information provided"""
        path = "/rust/made/up/path/lib.rs"
        function = "harness"
        line = 10
        column = 8
        json = source_json(path, function, line, column)
        src_loc = str(SourceLocation(json))
        self.assertIn(path, src_loc)
        self.assertIn(f"{path}:{line}:{column}", src_loc)
        self.assertIn(function, src_loc)


if __name__ == '__main__':
    unittest.main()
