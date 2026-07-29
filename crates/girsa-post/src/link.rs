//! `girsa://` and `ksav://` — a citation you can click, anywhere.
//!
//! spec.md §10.6: *click a citation anywhere, including inside a compiled PDF,
//! and land on the exact line.* The operating system registers the scheme and
//! hands the application the whole URL; this turns that string into an errand.
//!
//! # Two shapes of open, and the short one is the important one
//!
//! ```text
//! girsa://open?ref=girsa:bavli/berakhot/2a:1   the long form the spec names
//! girsa:bavli/berakhot/2a:1                    a ref, which is already a URI
//! ```
//!
//! The second is not a convenience. **A ref is already a `girsa:` URI**, so the
//! string a Ksav document has been storing all along is a link that works —
//! nothing has to be generated, escaped or kept in step with the ref beside it.
//! That is why the citation in Girsa's own HTML clipboard flavour is
//! `<a href="girsa:…">`: paste a quote into Word, print it to PDF, and the
//! mekor in the PDF opens the page it names.
//!
//! # What is deliberately not accepted
//!
//! Anything that is not one of these errands. A URL handler is an entry point
//! the whole machine can reach — a web page can navigate to `girsa://…` — so
//! this returns `None` rather than doing something approximate, and the two
//! errands it does accept are *show me a place* and *insert a source*, neither
//! of which touches a file.

use crate::App;

/// What a link asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Errand {
    /// Show this place. The payload is a ref, unparsed — the resolver lives in
    /// Girsa's half of the world and this crate does not need it.
    Open { reference: String },
    /// Insert this source into the open document.
    Insert { packet: String },
}

/// Read a URL the operating system handed us.
///
/// `None` for anything this does not recognise, including a scheme belonging
/// to the other application: `ksav://insert?…` arriving at Girsa is a
/// misconfigured machine, not an instruction.
#[must_use]
pub fn deep_link(app: App, url: &str) -> Option<Errand> {
    let url = url.trim();
    let rest = url.strip_prefix(app.as_str())?.strip_prefix(':')?;

    // `girsa:bavli/berakhot/2a:1` — a bare ref, which is the form a citation
    // in a document carries.
    let Some(rest) = rest.strip_prefix("//") else {
        return (app == App::Girsa && !rest.is_empty()).then(|| Errand::Open {
            reference: url.to_string(),
        });
    };

    let (errand, query) = rest.split_once('?').unwrap_or((rest, ""));
    match (app, errand.trim_end_matches('/')) {
        (App::Girsa, "open") => Some(Errand::Open {
            reference: field(query, "ref")?,
        }),
        (App::Ksav, "insert") => Some(Errand::Insert {
            packet: field(query, "packet")?,
        }),
        _ => None,
    }
}

/// One field out of a query string, percent-decoded.
fn field(query: &str, name: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| decode(value))
        .filter(|value| !value.is_empty())
}

/// Percent-decoding, and `+` for a space.
///
/// A packet is JSON and a ref carries `:` and `/`, so both arrive encoded.
/// Written here rather than taken from a crate because it is fifteen lines and
/// this is the only place either application decodes a URL.
fn decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' if at + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[at + 1..at + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        at += 3;
                    }
                    // Not an escape after all — a literal `%`, which real
                    // text contains.
                    Err(_) => {
                        out.push(b'%');
                        at += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn a_ref_is_already_a_link_and_that_is_the_whole_point() {
        // The string the document has been storing all along. Nothing is
        // generated, escaped, or kept in step with anything.
        assert_eq!(
            deep_link(App::Girsa, "girsa:bavli/berakhot/2a:1"),
            Some(Errand::Open {
                reference: "girsa:bavli/berakhot/2a:1".into()
            })
        );
        // There is no function here for *writing* one, and there should not
        // be: a ref is the link, so anything that formatted one would be a
        // second spelling to keep in step with the first.
    }

    #[test]
    fn the_long_form_the_spec_names_reads_too() {
        assert_eq!(
            deep_link(
                App::Girsa,
                "girsa://open?ref=girsa%3Ashulchan-arukh%2Forach-chayim%2F1%3A1"
            ),
            Some(Errand::Open {
                reference: "girsa:shulchan-arukh/orach-chayim/1:1".into()
            })
        );
    }

    #[test]
    fn a_packet_arrives_whole_through_a_url() {
        let packet = r#"{"schema":1,"ref":"girsa:x/1:1","display":"ד","text":"א ב"}"#;
        let encoded: String = packet
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        assert_eq!(
            deep_link(App::Ksav, &format!("ksav://insert?packet={encoded}")),
            Some(Errand::Insert {
                packet: packet.to_string()
            })
        );
    }

    #[test]
    fn hebrew_survives_the_round_trip_through_a_url() {
        // Percent-encoding is per byte and Hebrew is two of them; decoding a
        // character at a time would produce mojibake that still parses.
        assert_eq!(
            deep_link(
                App::Girsa,
                "girsa://open?ref=%D7%91%D7%A8%D7%9B%D7%95%D7%AA"
            ),
            Some(Errand::Open {
                reference: "ברכות".into()
            })
        );
    }

    #[test]
    fn the_other_applications_errand_is_refused_rather_than_approximated() {
        // A URL handler is an entry point the whole machine can reach.
        assert_eq!(deep_link(App::Girsa, "ksav://insert?packet=%7B%7D"), None);
        assert_eq!(deep_link(App::Ksav, "girsa:bavli/berakhot/2a:1"), None);
        assert_eq!(deep_link(App::Girsa, "girsa://delete?everything=yes"), None);
        assert_eq!(deep_link(App::Girsa, "https://example.com/girsa:x"), None);
        assert_eq!(deep_link(App::Girsa, "girsa://open"), None);
        assert_eq!(deep_link(App::Girsa, "girsa://open?ref="), None);
        assert_eq!(deep_link(App::Girsa, "girsa:"), None);
    }
}
