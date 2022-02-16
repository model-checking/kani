//! Strips all private import statements (use, extern crate) from a
//! crate.
use crate::clean;
use crate::core::DocContext;
use crate::fold::DocFolder;
use crate::passes::{ImportStripper, Pass};
