use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{CrateNum, DefId, CRATE_DEF_INDEX};
use rustc_middle::middle::privacy::{AccessLevel, AccessLevels};
use rustc_middle::ty::TyCtxt;
