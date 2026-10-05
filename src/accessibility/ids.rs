//! Node identifiers of the accessibility tree.
//!
//! A widget's node id is its [`Id`] value, so it is as stable as the widget's own identity.
//! The first few values are reserved for nodes that belong to no widget; an `Id` that
//! happens to hash into that range is moved out of it.

use crate::Id;

/// The native window.
pub(crate) const ROOT: u64 = 1;
/// Live regions that carry [`Context::announce`](crate::Context::announce) messages.
pub(crate) const ANNOUNCE_POLITE: u64 = 2;
pub(crate) const ANNOUNCE_ASSERTIVE: u64 = 3;
const RESERVED: u64 = 16;

fn outside_reserved(value: u64) -> u64 {
    if value < RESERVED {
        value | 1 << 63
    } else {
        value
    }
}

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

pub(crate) fn node(id: Id) -> u64 {
    outside_reserved(id.value())
}

/// The id of text run `run` of the node `owner`: stable for a given line number.
pub(crate) fn run(owner: u64, run: usize) -> u64 {
    outside_reserved(mix(owner ^ mix(run as u64 ^ 0x7a61_7869_735f_7275)))
}

/// A replacement for an id already used in this pass; `attempt` starts at one.
pub(crate) fn alternate(id: u64, attempt: u64) -> u64 {
    outside_reserved(mix(id ^ mix(attempt)))
}
