// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for processing Rust attributes (like `kani::proof`).

use rustc_ast::{AttrKind, Attribute, LitKind, MetaItem};

/// Partition all the attributes into two buckets, proof_attributes and other_attributes
pub fn partition_kanitool_attributes(
    all_attributes: &[Attribute],
) -> (Vec<&Attribute>, Vec<(String, &Attribute)>) {
    let mut proof_attributes = vec![];
    let mut other_attributes = vec![];

    for attr in all_attributes {
        // Get the string the appears after "kanitool::" in each attribute string.
        // Ex - "proof" | "unwind" etc.
        if let Some(attribute_string) = kanitool_attr_name(attr).as_deref() {
            if attribute_string == "proof" {
                proof_attributes.push(attr);
            } else {
                other_attributes.push((attribute_string.to_string(), attr));
            }
        }
    }

    (proof_attributes, other_attributes)
}

/// Extracts the integer value argument from the attribute provided
/// For example, `unwind(8)` return `Some(8)`
pub fn extract_integer_argument(attr: &Attribute) -> Option<u128> {
    // Vector of meta items , that contain the arguments given the attribute
    let attr_args = attr.meta_item_list()?;
    // Only extracts one integer value as argument
    if attr_args.len() == 1 {
        let x = attr_args[0].literal()?;
        match x.kind {
            LitKind::Int(y, ..) => Some(y),
            _ => None,
        }
    }
    // Return none if there are no attributes or if there's too many attributes
    else {
        None
    }
}

/// Extracts a vector with the path arguments of an attribute.
/// The length of the returned vector is equal to the number of arguments in the
/// attribute; an entry is `None` if the argument is not syntactically a path,
/// and `Some(<path>)` otherwise. Paths are returned as strings.
///
/// For example, on `stub(foo::bar, 42, baz)`, this returns
/// `vec![Some("foo::bar"), None, Some("baz")]`.
pub fn extract_path_arguments(attr: &Attribute) -> Vec<Option<String>> {
    let attr_args = attr.meta_item_list();
    if attr_args.is_none() {
        return vec![];
    }
    let mut paths = Vec::new();
    for arg in attr_args.unwrap() {
        let entry = arg.meta_item().map(extract_path).flatten();
        paths.push(entry)
    }
    paths
}

/// Extracts a path from an attribute item, returning `None` if the item is not
/// syntactically a path.
fn extract_path(meta_item: &MetaItem) -> Option<String> {
    if meta_item.is_word() {
        Some(
            meta_item
                .path
                .segments
                .iter()
                .map(|seg| seg.ident.as_str())
                .collect::<Vec<&str>>()
                .join("::"),
        )
    } else {
        None
    }
}

/// If the attribute is named `kanitool::name`, this extracts `name`
fn kanitool_attr_name(attr: &Attribute) -> Option<String> {
    match &attr.kind {
        AttrKind::Normal(normal) => {
            let segments = &normal.item.path.segments;
            if (!segments.is_empty()) && segments[0].ident.as_str() == "kanitool" {
                Some(segments[1].ident.as_str().to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}
