# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Benchcomp regression testing suite. This suite uses Python's stdlib unittest
# module, but nevertheless actually runs the binary rather than running unit
# tests.

import unittest
import uuid

import benchcomp.entry.run



class TestEnvironmentUpdater(unittest.TestCase):
    def test_environment_construction(self):
        """Test that the default constructor reads the OS environment"""

        update_environment = benchcomp.entry.run._EnvironmentUpdater()
        environment = update_environment({})
        self.assertIn("PATH", environment)


    def test_placeholder_construction(self):
        """Test that the placeholder constructor reads the placeholder"""

        key, value = [str(uuid.uuid4()) for _ in range(2)]
        update_environment = benchcomp.entry.run._EnvironmentUpdater({
            key: value,
        })
        environment = update_environment({})
        self.assertIn(key, environment)
        self.assertEqual(environment[key], value)


    def test_environment_update(self):
        """Test that the environment is updated"""

        key, value, update = [str(uuid.uuid4()) for _ in range(3)]
        update_environment = benchcomp.entry.run._EnvironmentUpdater({
            key: value,
        })
        environment = update_environment({
            key: update
        })
        self.assertIn(key, environment)
        self.assertEqual(environment[key], update)


    def test_environment_update_variable(self):
        """Test that the environment is updated"""

        old_env = {
            "key1": str(uuid.uuid4()),
            "key2": str(uuid.uuid4()),
        }

        actual_update = "${key2}xxx${key1}"
        expected_update = f"{old_env['key2']}xxx{old_env['key1']}"

        update_environment = benchcomp.entry.run._EnvironmentUpdater(old_env)
        environment = update_environment({
            "key1": actual_update,
        })
        self.assertIn("key1", environment)
        self.assertEqual(environment["key1"], expected_update)
