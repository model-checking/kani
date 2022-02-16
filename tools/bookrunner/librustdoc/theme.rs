use rustc_data_structures::fx::FxHashSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use rustc_errors::Handler;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Eq)]
crate struct CssPath {
    crate name: String,
    crate children: FxHashSet<CssPath>,
}

// This PartialEq implementation IS NOT COMMUTATIVE!!!
//
// The order is very important: the second object must have all first's rules.
// However, the first is not required to have all of the second's rules.
impl PartialEq for CssPath {
    fn eq(&self, other: &CssPath) -> bool {
        if self.name != other.name {
            false
        } else {
            for child in &self.children {
                if !other.children.iter().any(|c| child == c) {
                    return false;
                }
            }
            true
        }
    }
}

impl Hash for CssPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        for x in &self.children {
            x.hash(state);
        }
    }
}

impl CssPath {
    fn new(name: String) -> CssPath {
        CssPath { name, children: FxHashSet::default() }
    }
}

/// All variants contain the position they occur.
#[derive(Debug, Clone, Copy)]
enum Events {
    StartLineComment(usize),
    StartComment(usize),
    EndComment(usize),
    InBlock(usize),
    OutBlock(usize),
}

impl Events {
}
