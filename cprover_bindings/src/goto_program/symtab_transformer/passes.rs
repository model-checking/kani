// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::gen_c_transformer::{ExprTransformer, NameTransformer, NondetTransformer};
use super::identity_transformer::IdentityTransformer;
use crate::goto_program::SymbolTable;

/// Performs each pass provided on the given symbol table.
pub fn do_passes(mut symtab: SymbolTable, pass_names: &[String]) -> SymbolTable {
    for pass_name in pass_names {
        symtab = match &pass_name[..] {
            "gen-c" => {
                // Note: the order of these DOES matter;
                // ExprTransformer expects the NondetTransformer to happen after, and
                // NameTransformer should clean up any identifiers introduced by
                // the other two identifiers
                let symtab = ExprTransformer::transform(&symtab);
                let symtab = NondetTransformer::transform(&symtab);
                let symtab = NameTransformer::transform(&symtab);
                symtab
            }
            "identity" => IdentityTransformer::transform(&symtab),
            _ => panic!("Invalid symbol table transformation: {}", pass_name),
        }
    }

    symtab
}
