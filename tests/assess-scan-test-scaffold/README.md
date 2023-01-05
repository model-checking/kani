# cargo kani assess scan test scaffold

This directory contains two totally unrelated (i.e. no shared workspace) packages.
Its purpose is to test "assess scan" running in a directory containing such.

This is used from `kani-regression.sh` to include a test for scan in CI.
The only thing we currently check is that the exit status was success and that files were emitted.
We presently don't have a way to run an arbitrary command with e.g. compiletest and expect.

