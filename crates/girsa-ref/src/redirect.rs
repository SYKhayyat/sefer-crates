//! The redirect table, from day one.
//!
//! Refs travel between the two applications and get stored inside Ksav
//! documents. When upstream re-segments a text — and it does — every ref
//! written against the old shape has to keep resolving, or a sefer somebody
//! wrote last year stops compiling correctly.
//!
//! spec.md §4.2 is blunt about the cost: *"that is the promise that makes the
//! two-app system trustworthy, and it is a permanent maintenance burden. Accept
//! it now or don't build this."* So it exists now, before there is anything to
//! redirect, because retrofitting it means retrofitting it into documents that
//! have already been printed.

use std::collections::BTreeMap;

use crate::reference::Ref;

/// Old ref → where it went.
#[derive(Debug, Clone, Default)]
pub struct RedirectTable {
    entries: BTreeMap<String, Vec<Ref>>,
}

/// A chain longer than this is a cycle. Following it forever hangs the reader
/// instead of showing them a page.
const MAX_DEPTH: usize = 32;

impl RedirectTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `from` is now `to`.
    ///
    /// More than one target is normal: a re-segmentation that split a se'if in
    /// two means the old ref covers both, and the honest answer is both.
    pub fn insert(&mut self, from: &Ref, to: Vec<Ref>) {
        self.entries.insert(from.to_string(), to);
    }

    /// Where a ref points now, following the chain.
    ///
    /// A ref nobody has moved resolves to itself, so a caller can route every
    /// ref through this without checking first.
    #[must_use]
    pub fn follow(&self, r: &Ref) -> Vec<Ref> {
        let mut out = Vec::new();
        self.follow_into(r, 0, &mut out);
        if out.is_empty() {
            return vec![r.clone()];
        }
        out
    }

    fn follow_into(&self, r: &Ref, depth: usize, out: &mut Vec<Ref>) {
        if depth > MAX_DEPTH {
            return;
        }
        match self.entries.get(&r.to_string()) {
            Some(targets) => {
                for t in targets {
                    // A target that is the source is not a cycle — it is the
                    // ordinary shape of a split. `1:1` becoming `1:1` and `1:2`
                    // means the old ref still covers where it always did, plus
                    // somewhere new. Recursing on it would loop to the depth
                    // limit and then drop the half that did not move.
                    if t == r {
                        if !out.contains(t) {
                            out.push(t.clone());
                        }
                        continue;
                    }
                    self.follow_into(t, depth + 1, out);
                }
            }
            None => {
                if !out.contains(r) {
                    out.push(r.clone());
                }
            }
        }
    }

    /// Whether this ref has been moved.
    #[must_use]
    pub fn is_redirected(&self, r: &Ref) -> bool {
        self.entries.contains_key(&r.to_string())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn r(s: &str) -> Ref {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn a_ref_nobody_moved_resolves_to_itself() {
        let table = RedirectTable::new();
        let original = r("girsa:bavli/berakhot/2a:1");
        assert_eq!(table.follow(&original), vec![original]);
    }

    #[test]
    fn a_moved_ref_follows() {
        let mut table = RedirectTable::new();
        table.insert(
            &r("girsa:bavli/berakhot/2a:1"),
            vec![r("girsa:bavli/berakhot/2a:2")],
        );
        assert_eq!(
            table.follow(&r("girsa:bavli/berakhot/2a:1")),
            vec![r("girsa:bavli/berakhot/2a:2")]
        );
    }

    #[test]
    fn a_ref_that_became_two_places_resolves_to_both() {
        // Upstream split one se'if. The old ref covers both halves, and saying
        // so is more honest than picking the first.
        let mut table = RedirectTable::new();
        table.insert(
            &r("girsa:shulchan-arukh/orach-chayim/1:1"),
            vec![
                r("girsa:shulchan-arukh/orach-chayim/1:1"),
                r("girsa:shulchan-arukh/orach-chayim/1:2"),
            ],
        );
        assert_eq!(
            table
                .follow(&r("girsa:shulchan-arukh/orach-chayim/1:1"))
                .len(),
            2
        );
    }

    #[test]
    fn a_chain_of_two_re_segmentations_still_lands() {
        let mut table = RedirectTable::new();
        table.insert(&r("girsa:x/1:1"), vec![r("girsa:x/1:2")]);
        table.insert(&r("girsa:x/1:2"), vec![r("girsa:x/1:3")]);
        assert_eq!(table.follow(&r("girsa:x/1:1")), vec![r("girsa:x/1:3")]);
    }

    #[test]
    fn a_cycle_stops_rather_than_hanging() {
        let mut table = RedirectTable::new();
        table.insert(&r("girsa:x/1:1"), vec![r("girsa:x/1:2")]);
        table.insert(&r("girsa:x/1:2"), vec![r("girsa:x/1:1")]);
        assert!(table.follow(&r("girsa:x/1:1")).len() <= 1);
    }
}
