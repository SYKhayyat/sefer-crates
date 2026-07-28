//! Hebrew normalization for Torah text.
//!
//! One normalizer, shared by the query bar and the corpus indexer. If those two
//! ever disagree about what `וּבַשַּׁבָּת` reduces to, search silently fails to find
//! text that is right there on the page — so there is exactly one of these and
//! everything routes through it.
//!
//! Filled in by W2. This is the W1 scaffold: the crate exists, is wired into
//! both applications, and carries the contract marker below.

/// Bumped whenever a transformation changes what a normalized string looks
/// like. A tantivy index built under an older marker must be rebuilt, because
/// its terms were normalized by different rules.
pub const NORMALIZER_VERSION: u32 = 0;
