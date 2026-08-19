//! The classes this repository has already named, as executable prohibitions.
//!
//! # Why this file exists
//!
//! The 9 August three-repository report's finding is not a list of bugs. It is a
//! habit, and it counted eighteen instances of it:
//!
//! > the diagnosis is written down correctly and the sweep never runs
//!
//! A class named in prose, one member fixed, the siblings left standing. Both
//! applications have an instrument for turning a named class into an executable
//! prohibition — Ksav invented it, in `runner.test.mjs` — and both had scoped it
//! to one directory of one language. Each of the three repositories carries this
//! file now, and the rule for the future is the second half of it: **when a
//! finding names a class, the commit adds the sweep.**
//!
//! # What this repository in particular is for
//!
//! It exists so the two applications need not agree in prose. Every prohibition
//! below is a way that agreement leaks back out: a character table written a
//! second time, an escaper with its own opinion, a name spelled the way the
//! other repository forbids. This crate is the smallest of the three and it is
//! the one where a duplicate does the most damage, because a duplicate here is
//! a disagreement between two products rather than inside one.
//!
//! # How a prohibition is written here
//!
//! Comments are stripped before matching — every paragraph here that explains
//! what the old arrangement was would otherwise trip the rule that forbids it —
//! and an exemption is a **claim with a test attached**: an owner that stops
//! containing what it owns turns this red too.

use std::path::{Path, PathBuf};

/// The repository root: this crate is `crates/girsa-ksav`.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|e| panic!("the repository root resolves: {e}"))
}

