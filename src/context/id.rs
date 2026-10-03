//! Stable widget identities and scoped hashing.

use std::hash::{Hash, Hasher};

/// Stable widget identity. Scope child IDs to their parent using [`Id::with`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(u64);

impl Id {
    pub fn new(value: impl Hash) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn with(self, value: impl Hash) -> Self {
        Self::new((self, value))
    }
    pub fn value(self) -> u64 {
        self.0
    }
}
