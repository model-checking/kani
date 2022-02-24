//! Strip all doc(hidden) items from the output.
use rustc_span::symbol::sym;
use std::mem;

use crate::clean;
use crate::clean::{Item, ItemIdSet, NestedAttributesExt};
use crate::fold::{strip_item, DocFolder};

struct Stripper<'a> {
    retained: &'a mut ItemIdSet,
    update_retained: bool,
}

impl<'a> DocFolder for Stripper<'a> {
    fn fold_item(&mut self, i: Item) -> Option<Item> {
        if i.attrs.lists(sym::doc).has_word(sym::hidden) {
            debug!("strip_hidden: stripping {:?} {:?}", i.type_(), i.name);
            // use a dedicated hidden item for given item type if any
            match *i.kind {
                clean::StructFieldItem(..) | clean::ModuleItem(..) => {
                    // We need to recurse into stripped modules to
                    // strip things like impl methods but when doing so
                    // we must not add any items to the `retained` set.
                    let old = mem::replace(&mut self.update_retained, false);
                    let ret = strip_item(self.fold_item_recur(i));
                    self.update_retained = old;
                    return Some(ret);
                }
                _ => return None,
            }
        } else {
            if self.update_retained {
                self.retained.insert(i.def_id);
            }
        }
        Some(self.fold_item_recur(i))
    }
}
