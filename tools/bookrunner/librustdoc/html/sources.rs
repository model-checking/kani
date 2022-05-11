// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use crate::clean;
use crate::visit::DocVisitor;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_middle::ty::TyCtxt;
use rustc_session::Session;
use rustc_span::source_map::FileName;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

struct LocalSourcesCollector<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    local_sources: FxHashMap<PathBuf, String>,
    src_root: &'a Path,
}

fn is_real_and_local(span: clean::Span, sess: &Session) -> bool {
    span.cnum(sess) == LOCAL_CRATE && span.filename(sess).is_real()
}

impl LocalSourcesCollector<'_, '_> {
    fn add_local_source(&mut self, item: &clean::Item) {
        let sess = self.tcx.sess;
        let span = item.span(self.tcx);
        // skip all synthetic "files"
        if !is_real_and_local(span, sess) {
            return;
        }
        let filename = span.filename(sess);
        let p = if let FileName::Real(file) = filename {
            match file.into_local_path() {
                Some(p) => p,
                None => return,
            }
        } else {
            return;
        };
        if self.local_sources.contains_key(&*p) {
            // We've already emitted this source
            return;
        }

        let mut href = String::new();
        clean_path(self.src_root, &p, false, |component| {
            href.push_str(&component.to_string_lossy());
            href.push('/');
        });

        let mut src_fname = p.file_name().expect("source has no filename").to_os_string();
        src_fname.push(".html");
        href.push_str(&src_fname.to_string_lossy());
        self.local_sources.insert(p, href);
    }
}

impl DocVisitor for LocalSourcesCollector<'_, '_> {
    fn visit_item(&mut self, item: &clean::Item) {
        self.add_local_source(item);

        self.visit_item_recur(item)
    }
}

/// Takes a path to a source file and cleans the path to it. This canonicalizes
/// things like ".." to components which preserve the "top down" hierarchy of a
/// static HTML tree. Each component in the cleaned path will be passed as an
/// argument to `f`. The very last component of the path (ie the file name) will
/// be passed to `f` if `keep_filename` is true, and ignored otherwise.
crate fn clean_path<F>(src_root: &Path, p: &Path, keep_filename: bool, mut f: F)
where
    F: FnMut(&OsStr),
{
    // make it relative, if possible
    let p = p.strip_prefix(src_root).unwrap_or(p);

    let mut iter = p.components().peekable();

    while let Some(c) = iter.next() {
        if !keep_filename && iter.peek().is_none() {
            break;
        }

        match c {
            Component::ParentDir => f("up".as_ref()),
            Component::Normal(c) => f(c),
            _ => continue,
        }
    }
}
