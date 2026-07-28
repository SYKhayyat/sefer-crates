//! The Source Packet — the wire contract between Girsa and Ksav.
//!
//! Adding a field is a compile error on the side that ignores it, rather than a
//! silent production bug. That is the entire reason this is a shared crate and
//! not a documented JSON shape.
//!
//! Filled in by W4. This is the W1 scaffold.

/// Wire-format version of the Source Packet.
///
/// Three repositories means a mismatched pair is possible, so a packet carries
/// its schema version and a consumer that does not understand it errors at the
/// handshake instead of half-deserializing a citation.
pub const PACKET_SCHEMA_VERSION: u32 = 1;
