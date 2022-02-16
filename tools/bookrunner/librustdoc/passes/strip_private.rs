//! Strip all private items from the output. Additionally implies strip_priv_imports.
//! Basically, the goal is to remove items that are not relevant for public documentation.
use crate::clean::{self, ItemIdSet};
use crate::core::DocContext;
use crate::fold::DocFolder;
use crate::passes::{ImplStripper, ImportStripper, Pass, Stripper};
