// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use rustc_middle::mir::{Local, VarDebugInfo, VarDebugInfoContents};
use rustc_span::Span;

use super::cbmc::goto_program::{Expr, Location, SymbolTable, Type};
use super::metadata::*;

use crate::btree_string_map;

pub fn slice_fat_ptr(typ: Type, data: Expr, len: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("len", len)], symbol_table)
}

pub fn dynamic_fat_ptr(typ: Type, data: Expr, vtable: Expr, symbol_table: &SymbolTable) -> Expr {
    Expr::struct_expr(typ, btree_string_map![("data", data), ("vtable", vtable)], symbol_table)
}

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_var_name(&self, l: &Local) -> String {
        let fname = self.fname();
        match self.find_debug_info(l) {
            Some(info) => format!("{}::1::var{:?}::{}", fname, l, info.name),
            None => format!("{}::1::var{:?}", fname, l),
        }
    }

    pub fn find_debug_info(&self, l: &Local) -> Option<&VarDebugInfo<'tcx>> {
        self.mir().var_debug_info.iter().find(|info| match info.value {
            VarDebugInfoContents::Place(p) => p.local == *l && p.projection.len() == 0,
            VarDebugInfoContents::Const(_) => false,
        })
    }

    //TODO fix this name
    pub fn codegen_span_option2(&self, sp: Option<Span>) -> Location {
        sp.map_or(Location::none(), |x| self.codegen_span2(&x))
    }

    //TODO fix this name
    pub fn codegen_span2(&self, sp: &Span) -> Location {
        let smap = self.tcx.sess.source_map();
        let lo = smap.lookup_char_pos(sp.lo());
        let line = lo.line;
        let col = 1 + lo.col_display;
        Location::new(lo.file.name.to_string(), self.fname_option(), line, Some(col))
    }

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
        assert!(e.typ().is_rust_box());
        let unique_ptr_typ =
            self.symbol_table.lookup_field_type_in_type(e.typ(), "0").unwrap().clone();
        assert!(unique_ptr_typ.is_rust_unique_pointer());
        e.member("0", &self.symbol_table).member("pointer", &self.symbol_table)
    }

    /// Box<T> initializer
    /// `boxed_type the_box = >>> { .0=nondet(), .1={ ._marker=nondet(), .pointer=boxed_value } } <<<`
    /// `boxed_type` is the type of the resulting expression
    pub fn box_value(&self, boxed_value: Expr, boxed_type: Type) -> Expr {
        assert!(boxed_type.is_rust_box());
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
    /// Checks if the struct represents a Rust "Box"
    pub fn is_rust_box(&self) -> bool {
        self.type_name().map_or(false, |name| {
            name.starts_with("tag-std::boxed::Box") || name.starts_with("tag-core::boxed::Box")
        })
    }

    /// Checks if the struct represents a Rust "Unique"
    pub fn is_rust_unique_pointer(&self) -> bool {
        self.type_name().map_or(false, |name| {
            name.starts_with("tag-std::ptr::Unique") || name.starts_with("tag-core::ptr::Unique")
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
