//! Where inside a work a ref points, and the notations that mean the same place.

use std::fmt;

use crate::numerals;

/// One level of an address: a siman, a se'if, a perek, a daf.
///
/// Held as the canonical string rather than a number, because not every level
/// is a number — `2a` is a daf and an amud, and `Introduction` is a section
/// with a name. Levels that *are* numbers answer [`Level::as_number`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(String);

impl Level {
    /// Read a level however it was written — `5`, `ה'`, `ב.`, `ב ע"ב`, `2b`.
    ///
    /// Returns `None` for something that is not a level at all, so a caller
    /// gets to decide rather than being handed a guess.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        if let Some(daf) = crate::daf::parse(raw) {
            return Some(Self(daf));
        }
        if let Some(n) = numerals::parse(raw) {
            return Some(Self(n.to_string()));
        }
        // A named section — `Introduction`, `הקדמה`. Kept as written.
        Some(Self(raw.to_string()))
    }

    /// Build a level from a number.
    #[must_use]
    pub fn number(n: u32) -> Self {
        Self(n.to_string())
    }

    /// Build a level from an already-canonical string.
    #[must_use]
    pub fn canonical(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// The number, if this level is one.
    #[must_use]
    pub fn as_number(&self) -> Option<u32> {
        self.0.parse().ok()
    }

    /// Whether this level is addressed by number — a siman, a se'if, a perek, a
    /// daf — rather than named.
    ///
    /// This is what tells a hyphen apart from a hyphen. `2a:1-2b:4` is a range
    /// because both ends are numbered; `כסלו-טבת` is one named level because
    /// neither end is. See [`crate::reference`].
    #[must_use]
    pub fn is_numbered(&self) -> bool {
        self.as_number().is_some() || crate::daf::parse(&self.0).is_some()
    }

    /// The canonical text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A point inside a work — `1:1`, `2a:4`, `4:1`.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(Vec<Level>);

impl Address {
    #[must_use]
    pub fn new(levels: Vec<Level>) -> Self {
        Self(levels)
    }

    /// Read `1:1`, or Sefaria's own `2a:4`.
    ///
    /// Returns `None` if any level is unreadable, rather than dropping it — a
    /// citation half understood is a citation pointing somewhere else.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let levels: Option<Vec<Level>> = raw
            .split(':')
            .filter(|part| !part.trim().is_empty())
            .map(Level::parse)
            .collect();
        let levels = levels?;
        (!levels.is_empty()).then_some(Self(levels))
    }

    #[must_use]
    pub fn levels(&self) -> &[Level] {
        &self.0
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Extend a partial address against the one currently being read.
    ///
    /// "see se'if 5" while standing in Orach Chayim siman 1 means `1:5`. This
    /// is spec.md §4.3's partial-ref case, and it is the one place a resolver
    /// is *allowed* to use context — because the reader supplied it by being
    /// there.
    #[must_use]
    pub fn completed_against(&self, context: &Address) -> Self {
        if self.depth() >= context.depth() {
            return self.clone();
        }
        let mut levels = context.0[..context.depth() - self.depth()].to_vec();
        levels.extend(self.0.iter().cloned());
        Self(levels)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, level) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(":")?;
            }
            write!(f, "{level}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // A panic in a test is a failure report. The workspace bans these in
    // library code, where a panic would take the reader's window with it.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_siman_and_seif_read_however_they_were_written() {
        assert_eq!(
            Address::parse("1:1").map(|a| a.to_string()),
            Some("1:1".into())
        );
        assert_eq!(
            Address::parse("קכ\"א:ג'").map(|a| a.to_string()),
            Some("121:3".into())
        );
    }

    #[test]
    fn a_daf_keeps_its_amud() {
        assert_eq!(
            Address::parse("2a:4").map(|a| a.to_string()),
            Some("2a:4".into())
        );
        assert_eq!(
            Address::parse("ב.").map(|a| a.to_string()),
            Some("2a".into())
        );
        assert_eq!(
            Address::parse("ב ע\"ב").map(|a| a.to_string()),
            Some("2b".into())
        );
    }

    #[test]
    fn a_partial_address_completes_against_where_you_are_standing() {
        let here = Address::parse("121:3").expect("context parses");
        let partial = Address::parse("5").expect("partial parses");
        assert_eq!(partial.completed_against(&here).to_string(), "121:5");
    }

    #[test]
    fn a_full_address_is_not_altered_by_context() {
        let here = Address::parse("121:3").expect("context parses");
        let full = Address::parse("4:9").expect("full parses");
        assert_eq!(full.completed_against(&here).to_string(), "4:9");
    }

    #[test]
    fn an_unreadable_level_fails_the_whole_address() {
        // Reading `1:` as `1` would silently promote a se'if-level citation to
        // a siman-level one and land the reader on the wrong paragraph.
        assert_eq!(Address::parse(""), None);
        assert_eq!(Address::parse(":"), None);
    }
}
