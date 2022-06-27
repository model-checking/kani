// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
//! Markdown formatting for rustdoc.
//!

use rustc_hir::def_id::DefId;
use rustc_hir::HirId;
use rustc_middle::ty::TyCtxt;
use rustc_span::edition::Edition;
use rustc_span::Span;

use std::borrow::Cow;
use std::default::Default;
use std::str;

use crate::clean::RenderedLink;
use crate::doctest;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, LinkType, Parser, Tag};

#[derive(Copy, Clone, PartialEq, Debug)]
pub /* via find_testable_code */ enum ErrorCodes {
    Yes,
    No,
}

impl ErrorCodes {
    pub(crate) fn as_bool(self) -> bool {
        match self {
            ErrorCodes::Yes => true,
            ErrorCodes::No => false,
        }
    }
}

/// Controls whether a line will be hidden or shown in HTML output.
///
/// All lines are used in documentation tests.
enum Line<'a> {
    Hidden(&'a str),
    Shown(Cow<'a, str>),
}

impl<'a> Line<'a> {
    fn for_code(self) -> Cow<'a, str> {
        match self {
            Line::Shown(l) => l,
            Line::Hidden(l) => Cow::Borrowed(l),
        }
    }
}

// FIXME: There is a minor inconsistency here. For lines that start with ##, we
// have no easy way of removing a potential single space after the hashes, which
// is done in the single # case. This inconsistency seems okay, if non-ideal. In
// order to fix it we'd have to iterate to find the first non-# character, and
// then reallocate to remove it; which would make us return a String.
fn map_line(s: &str) -> Line<'_> {
    let trimmed = s.trim();
    if trimmed.starts_with("##") {
        Line::Shown(Cow::Owned(s.replacen("##", "#", 1)))
    } else if let Some(stripped) = trimmed.strip_prefix("# ") {
        // # text
        Line::Hidden(stripped)
    } else if trimmed == "#" {
        // We cannot handle '#text' because it could be #[attr].
        Line::Hidden("")
    } else {
        Line::Shown(Cow::Borrowed(s))
    }
}

/// Make headings links with anchor IDs and build up TOC.
struct LinkReplacer<'a, I: Iterator<Item = Event<'a>>> {
    inner: I,
    links: &'a [RenderedLink],
    shortcut_link: Option<&'a RenderedLink>,
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for LinkReplacer<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut event = self.inner.next();

        // Replace intra-doc links and remove disambiguators from shortcut links (`[fn@f]`).
        match &mut event {
            // This is a shortcut link that was resolved by the broken_link_callback: `[fn@f]`
            // Remove any disambiguator.
            Some(Event::Start(Tag::Link(
                // [fn@f] or [fn@f][]
                LinkType::ShortcutUnknown | LinkType::CollapsedUnknown,
                dest,
                title,
            ))) => {
                debug!("saw start of shortcut link to {} with title {}", dest, title);
                // If this is a shortcut link, it was resolved by the broken_link_callback.
                // So the URL will already be updated properly.
                let link = self.links.iter().find(|&link| *link.href == **dest);
                // Since this is an external iterator, we can't replace the inner text just yet.
                // Store that we saw a link so we know to replace it later.
                if let Some(link) = link {
                    trace!("it matched");
                    assert!(self.shortcut_link.is_none(), "shortcut links cannot be nested");
                    self.shortcut_link = Some(link);
                }
            }
            // Now that we're done with the shortcut link, don't replace any more text.
            Some(Event::End(Tag::Link(
                LinkType::ShortcutUnknown | LinkType::CollapsedUnknown,
                dest,
                _,
            ))) => {
                debug!("saw end of shortcut link to {}", dest);
                if self.links.iter().any(|link| *link.href == **dest) {
                    assert!(self.shortcut_link.is_some(), "saw closing link without opening tag");
                    self.shortcut_link = None;
                }
            }
            // Handle backticks in inline code blocks, but only if we're in the middle of a shortcut link.
            // [`fn@f`]
            Some(Event::Code(text)) => {
                trace!("saw code {}", text);
                if let Some(link) = self.shortcut_link {
                    trace!("original text was {}", link.original_text);
                    // NOTE: this only replaces if the code block is the *entire* text.
                    // If only part of the link has code highlighting, the disambiguator will not be removed.
                    // e.g. [fn@`f`]
                    // This is a limitation from `collect_intra_doc_links`: it passes a full link,
                    // and does not distinguish at all between code blocks.
                    // So we could never be sure we weren't replacing too much:
                    // [fn@my_`f`unc] is treated the same as [my_func()] in that pass.
                    //
                    // NOTE: &[1..len() - 1] is to strip the backticks
                    if **text == link.original_text[1..link.original_text.len() - 1] {
                        debug!("replacing {} with {}", text, link.new_text);
                        *text = CowStr::Borrowed(&link.new_text);
                    }
                }
            }
            // Replace plain text in links, but only in the middle of a shortcut link.
            // [fn@f]
            Some(Event::Text(text)) => {
                trace!("saw text {}", text);
                if let Some(link) = self.shortcut_link {
                    trace!("original text was {}", link.original_text);
                    // NOTE: same limitations as `Event::Code`
                    if **text == *link.original_text {
                        debug!("replacing {} with {}", text, link.new_text);
                        *text = CowStr::Borrowed(&link.new_text);
                    }
                }
            }
            // If this is a link, but not a shortcut link,
            // replace the URL, since the broken_link_callback was not called.
            Some(Event::Start(Tag::Link(_, dest, _))) => {
                if let Some(link) = self.links.iter().find(|&link| *link.original_text == **dest) {
                    *dest = CowStr::Borrowed(link.href.as_ref());
                }
            }
            // Anything else couldn't have been a valid Rust path, so no need to replace the text.
            _ => {}
        }

