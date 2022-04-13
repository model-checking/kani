// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// See GitHub history for details.
//! Propagates [`#[doc(cfg(...))]`](https://github.com/rust-lang/rust/issues/43781) to child items.
use std::sync::Arc;

use crate::clean::cfg::Cfg;
use crate::clean::Item;
use crate::fold::DocFolder;

struct CfgPropagator {
    parent_cfg: Option<Arc<Cfg>>,
}

impl DocFolder for CfgPropagator {
    fn fold_item(&mut self, mut item: Item) -> Option<Item> {
        let old_parent_cfg = self.parent_cfg.clone();

        let new_cfg = match (self.parent_cfg.take(), item.cfg.take()) {
            (None, None) => None,
            (Some(rc), None) | (None, Some(rc)) => Some(rc),
            (Some(mut a), Some(b)) => {
                let b = Arc::try_unwrap(b).unwrap_or_else(|rc| Cfg::clone(&rc));
                *Arc::make_mut(&mut a) &= b;
                Some(a)
            }
        };
        self.parent_cfg = new_cfg.clone();
        item.cfg = new_cfg;

        let result = self.fold_item_recur(item);
        self.parent_cfg = old_parent_cfg;

        Some(result)
    }
}
