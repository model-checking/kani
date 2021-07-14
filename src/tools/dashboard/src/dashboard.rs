// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Data structures representing the dashboard and their utilities.

use std::fmt::{Display, Formatter, Result, Write};

/// This data structure holds the results of running a test or a suite.
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub num_pass: u32,
    pub num_fail: u32,
}

impl Node {
    /// Creates a new test [`Node`].
    pub fn new(name: String, num_pass: u32, num_fail: u32) -> Node {
        Node { name, num_pass, num_fail }
    }
}

/// Tree data structure representing a confidence dashboard. `children`
/// represent sub-tests and sub-suites of the current test suite. This tree
/// structure allows us to collect and display a summary for test results in an
/// organized manner.
#[derive(Clone, Debug)]
pub struct Tree {
    pub data: Node,
    pub children: Vec<Tree>,
}

impl Tree {
    /// Creates a new [`Tree`] representing a dashboard or a part of it.
    pub fn new(data: Node, children: Vec<Tree>) -> Tree {
        Tree { data, children }
    }

    /// Merges two trees, if their root have equal node names, and returns the
    /// merged tree.
    pub fn merge(mut l: Tree, r: Tree) -> Option<Tree> {
        if l.data.name != r.data.name {
            return None;
        }
        // For each subtree of `r`...
        for cnr in r.children {
            // Look for a subtree of `l` with an equal root node name.
            let index = l.children.iter().position(|cnl| cnl.data.name == cnr.data.name);
            if let Some(index) = index {
                // If you find one, merge it with `r`'s subtree.
                let cnl = l.children.remove(index);
                l.children.insert(index, Tree::merge(cnl, cnr)?);
            } else {
                // Otherwise, `r`'s subtree is new. So, add it to `l`'s
                // list of subtrees.
                l.children.push(cnr);
            }
        }
        Some(Tree::new(
            Node::new(
                l.data.name,
                l.data.num_pass + r.data.num_pass,
                l.data.num_fail + r.data.num_fail,
            ),
            l.children,
        ))
    }

    /// A helper format function that indents each level of the tree.
    fn fmt_aux(&self, p: usize, f: &mut Formatter<'_>) -> Result {
        // Do not print line numbers.
        if self.children.len() == 0 {
            return Ok(());
        }
        // Write `p` spaces into the formatter.
        f.write_fmt(format_args!("{:1$}", "", p))?;
        f.write_str(&self.data.name)?;
        if self.data.num_pass > 0 {
            f.write_fmt(format_args!(" ✔️ {}", self.data.num_pass))?;
        }
        if self.data.num_fail > 0 {
            f.write_fmt(format_args!(" ❌ {}", self.data.num_fail))?;
        }
        f.write_char('\n')?;
        for cn in &self.children {
            cn.fmt_aux(p + 2, f)?;
        }
        Ok(())
    }
}

impl Display for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.fmt_aux(0, f)
    }
}