        // Yield the modified event
        event
    }
}

pub fn find_testable_code<T: doctest::Tester>(
    doc: &str,
    tests: &mut T,
    error_codes: ErrorCodes,
    enable_per_target_ignores: bool,
    extra_info: Option<&ExtraInfo<'_>>,
) {
    let mut parser = Parser::new(doc).into_offset_iter();
    let mut prev_offset = 0;
    let mut nb_lines = 0;
    let mut register_header = None;
    while let Some((event, offset)) = parser.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let block_info = match kind {
                    CodeBlockKind::Fenced(ref lang) => {
                        if lang.is_empty() {
                            Default::default()
                        } else {
                            LangString::parse(
                                lang,
                                error_codes,
                                enable_per_target_ignores,
                                extra_info,
                            )
                        }
                    }
                    CodeBlockKind::Indented => Default::default(),
                };
                if !block_info.rust {
                    continue;
                }

                let mut test_s = String::new();

                while let Some((Event::Text(s), _)) = parser.next() {
                    test_s.push_str(&s);
                }
                let text = test_s
                    .lines()
                    .map(|l| map_line(l).for_code())
                    .collect::<Vec<Cow<'_, str>>>()
                    .join("\n");

                nb_lines += doc[prev_offset..offset.start].lines().count();
                // If there are characters between the preceding line ending and
                // this code block, `str::lines` will return an additional line,
                // which we subtract here.
                if nb_lines != 0 && !&doc[prev_offset..offset.start].ends_with('\n') {
                    nb_lines -= 1;
                }
                let line = tests.get_line() + nb_lines + 1;
                tests.add_test(text, block_info, line);
                prev_offset = offset.start;
            }
            Event::Start(Tag::Heading(level, _, _)) => {
                register_header = Some(level as u32);
            }
            Event::Text(ref s) if register_header.is_some() => {
                let level = register_header.unwrap();
                if s.is_empty() {
                    tests.register_header("", level);
                } else {
                    tests.register_header(s, level);
                }
                register_header = None;
            }
            _ => {}
        }
    }
}

pub struct ExtraInfo<'tcx> {
    id: ExtraInfoId,
    sp: Span,
    tcx: TyCtxt<'tcx>,
}

enum ExtraInfoId {
    Hir(HirId),
    Def(DefId),
}

