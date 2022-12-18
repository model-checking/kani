// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains code that helps build tables more easily.
//!
//! Virtually all assess tables are built via the same method:
//!
//! 1. There is some input data with irregular structure
//! 2. There is an implied (we avoid writing it) type with regular structure
//! 3. There is the type of the table row
//!
//! As an abstract example, #1 is often something like `[(Node, [Node])]` which you can think of
//! as a graph. #2 is a normalized version like `[(Node, Node)]` which is an ordinary
//! adjacency list. #3 ends up being a structure like `(Node name, outgoing edge count)`.
//!
//! With this module, you construct a table by:
//!
//! 1. Create a type for a table row (#3 above).
//! 2. Write the code to traverse your irregular structure (#1) and produce table rows.
//! 3. Implement [`TableRow`] for your table row type, which implements
//!    an [`TableRow::merge`] method. This avoids having to create a type for
//!    the intermediate "regular" structure (#2 above), by instead thinking in
//!    terms of merging rows by their [`TableRow::key`]. (Vaguely like map-reduce.)
//! 4. Use [`TableBuilder`] to construct the table.
//! 5. [Optional] Implement [`RenderableTableRow`] so you can print the table.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;

use comfy_table::Table;

/// A [`TableRow`] is a type where multiple "rows" with the same `key` should be `merge`d into one.
/// This type is used in conjuction with [`TableBuilder`].
///
/// We further give an ordering (via `compare`) that defines how rows should appear in the final table.
pub trait TableRow {
    type Key: Eq + Hash;

    /// Returns the key for this row entry, so that multiple entries with the same key can be aggregated
    fn key(&self) -> Self::Key;
    /// Merges two rows into one. `self` is always the existing (or first) entry, `new` is what should be merged in.
    fn merge(&mut self, new: Self);
    /// Define the order rows should appear in a table. Not needed for merging, but required for producing results.
    fn compare(&self, right: &Self) -> Ordering;
}

/// What kind of data appears in a table column, when visually rendered for the user.
pub enum ColumnType {
    /// Text might be very wide and need limiting, and it should be left-justified.
    Text,
    /// Numbers should be right-justified.
    Number,
}

/// Describes how a table should be printed visually.
///
/// This is an extension to [`TableRow`] (which is solely concerned with computing a table's contents.)
/// In this trait, we are solely concerned with how the table should look when printed.
pub trait RenderableTableRow {
    /// Headers, e.g. " Reason for failure | Number of tests "
    fn headers() -> Vec<&'static str>;
    /// Column types, e.g. [Text, Number]
    fn columns() -> Vec<ColumnType>;
    /// The row contents, e.g. ["unwind", "1"]
    fn row(&self) -> Vec<String>;
}

/// Implements the basic algorithm for constructing tables by merging rows by their keys.
///
/// ```
/// let mut builder = TableBuilder::new();
///
/// for entry in some_data {
///     builder.add(MyTableRowType { ... });
/// }
///
/// builder.build()
/// // or
/// builder.render()
/// ```
pub struct TableBuilder<R>
where
    R: TableRow,
{
    map: HashMap<R::Key, R>,
}

impl<R: TableRow> TableBuilder<R> {
    /// Creates a new TableBuilder. `R` must implement `TableRow`.
    pub fn new() -> Self {
        TableBuilder { map: HashMap::new() }
    }

    /// Incrementally add new row data to the table, by merging it with any entry that
    /// shares its key.
    pub fn add(&mut self, row: R) {
        let entry = self.map.entry(row.key());
        // unfortunately ownership of row makes it somewhat difficult to use the entry methods,
        // so actually branch here so it's clear we only use 'row' once really
        use std::collections::hash_map::Entry::*;
        match entry {
            Occupied(o) => {
                o.into_mut().merge(row);
            }
            Vacant(v) => {
                v.insert(row);
            }
        }
    }

    /// Construct a pure-data table.
    pub fn build(&self) -> Vec<&R> {
        let mut values: Vec<_> = self.map.values().collect();
        values.sort_by(|a, b| a.compare(b));
        values
    }

    /// Returns a renderable `Table` for human consumption.
    ///
    /// This is the only part of `TableBuilder` that requires `RenderableTableRow`.
    pub fn render(&self) -> Table
    where
        R: RenderableTableRow,
    {
        use comfy_table::*;

        let mut table = assess_table_new();
        table.set_header(R::headers());
        for (index, ty) in R::columns().iter().enumerate() {
            let col = table.column_mut(index).unwrap();

            match ty {
                ColumnType::Text => {
                    col.set_cell_alignment(CellAlignment::Left);
                    col.set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(80)));
                }
                ColumnType::Number => {
                    col.set_cell_alignment(CellAlignment::Right);
                }
            }
        }

        for v in self.build() {
            table.add_row(v.row());
        }

        table
    }
}

/// Internal helper function for [`TableBuilder::render`] that sets our "comfy table" styling
fn assess_table_new() -> Table {
    use comfy_table::*;

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
        .load_preset(comfy_table::presets::NOTHING)
        .set_style(TableComponent::BottomBorder, '=')
        .set_style(TableComponent::BottomBorderIntersections, '=')
        .set_style(TableComponent::TopBorder, '=')
        .set_style(TableComponent::TopBorderIntersections, '=')
        .set_style(TableComponent::HeaderLines, '-')
        .set_style(TableComponent::MiddleHeaderIntersections, '+')
        .set_style(TableComponent::VerticalLines, '|');
    table
}
