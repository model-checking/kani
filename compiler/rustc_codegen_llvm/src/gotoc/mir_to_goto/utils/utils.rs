// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::btree_string_map;
use crate::gotoc::cbmc::goto_program::{Expr, Location, Stmt, SymbolTable, Type};
use crate::gotoc::mir_to_goto::GotocCtx;

// Should move into rvalue
//make this a member function
pub fn slice_fat_ptr(typ: Type, data: Expr, len: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("len", len)], symbol_table)
}

pub fn dynamic_fat_ptr(typ: Type, data: Expr, vtable: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("vtable", vtable)], symbol_table)
}

impl<'tcx> GotocCtx<'tcx> {
    /// RMC does not currently support all MIR constructs.
    /// When we hit a construct we don't handle, we have two choices:
    /// We can use the `unimplemented!()` macro, which causes a compile time failure.
    /// Or, we can use this function, which inserts an `assert(false, "FOO is not currently supported by RMC")` into the generated code.
    /// This means that if the unimplemented feature is dynamically used by the code being verified, we will see an assertion failure.
    /// If it is not used, we the assertion will pass.
    /// This allows us to continue to make progress parsing rust code, while remaining sound (thanks to the `assert(false)`)
    ///
    /// TODO: https://github.com/model-checking/rmc/issues/8 assume the required validity constraints for the nondet return
    /// TODO: https://github.com/model-checking/rmc/issues/9 Have a parameter that decides whether to `assume(0)` to block further traces or not
    pub fn codegen_unimplemented(
        &mut self,
        operation_name: &str,
        t: Type,
        loc: Location,
        url: &str,
    ) -> Expr {
        let body = vec![
            // Assert false to alert the user that there is a path that uses an unimplemented feature.
            Stmt::assert_false(
                &format!(
                    "{} is not currently supported by RMC. Please post your example at {} ",
                    operation_name, url
                ),
                loc.clone(),
            ),
            // Assume false to block any further exploration of this path.
            Stmt::assume(Expr::bool_false(), loc.clone()),
            t.nondet().as_stmt(loc.clone()).with_location(loc.clone()), //TODO assume rust validity contraints
        ];

        Expr::statement_expression(body, t).with_location(loc)
    }
}

impl<'tcx> GotocCtx<'tcx> {
    /// Dereference a boxed type `std::boxed::Box<T>` to get a `*T`.
    ///
    /// WARNING: This is based on a manual inspection of how boxed types are currently
    /// a) implemented by the rust standard library
    /// b) codegenned by RMC.
    /// If either of those change, this will almost certainly stop working.
    pub fn deref_box(&self, e: Expr) -> Expr {
        // Internally, a Boxed type is stored as a chain of structs.
        // In particular:
        // `Box<T>` is an owning reference to an allocation of type T on the heap.
        // It has a pointer of type `ptr::Unique<T>` and an allocator of type `alloc::Global`
        // Unique<T> is an owning raw pointer to a location in memory.
        // So given a Box<T>, we can follow the chain to get the desired pointer.
        // If either rustc or RMC changes how boxed types are represented, this will need to be
        // updated.
        //
        // The following C code is the result of running `rmc --gen-c` on rust with boxed types:
        // Given a boxed type (note that Rust can reorder fields to improve struct packing):
        // ```
        // struct std::boxed::Box<[u8]>
        // {
        //   struct std::alloc::Global 1;
        //   struct std::ptr::Unique<[u8]> 0;
        // };
        // ```
        // We follow the Unique pointer:
        // ```
        // struct std::ptr::Unique<[u8]>
        // {
        //   struct std::marker::PhantomData<[u8]> _marker;
        //   struct &[u8] pointer;
        // };
        // ```
        // And notice that its `.pointer` field is exactly what we want.
        assert!(e.typ().is_rust_box(), "expected rust box {:?}", e);
        let unique_ptr_typ =
            self.symbol_table.lookup_field_type_in_type(e.typ(), "0").unwrap().clone();
        assert!(
            unique_ptr_typ.is_rust_unique_pointer(),
            "{:?}\n\t{}",
            unique_ptr_typ,
            self.current_fn().readable_name()
        );
        e.member("0", &self.symbol_table).member("pointer", &self.symbol_table)
    }

