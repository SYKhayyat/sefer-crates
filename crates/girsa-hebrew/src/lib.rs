//! Hebrew normalization for Torah text.
//!
//! One normalizer, shared by the query bar and the corpus indexer. If those two
//! ever disagree about what `וּבַשַּׁבָּת` reduces to, search silently fails to find
//! text that is right there on the page — so there is exactly one of these and
//! everything routes through it.
//!
//! # Two operations, and the difference between them is the product
//!
//! [`normalize`] produces **the normal form**: what goes in the index and what a
//! query is compared against. It removes marks nobody types and folds
//! characters that are the same character written twice — and **nothing else**.
//! It never destroys a word.
//!
//! [`variants`] produces **other surface forms the same word might wear**:
//! prefixes peeled, ktiv male/chaser swapped, abbreviations expanded. Each one
//! is labelled with the [`VariantKind`] that produced it.
//!
//! The split is not tidiness. spec.md §9.3 makes the literal mode the default
//! and requires that *what you typed is what was searched for*; §9.6 requires
//! that widening be **offered with a count computed up front** and applied only
//! when clicked. Neither is possible if the widening is baked into the index.
//! So the index is normalized and the widening is a set the caller may choose
//! to use — the engine never changes a query without the reader knowing.
//!
//! ```
//! use girsa_hebrew::{normalize, variants, VariantKind};
//!
//! // Marks are stripped; the word survives.
//! assert_eq!(normalize("וּבַשַּׁבָּת"), "ובשבת");
//!
//! // Peeling is offered, never applied.
//! let v = variants("ובשבת");
//! assert!(v.forms_of_kind(VariantKind::PrefixPeeled).any(|f| f == "שבת"));
//! ```

#![doc(html_root_url = "https://docs.rs/girsa-hebrew/0.5.0")]

mod abbrev;
mod ktiv;
mod marks;
mod normalize;
mod prefix;
mod variants;

pub use abbrev::{expansions_of, AbbreviationTable, ABBREVIATIONS};
pub use marks::{is_hebrew_letter, is_mark, is_word_breaking_punctuation};
pub use normalize::{normalize, normalize_into, tokenize, Token};
pub use prefix::{peelings, PREFIX_LETTERS};
pub use variants::{variants, variants_with, VariantKind, VariantSet};

/// Bumped whenever a transformation changes what a normalized string looks
/// like.
///
/// A tantivy index built under an older marker must be rebuilt, because its
/// terms were normalized by different rules: a query normalized by the new
/// rules against terms written under the old ones silently fails to find text
/// that is right there. `girsa_corpus::CacheProvenance` records this so the
/// mismatch is detectable instead of invisible.
///
/// This tracks [`normalize`] only. [`variants`] is applied at query time, so
/// changing it changes results without invalidating anything on disk.
pub const NORMALIZER_VERSION: u32 = 1;
