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

/// A redirect chain longer than this is a cycle somebody built by hand.
///
/// Following it forever hangs the reader instead of showing them a page.
///
/// **Public, and the only one.** There were four caps of 32 across the two
/// applications — here, `girsa_corpus::store`, `girsa_corpus::standing` and
/// `girsa_app::shelf` — walking four different structures for one reason, and
/// `standing.rs` named two of the other three in its own comment and then wrote
/// its own constant anyway. Four walks is right: they traverse a `Ref` chain, a
/// `SegmentId` table, an ancestor-and-redirect queue and a position map, and
/// forcing one walker on them would be a worse abstraction than four honest
/// ones. Four *numbers* is not right, because they are one claim: **thirty-two
/// hops is a hand-built loop**, and a repo that changed its mind about that
/// would want to change it once.
///
/// It lives here because this crate owns redirects (spec.md §4.2) and is the one
/// both applications already depend on.
pub const MAX_REDIRECT_DEPTH: usize = 32;

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
        if depth > MAX_REDIRECT_DEPTH {
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

    /// Every row, in ref order, ready to travel.
    #[must_use]
    pub fn rows(&self) -> Vec<Moved> {
        self.entries
            .iter()
            .map(|(from, to)| Moved {
                from: from.clone(),
                to: to.iter().map(ToString::to_string).collect(),
            })
            .collect()
    }

    /// Read a table back off the wire.
    ///
    /// A row whose `from` will not parse is dropped rather than kept as a key
    /// nothing can ask about, and a target that will not parse is dropped from
    /// its row — a redirect to a name this build cannot read is not a place to
    /// send a reader. A row left with no targets at all is `Gone` said badly
    /// and is dropped whole.
    #[must_use]
    pub fn of_rows(rows: &[Moved]) -> Self {
        let mut table = Self::new();
        for row in rows {
            let Ok(from) = row.from.parse::<Ref>() else {
                continue;
            };
            let to: Vec<Ref> = row.to.iter().filter_map(|r| r.parse().ok()).collect();
            if to.is_empty() {
                continue;
            }
            table.insert(&from, to);
        }
        table
    }
}

/// One row of the table, as it travels between the two applications.
///
/// Both ends are the **printed ref**, because that is how a ref moves
/// everywhere else in this system — into a Source Packet's `ref`, into a Ksav
/// document, into the corpus's own `redirects.jsonl`, whose rows are this shape
/// one level down (`SegmentId` rather than `Ref`). One spelling, not two.
///
/// Strings and not [`Ref`]s because `Ref` is `Display` + `FromStr` and carries
/// no derive: a serde representation invented for it here would be a second way
/// to write a ref down, in the crate whose whole job is that there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Moved {
    pub from: String,
    pub to: Vec<String>,
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

#[cfg(all(test, feature = "serde"))]
mod travelling {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn r(s: &str) -> Ref {
        s.parse().unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn a_table_survives_the_wire() {
        // The whole reason this type is here rather than in either application:
        // Girsa knows a mareh makom moved and Ksav is the one holding the
        // document that says the old name. The fact has to cross, and it
        // crosses as this.
        let mut here = RedirectTable::new();
        here.insert(
            &r("girsa:shulchan-arukh/orach-chayim/1:3"),
            vec![
                r("girsa:shulchan-arukh/orach-chayim/1:3"),
                r("girsa:shulchan-arukh/orach-chayim/1:4"),
            ],
        );
        here.insert(
            &r("girsa:bavli/berakhot/2a:1"),
            vec![r("girsa:bavli/berakhot/2a:2")],
        );

        let json = serde_json::to_string(&here.rows()).expect("rows serialise");
        let rows: Vec<Moved> = serde_json::from_str(&json).expect("rows read back");
        let there = RedirectTable::of_rows(&rows);

        assert_eq!(there.len(), here.len());
        for original in [
            "girsa:shulchan-arukh/orach-chayim/1:3",
            "girsa:bavli/berakhot/2a:1",
            "girsa:bavli/berakhot/9b:1",
        ] {
            assert_eq!(
                there.follow(&r(original)),
                here.follow(&r(original)),
                "{original} lands somewhere else after the trip"
            );
        }
    }

    #[test]
    fn a_row_naming_something_this_build_cannot_read_is_dropped() {
        // Both halves, and they are different failures. A `from` that will not
        // parse is a key nothing can ever ask about — dead weight. A `to` that
        // will not parse is worse: it is a place to send a reader that does not
        // exist, and a row left with none of them is `Gone` said badly.
        let rows = vec![
            Moved {
                from: "not a ref".into(),
                to: vec!["girsa:bavli/berakhot/2a:1".into()],
            },
            Moved {
                from: "girsa:bavli/berakhot/2a:1".into(),
                to: vec!["also not a ref".into()],
            },
            Moved {
                from: "girsa:bavli/berakhot/3a:1".into(),
                to: vec!["nonsense".into(), "girsa:bavli/berakhot/3a:2".into()],
            },
        ];
        let table = RedirectTable::of_rows(&rows);

        assert_eq!(table.len(), 1, "one row of the three survives");
        assert!(!table.is_redirected(&r("girsa:bavli/berakhot/2a:1")));
        assert_eq!(
            table.follow(&r("girsa:bavli/berakhot/3a:1")),
            vec![r("girsa:bavli/berakhot/3a:2")]
        );
    }
}