/// Every `.rs` file in the workspace, repo-relative, with comments stripped.
fn sources(root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let base = entry.file_name();
            let base = base.to_string_lossy().to_string();
            if path.is_dir() {
                if base == "target" || base == ".git" || base == "node_modules" {
                    continue;
                }
                walk(&path, root, out);
            } else if base.ends_with(".rs") {
                // This file states each forbidden pattern in order to look for
                // it, which is the one exemption every prohibition sweep needs
                // and the only one any of them has.
                if base == "prohibitions.rs" {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let shown = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((shown, strip(&text)));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// Comments out.
///
/// A `//` must open the line, or a `///` doc comment three lines above a rule
/// would be read as the rule. Rust has no line-initial-only block comment worth
/// worrying about here, and `/* */` is not used in this workspace.
fn strip(s: &str) -> String {
    s.lines()
        .map(str::trim_start)
        .filter(|l| !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One prohibition: the class, the fragments that spell it, and its owners.
struct Rule {
    what: &'static str,
    /// Any of these appearing in a file is a breach.
    fragments: &'static [&'static str],
    /// Path **prefixes** that are allowed to contain it. A prefix rather than
    /// an exact path, because the owner of a rule is sometimes a crate rather
    /// than a file: `girsa-hebrew` is where the mark block lives, and its own
    /// example and its own regression corpus are entitled to name the range
    /// they exist to test.
    owners: &'static [&'static str],
}

const RULES: &[Rule] = &[
    Rule {
        // dup §1.1 — the class: **the Hebrew mark block, written out by hand.**
        // `U+0591–U+05C7` is not "the marks": four characters in it are
        // punctuation that separates words — maqaf, paseq, sof pasuq, nun
        // hafukha. Every hand-written copy of the range across the three
        // repositories got that wrong or half-wrong, and the *correct* one was
        // here all along, compiled into Ksav and referenced by nothing.
        what: "nothing writes the Hebrew mark block out by hand",
        fragments: &["0591", "05C7", "05c7"],
        owners: &["crates/girsa-hebrew/"],
    },
    Rule {
        // dup §1.1 again, one table down: the five final letters and their
        // medial forms. `girsa-hebrew` owns it; Ksav's speller had its own copy
        // and so did the Typst prelude, and the copies are the reason a peeled
        // prefix could leave a final letter stranded mid-stem in one of them
        // and not the other.
        // The **pair**, not the letter. `ך` on its own is a letter, and
        // `girsa-ref`'s gematria table and `girsa-hebrew`'s abbreviations both
        // legitimately contain it; what nobody else may write is the mapping
        // from a final form to its medial one.
        what: "nothing keeps its own final-letter fold",
        fragments: &["('ך', 'כ')", "('\\u{05DA}', '\\u{05DB}')", "\"ך\": \"כ\""],
        owners: &["crates/girsa-hebrew/src/marks.rs"],
    },
    Rule {
        // dup §1.2 — the class: **two escapers for one markup language.** This
        // crate's escaped ten characters and Ksav's editor escaped five, and
        // both write `#מראה_מקום(מקור: …)[…]` out of the same `display` string.
        // The list lives in Ksav's `engine/src/escape.rs` — the escaper is
        // needed in its browser build, which does not compile this crate — and
        // `MARKUP` here is held against it by `from_girsa.rs`.
        what: "nothing carries a second list of Typst markup characters",
        fragments: &["'#', '[', ']'", "'#' | '[' | ']'"],
        owners: &["crates/girsa-ksav/src/lib.rs"],
    },
    Rule {
        // §1 #12 — the class, stated in Ksav's `lib.rs`: **nothing else is
        // allowed to build a string literal by hand.** There were four copies of
        // it across the two repositories, one of them in this crate.
        what: "nothing builds a Typst string literal by hand",
        fragments: &["replace('\\\\', \"\\\\\\\\\")"],
        owners: &["crates/girsa-ksav/src/lib.rs"],
    },
    Rule {
        // §1 #14 — `כסב` is kaf-samekh-bet, a letter-by-letter transliteration
        // of the Latin "Ksav" back into Hebrew. It is not a word. Both
        // applications forbid it and neither guard could read this tree.
        what: "nothing spells the writing application כסב",
        fragments: &["כסב"],
        owners: &[],
    },
];

#[test]
fn every_named_class_is_swept() {
    let root = repo();
    let files = sources(&root);
    assert!(
        files.len() > 20,
        "the sweep found {} files — it is not reaching the workspace",
        files.len()
    );

    let mut broken: Vec<String> = Vec::new();
    for rule in RULES {
        for (path, body) in &files {
            if rule.owners.iter().any(|o| path.starts_with(o)) {
                continue;
            }
            for fragment in rule.fragments {
                if body.contains(fragment) {
                    broken.push(format!("{path}: {} ({fragment:?})", rule.what));
                }
            }
        }

        // An exemption is a claim with a test attached. An owner that no longer
        // contains what it owns is either a moved authority nobody updated here,
        // or a rule that has quietly stopped matching anything at all — and the
        // second is how a green sweep comes to guard nothing.
        for owner in rule.owners {
            let under: Vec<&(String, String)> =
                files.iter().filter(|(p, _)| p.starts_with(owner)).collect();
            if under.is_empty() {
                broken.push(format!("{owner}: named as an owner and not in the sweep"));
                continue;
            }
            if !under
                .iter()
                .any(|(_, body)| rule.fragments.iter().any(|f| body.contains(f)))
            {
                broken.push(format!(
                    "{owner}: exempt from {:?} and no longer contains it",
                    rule.what
                ));
            }
        }
    }

    assert!(
        broken.is_empty(),
        "a class this project has already named, recurring:\n  {}",
        broken.join("\n  ")
    );
}

/// A document may nest as deep as it likes and the process survives it.
///
/// This is a prohibition rather than a unit test because the class it names is
/// *unbounded recursion over input somebody else wrote*, and the cost of
/// breaking it is not a wrong answer. `read` is mutually recursive — `run` →
/// `command` → `content` → `sub` → `run` — and before `NESTING_LIMIT` nothing
/// compared the depth to anything. `"#ציטוט["` repeated two thousand times is
/// 26 KB of input and a stack overflow, and a stack overflow in Rust is an
/// immediate abort: the process is gone between one instruction and the next,
/// taking the reader's unsaved writing with it.
///
/// Both applications compile this crate and both reach it from a file a person
/// did not write — Girsa when a file is dropped on the window, Ksav on its own
/// primary input. So the depth is bounded here.
///
/// The numbers below bracket the reproduction: 700 survived on an 8 MB main
/// thread before the fix and 1000 did not, and every command that reaches this
/// runs on a blocking-pool thread with less stack than that.
#[test]
fn a_document_that_nests_forever_is_read_rather_than_aborting() {
    for levels in [64, 65, 700, 2_000, 40_000] {
        let markup = format!("{}דבר{}", "#ציטוט[".repeat(levels), "]".repeat(levels));
        let blocks = girsa_ksav::read(&markup);
        assert!(
            !blocks.is_empty(),
            "{levels} levels: a truncated reading is the right answer, and an \
             empty one says the words were lost rather than flattened"
        );
    }
}

/// The same shape with nothing closing it, which is what a half-typed document
/// looks like and is the cheaper attack: *n* levels for *n* × 7 bytes.
#[test]
fn a_document_that_never_closes_its_brackets_is_read_rather_than_aborting() {
    let markup = "#ציטוט[".repeat(40_000);
    let _ = girsa_ksav::read(&markup);
}