    /// Box<T> initializer
    /// `boxed_type the_box = >>> { .0=nondet(), .1={ ._marker=nondet(), .pointer=boxed_value } } <<<`
    /// `boxed_type` is the type of the resulting expression
    pub fn box_value(&self, boxed_value: Expr, boxed_type: Type) -> Expr {
        assert!(boxed_type.is_rust_box(), "expected rust box {:?}", boxed_type);
        let get_field_type = |struct_typ, field| {
            self.symbol_table.lookup_field_type_in_type(struct_typ, field).unwrap().clone()
        };
        let unique_ptr_typ = get_field_type(&boxed_type, "0");
        assert!(unique_ptr_typ.is_rust_unique_pointer());
        let unique_ptr_pointer_typ = get_field_type(&unique_ptr_typ, "pointer");
        assert_eq!(&unique_ptr_pointer_typ, boxed_value.typ());
        let unique_ptr_val = Expr::struct_expr_with_nondet_fields(
            unique_ptr_typ,
            btree_string_map![("pointer", boxed_value),],
            &self.symbol_table,
        );
        let boxed_val = Expr::struct_expr_with_nondet_fields(
            boxed_type,
            btree_string_map![("0", unique_ptr_val),],
            &self.symbol_table,
        );
        boxed_val
    }
}

impl Type {
    /// Best effort check if the struct represents a Rust "Box". May return false positives.
    pub fn is_rust_box(&self) -> bool {
        // We have seen variants on the name, including
        // tag-std::boxed::Box, tag-core::boxed::Box, tag-boxed::Box.
        // If we match on exact names, we're playing whack-a-mole trying to keep track of how this
        // can be reimported.
        // If we don't, we spuriously fail. https://github.com/model-checking/rmc/issues/113
        // TODO: find a better way of checking this https://github.com/model-checking/rmc/issues/152
        self.type_name().map_or(false, |name| name.contains("boxed::Box"))
    }

    /// Checks if the struct represents a Rust "Unique"
    pub fn is_rust_unique_pointer(&self) -> bool {
        self.type_name().map_or(false, |name| {
            name.starts_with("tag-std::ptr::Unique")
                || name.starts_with("tag-core::ptr::Unique")
                || name.starts_with("tag-rustc_std_workspace_core::ptr::Unique")
        })
    }

    pub fn is_rust_slice_fat_ptr(&self, st: &SymbolTable) -> bool {
        match self {
            Type::Struct { components, .. } => {
                components.len() == 2
                    && components.iter().any(|x| x.name() == "data" && x.typ().is_pointer())
                    && components.iter().any(|x| x.name() == "len" && x.typ().is_integer())
            }
            Type::StructTag(tag) => st.lookup(tag).unwrap().typ.is_rust_slice_fat_ptr(st),
            _ => false,
        }
    }

    pub fn is_rust_trait_fat_ptr(&self, st: &SymbolTable) -> bool {
        match self {
            Type::Struct { components, .. } => {
                components.len() == 2
                    && components.iter().any(|x| x.name() == "data" && x.typ().is_pointer())
                    && components.iter().any(|x| x.name() == "vtable" && x.typ().is_pointer())
            }
            Type::StructTag(tag) => st.lookup(tag).unwrap().typ.is_rust_trait_fat_ptr(st),
            _ => false,
        }
    }

    pub fn is_rust_fat_ptr(&self, st: &SymbolTable) -> bool {
        self.is_rust_slice_fat_ptr(st) || self.is_rust_trait_fat_ptr(st)
    }
}
