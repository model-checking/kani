// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[test]
fn uses_not_for_kani() {
    assert_eq!(agent_plugin_fixture::not_for_kani(), 1);
}
