//! Maps and sets keyed by [`Id`]. An id is already a well-mixed 64-bit hash, so these use
//! it as the hash itself instead of hashing it again: the per-pass tables of key claims and
//! focus groups, which touch every control, stay cheap.

use super::Id;
use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, Hasher},
};

#[derive(Default)]
pub(crate) struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 << 8 | self.0 >> 56) ^ u64::from(*byte);
        }
    }
    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

pub(crate) type IdMap<V> = HashMap<Id, V, BuildHasherDefault<IdHasher>>;
pub(crate) type IdSet = HashSet<Id, BuildHasherDefault<IdHasher>>;
