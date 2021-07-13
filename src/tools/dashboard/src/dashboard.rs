// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Data-structures representing the dashboard and and their utilities.

use std::fmt::{Display, Write};

/// This data-structure holds test results and info for each directory and test
/// in the testing suite.
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

/// Tree data-structure representing the dashboard.
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

    /// Merges two trees, if they have equal roots, and returns the merged tree.
    pub fn merge(mut l: Tree, r: Tree) -> Option<Tree> {
        // For each subtree of `r`...
        for cnr in r.children {
            // Look for a subtree of `l` with an equal root.
            let index = l.children.iter().position(|cnl| cnl.data.name == cnr.data.name);
            if let Some(index) = index {
                // If you find one, merge it with `r`'s subtree.
                let cnl = l.children.remove(index);
                l.children.insert(index, Tree::merge(cnl, cnr).unwrap());
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
    fn fmt_aux(&self, p: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not print line numbers.
        if self.children.len() == 0 {
            return Ok(());
        }
        // Write `p` spaces into the formatter.
        f.write_fmt(format_args!("{:1$}", "", p))?;
        f.write_str(&self.data.name)?;
        let num = self.data.num_pass;
        if num > 0 {
            f.write_fmt(format_args!(" ✔️ {}", num))?;
        }
        let num = self.data.num_fail;
        if num > 0 {
            f.write_fmt(format_args!(" ❌ {}", num))?;
        }
        f.write_char('\n')?;
        for cn in &self.children {
            cn.fmt_aux(p + 2, f)?;
        }
        Ok(())
    }
}

impl Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_aux(0, f)
    }
}