impl<'tcx> ExtraInfo<'tcx> {
    pub(crate) fn new(tcx: TyCtxt<'tcx>, hir_id: HirId, sp: Span) -> ExtraInfo<'tcx> {
        ExtraInfo { id: ExtraInfoId::Hir(hir_id), sp, tcx }
    }

    fn error_invalid_codeblock_attr(&self, msg: &str, help: &str) {
        let hir_id = match self.id {
            ExtraInfoId::Hir(hir_id) => hir_id,
            ExtraInfoId::Def(item_did) => {
                match item_did.as_local() {
                    Some(item_did) => self.tcx.hir().local_def_id_to_hir_id(item_did),
                    None => {
                        // If non-local, no need to check anything.
                        return;
                    }
                }
            }
        };
        self.tcx.struct_span_lint_hir(
            crate::lint::INVALID_CODEBLOCK_ATTRIBUTES,
            hir_id,
            self.sp,
            |lint| {
                let mut diag = lint.build(msg);
                diag.help(help);
                diag.emit();
            },
        );
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct LangString {
    original: String,
    pub should_panic: bool,
    pub(crate) no_run: bool,
    pub ignore: Ignore,
    pub(crate) rust: bool,
    pub(crate) test_harness: bool,
    pub compile_fail: bool,
    pub(crate) error_codes: Vec<String>,
    pub(crate) allow_fail: bool,
    pub edition: Option<Edition>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Ignore {
    All,
    None,
    Some(Vec<String>),
}

impl Default for LangString {
    fn default() -> Self {
        Self {
            original: String::new(),
            should_panic: false,
            no_run: false,
            ignore: Ignore::None,
            rust: true,
            test_harness: false,
            compile_fail: false,
            error_codes: Vec::new(),
            allow_fail: false,
            edition: None,
        }
    }
}

impl LangString {
    fn tokens(string: &str) -> impl Iterator<Item = &str> {
        // Pandoc, which Rust once used for generating documentation,
        // expects lang strings to be surrounded by `{}` and for each token
        // to be proceeded by a `.`. Since some of these lang strings are still
        // loose in the wild, we strip a pair of surrounding `{}` from the lang
        // string and a leading `.` from each token.

        let string = string.trim();

        let first = string.chars().next();
        let last = string.chars().last();

        let string = if first == Some('{') && last == Some('}') {
            &string[1..string.len() - 1]
        } else {
            string
        };

        string
            .split(|c| c == ',' || c == ' ' || c == '\t')
            .map(str::trim)
            .map(|token| token.strip_prefix('.').unwrap_or(token))
            .filter(|token| !token.is_empty())
    }

    fn parse(
        string: &str,
        allow_error_code_check: ErrorCodes,
        enable_per_target_ignores: bool,
        extra: Option<&ExtraInfo<'_>>,
    ) -> LangString {
        let allow_error_code_check = allow_error_code_check.as_bool();
        let mut seen_rust_tags = false;
        let mut seen_other_tags = false;
        let mut data = LangString::default();
        let mut ignores = vec![];

        data.original = string.to_owned();

        for token in Self::tokens(string) {
            match token {
                "should_panic" => {
                    data.should_panic = true;
                    seen_rust_tags = !seen_other_tags;
                }
                "no_run" => {
                    data.no_run = true;
                    seen_rust_tags = !seen_other_tags;
                }
                "ignore" => {
                    data.ignore = Ignore::All;
                    seen_rust_tags = !seen_other_tags;
                }
                x if x.starts_with("ignore-") => {
                    if enable_per_target_ignores {
                        ignores.push(x.trim_start_matches("ignore-").to_owned());
                        seen_rust_tags = !seen_other_tags;
                    }
                }
                "allow_fail" => {
                    data.allow_fail = true;
                    seen_rust_tags = !seen_other_tags;
                }
                "rust" => {
                    data.rust = true;
                    seen_rust_tags = true;
                }
                "test_harness" => {
                    data.test_harness = true;
                    seen_rust_tags = !seen_other_tags || seen_rust_tags;
                }
                "compile_fail" => {
                    data.compile_fail = true;
                    seen_rust_tags = !seen_other_tags || seen_rust_tags;
                    data.no_run = true;
                }
                x if x.starts_with("edition") => {
                    data.edition = x[7..].parse::<Edition>().ok();
                }
                x if allow_error_code_check && x.starts_with('E') && x.len() == 5 => {
                    if x[1..].parse::<u32>().is_ok() {
                        data.error_codes.push(x.to_owned());
                        seen_rust_tags = !seen_other_tags || seen_rust_tags;
                    } else {
                        seen_other_tags = true;
                    }
                }
                x if extra.is_some() => {
                    let s = x.to_lowercase();
                    if let Some((flag, help)) = if s == "compile-fail"
                        || s == "compile_fail"
                        || s == "compilefail"
                    {
                        Some((
                            "compile_fail",
                            "the code block will either not be tested if not marked as a rust one \
                             or won't fail if it compiles successfully",
                        ))
                    } else if s == "should-panic" || s == "should_panic" || s == "shouldpanic" {
                        Some((
                            "should_panic",
                            "the code block will either not be tested if not marked as a rust one \
                             or won't fail if it doesn't panic when running",
                        ))
                    } else if s == "no-run" || s == "no_run" || s == "norun" {
                        Some((
                            "no_run",
                            "the code block will either not be tested if not marked as a rust one \
                             or will be run (which you might not want)",
                        ))
                    } else if s == "allow-fail" || s == "allow_fail" || s == "allowfail" {
                        Some((
                            "allow_fail",
                            "the code block will either not be tested if not marked as a rust one \
                             or will be run (which you might not want)",
                        ))
                    } else if s == "test-harness" || s == "test_harness" || s == "testharness" {
                        Some((
                            "test_harness",
                            "the code block will either not be tested if not marked as a rust one \
                             or the code will be wrapped inside a main function",
                        ))
                    } else {
                        None
                    } {
                        if let Some(extra) = extra {
                            extra.error_invalid_codeblock_attr(
                                &format!("unknown attribute `{}`. Did you mean `{}`?", x, flag),
                                help,
                            );
                        }
                    }
                    seen_other_tags = true;
                }
                _ => seen_other_tags = true,
            }
        }

        // ignore-foo overrides ignore
        if !ignores.is_empty() {
            data.ignore = Ignore::Some(ignores);
        }

        data.rust &= !seen_other_tags || seen_rust_tags;

        data
    }
}
