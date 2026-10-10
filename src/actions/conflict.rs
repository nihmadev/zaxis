use super::Chord;
use crate::Id;

/// Why two bindings cannot both work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both actions have the same chord in the same context. The first one registered wins.
    Same,
    /// The first action's chord is the beginning of the second's: the second can never
    /// complete, because the first runs as soon as its keys are pressed.
    Prefix,
}

/// Two actions bound to keys that collide in one context. Bindings in different contexts
/// never conflict: the narrower context wins by design.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub context: Option<String>,
    /// The chord of `first`.
    pub chord: Chord,
    pub first: Id,
    pub second: Id,
    pub kind: ConflictKind,
}
