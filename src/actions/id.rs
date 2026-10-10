use crate::Id;
use std::hash::{Hash, Hasher};

/// The id of the action a value names: the value of an `Id` itself, so code that holds only
/// the id of an action (from [`Actions::iter`](super::Actions::iter), for example) can pass it
/// wherever the declaring value is accepted, and otherwise `Id::new(value)`.
///
/// An `Id` hashes as exactly one `u64`; so does a bare `u64` key, which then stands for the
/// `Id` of that number. Enums, strings and tuples hash differently and never collide with it.
pub(crate) fn action_id(value: impl Hash) -> Id {
    let mut probe = Probe::default();
    value.hash(&mut probe);
    if probe.writes == 1 && !probe.other {
        Id::from_raw(probe.value)
    } else {
        Id::new(value)
    }
}

#[derive(Default)]
struct Probe {
    writes: u32,
    value: u64,
    other: bool,
}

impl Hasher for Probe {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, _: &[u8]) {
        self.other = true;
    }
    fn write_u64(&mut self, value: u64) {
        self.writes += 1;
        self.value = value;
    }
}
