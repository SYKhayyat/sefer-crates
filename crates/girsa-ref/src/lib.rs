//! Canonical refs, citation parsing, offline resolution, and the redirect table.
//!
//! A ref points at a *span*, because a quote is a range. Refs travel between
//! Girsa and Ksav and get stored inside Ksav documents, so they have to survive
//! corpus updates — that is the promise the two-app system rests on.
//!
//! Filled in by W3. This is the W1 scaffold.

/// The scheme every canonical ref is written under: `girsa:...`.
pub const SCHEME: &str = "girsa";
