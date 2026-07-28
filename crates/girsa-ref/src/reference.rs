//! The canonical ref: a stable, resolvable pointer to a **span**.
//!
//! ```text
//! girsa:shulchan-arukh/orach-chayim/1:1        a point
//! girsa:bavli/berakhot/2a:1-2b:4               a span
//! girsa:mishnah-berurah/1:1#7                  a permanent segment anchor
//!       └─ work path ─┘ └addr┘ └ord
//! ```
//!
//! Two separators, and the difference between them is load-bearing. `/`
//! separates the parts of the **work**; `:` separates the levels of the
//! **address**. With one separator, `girsa:bavli/berakhot/2a/1` cannot be read
//! without already knowing the lexicon — is the work `bavli/berakhot` or
//! `bavli/berakhot/2a`? Refs are stored as text inside Ksav documents and have
//! to be readable on their own, so the shape carries the answer: **the last
//! `/`-separated component is the address, always**.
//!
//! A quote is a range, so a ref is a range (spec.md §4.2). A point is the case
//! where the range happens to be one segment long, not a different type.

use std::fmt;
use std::str::FromStr;

use crate::address::Address;

/// Why a ref would not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RefError {
    #[error("a ref must start with `girsa:`")]
    NotAGirsaRef,
    #[error("a ref must name a work")]
    NoWork,
    #[error("`{0}` is not an address")]
    BadAddress(String),
}

/// A stable pointer to a span of one work.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ref {
    work: Vec<String>,
    from: Address,
    to: Option<Address>,
}

impl Ref {
    /// A ref to one place.
    #[must_use]
    pub fn point(work: Vec<String>, from: Address) -> Self {
        Self {
            work,
            from,
            to: None,
        }
    }

    /// A ref to a range. If the two ends are the same it is a point, because
    /// `2a:1-2a:1` and `2a:1` should not be two different strings for one
    /// place — they would compare unequal and split a lookup in two.
    #[must_use]
    pub fn span(work: Vec<String>, from: Address, to: Address) -> Self {
        if from == to {
            return Self::point(work, from);
        }
        Self {
            work,
            from,
            to: Some(to),
        }
    }

    /// A ref to a whole work, with no address.
    #[must_use]
    pub fn whole_work(work: Vec<String>) -> Self {
        Self {
            work,
            from: Address::default(),
            to: None,
        }
    }

    /// The work path — `["shulchan-arukh", "orach-chayim"]`.
    #[must_use]
    pub fn work(&self) -> &[String] {
        &self.work
    }

    /// The work as it appears in a ref — `shulchan-arukh/orach-chayim`.
    #[must_use]
    pub fn work_slug(&self) -> String {
        self.work.join("/")
    }

    /// Where the span starts.
    #[must_use]
    pub fn from(&self) -> &Address {
        &self.from
    }

    /// Where the span ends, if it is longer than a point.
    #[must_use]
    pub fn to(&self) -> Option<&Address> {
        self.to.as_ref()
    }

    /// Whether this ref covers more than one place.
    #[must_use]
    pub fn is_span(&self) -> bool {
        self.to.is_some()
    }
}

impl fmt::Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "girsa:{}", self.work.join("/"))?;
        if self.from.is_empty() {
            return Ok(());
        }
        write!(f, "/{}", self.from)?;
        if let Some(to) = &self.to {
            write!(f, "-{to}")?;
        }
        Ok(())
    }
}

impl FromStr for Ref {
    type Err = RefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let body = s.strip_prefix("girsa:").ok_or(RefError::NotAGirsaRef)?;
        // A `#ordinal` belongs to a segment id, not to a ref. Dropping it here
        // means a segment id read as a ref points at the right place, which is
        // what a citation formatter wants.
        let body = body.split('#').next().unwrap_or(body);

        match body.rsplit_once('/') {
            // The last component is the address — unless it does not read as
            // one, in which case the whole thing is a work path and this is a
            // ref to an entire sefer.
            Some((work_path, tail)) => {
                let work: Vec<String> = work_path.split('/').map(str::to_string).collect();
                if work.iter().any(String::is_empty) {
                    return Err(RefError::NoWork);
                }
                match parse_range(tail) {
                    Some((from, to)) => Ok(Self { work, from, to }),
                    None => Ok(Self::whole_work(
                        body.split('/').map(str::to_string).collect(),
                    )),
                }
            }
            None if body.is_empty() => Err(RefError::NoWork),
            None => Ok(Self::whole_work(vec![body.to_string()])),
        }
    }
}

/// `2a:1-2b:4` → two addresses; `1:1` → one.
///
/// A hyphen inside a *level* would be ambiguous, but no address level in the
/// corpus contains one — dafim, simanim and se'ifim are numbers or a number
/// and a letter.
fn parse_range(tail: &str) -> Option<(Address, Option<Address>)> {
    match tail.split_once('-') {
        Some((from, to)) => {
            let from = Address::parse(from)?;
            let to = Address::parse(to)?;
            if from == to {
                Some((from, None))
            } else {
                Some((from, Some(to)))
            }
        }
        None => Some((Address::parse(tail)?, None)),
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_examples_in_the_spec_round_trip() {
        for s in [
            "girsa:shulchan-arukh/orach-chayim/1:1",
            "girsa:bavli/berakhot/2a:1-2b:4",
            "girsa:mishnah-berurah/121:3",
        ] {
            let parsed: Ref = match s.parse() {
                Ok(r) => r,
                Err(e) => panic!("{s} did not parse: {e}"),
            };
            assert_eq!(parsed.to_string(), s);
        }
    }

    #[test]
    fn the_last_component_is_the_address_and_the_rest_is_the_work() {
        let r: Ref = "girsa:bavli/berakhot/2a:1-2b:4".parse().expect("parses");
        assert_eq!(r.work_slug(), "bavli/berakhot");
        assert_eq!(r.from().to_string(), "2a:1");
        assert_eq!(r.to().map(ToString::to_string), Some("2b:4".into()));
        assert!(r.is_span());
    }

    #[test]
    fn a_ref_to_a_whole_sefer_has_no_address() {
        let r: Ref = "girsa:berakhot".parse().expect("parses");
        assert_eq!(r.work_slug(), "berakhot");
        assert_eq!(r.to_string(), "girsa:berakhot");
        assert!(!r.is_span());
    }

    #[test]
    fn a_span_of_length_one_is_a_point() {
        // Otherwise `2a:1-2a:1` and `2a:1` are two strings for one place, and
        // anything keyed on the text of a ref splits into two buckets.
        let a: Ref = "girsa:bavli/berakhot/2a:1-2a:1".parse().expect("parses");
        let b: Ref = "girsa:bavli/berakhot/2a:1".parse().expect("parses");
        assert_eq!(a, b);
        assert_eq!(a.to_string(), b.to_string());
    }

    #[test]
    fn a_segment_id_read_as_a_ref_points_at_the_same_place() {
        // A citation formatter is handed segment ids and has to print refs.
        let r: Ref = "girsa:mishnah-berurah/1:1#7".parse().expect("parses");
        assert_eq!(r.to_string(), "girsa:mishnah-berurah/1:1");
    }

    #[test]
    fn something_that_is_not_a_girsa_ref_is_refused() {
        assert_eq!(
            "Berakhot 2a".parse::<Ref>().unwrap_err(),
            RefError::NotAGirsaRef
        );
        assert_eq!("girsa:".parse::<Ref>().unwrap_err(), RefError::NoWork);
    }
}
