// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{
    CIntType, DatatypeComponent, Expr, Location, Parameter, Stmt, StmtBody, Symbol, SymbolTable,
    SymbolValues, Transformer, Type,
};
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct GenCTransformer {
    new_symbol_table: SymbolTable,
}

impl GenCTransformer {
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        GenCTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

impl Transformer for GenCTransformer {
    fn symbol_table(&self) -> &SymbolTable {
        &self.new_symbol_table
    }

    fn mut_symbol_table(&mut self) -> &mut SymbolTable {
        &mut self.new_symbol_table
    }

    fn extract_symbol_table(self) -> SymbolTable {
        self.new_symbol_table
    }

    fn postprocess(&mut self) {
        let memcpy = self.mut_symbol_table().remove("memcpy");
        assert!(memcpy.is_some());
        let memmove = self.mut_symbol_table().remove("memmove");
        assert!(memmove.is_some());

        let old_main = self.symbol_table().lookup("main_");
        if let Some(old_main) = old_main {
            let new_main = Symbol::function(
                "main",
                Type::code(Vec::new(), Type::CInteger(CIntType::Int)),
                Some(Stmt::block(
                    vec![
                        Stmt::code_expression(
                            Expr::symbol_expression("main_".to_string(), old_main.typ.clone())
                                .call(Vec::new()),
                            Location::none(),
                        ),
                        Stmt::ret(
                            Some(Expr::int_constant(0, Type::CInteger(CIntType::Int))),
                            Location::none(),
                        ),
                    ],
                    Location::none(),
                )),
                Some("main".to_string()),
                Location::none(),
            );
            self.mut_symbol_table().insert(new_main);
        }
    }
}
