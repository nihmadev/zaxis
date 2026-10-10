//! Matching pressed strokes against the keymap.

use super::Keymap;
use crate::{actions::Stroke, Id};

/// What the strokes pressed so far stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lookup {
    None,
    /// The beginning of a longer chord: wait for the next stroke.
    Prefix,
    Full(Id),
}

/// Whether a binding scoped to `scope` applies while `active` is the current context.
pub(super) fn applies(scope: Option<&str>, active: &str) -> bool {
    match scope {
        None => true,
        Some(scope) => {
            active == scope
                || active
                    .strip_prefix(scope)
                    .is_some_and(|rest| rest.starts_with('/'))
        }
    }
}

impl Keymap {
    /// The action `strokes` complete, or whether they begin a chord. `strokes` are the keys
    /// as pressed (modifiers concrete). The narrowest context with an answer wins, then each
    /// wider one, then the unscoped bindings; `usable` drops actions that cannot run now.
    pub(crate) fn lookup(
        &self,
        context: &str,
        strokes: &[Stroke],
        usable: &dyn Fn(Id) -> bool,
    ) -> Lookup {
        let Some(candidates) = strokes.first().and_then(|key| self.first.get(key)) else {
            return Lookup::None;
        };
        let mut scopes: Vec<Option<&str>> = Vec::new();
        let mut path = context;
        while !path.is_empty() {
            scopes.push(Some(path));
            path = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        }
        scopes.push(None);
        for scope in scopes {
            let mut prefix = false;
            for &index in candidates {
                let entry = &self.entries[index];
                if entry.binding.context.as_deref() != scope
                    || !entry.resolved.starts_with(strokes)
                    || !usable(entry.binding.action)
                {
                    continue;
                }
                if entry.resolved.len() == strokes.len() {
                    return Lookup::Full(entry.binding.action);
                }
                prefix = true;
            }
            if prefix {
                return Lookup::Prefix;
            }
        }
        Lookup::None
    }
}
