//! Saving and loading the user's key bindings as plain text, without serde.
//!
//! ```text
//! # zaxis keymap 1
//! file.save = mod+s, f2
//! edit.find@editor = mod+f
//! view.sidebar = mod+k mod+b
//! file.close =
//! ```
//!
//! One line holds the chords of one action in one context, separated by commas; chords are
//! written as [`Chord`] text. The first line of an action replaces all its default bindings,
//! later lines for the same action add the bindings of other contexts, and an empty right
//! side leaves the action without keys. Only the actions the user changed are written, so
//! default bindings can change between versions.

use super::{Binding, Keymap};
use crate::{actions::Chord, Id};
use std::collections::HashMap;

const HEADER: &str = "# zaxis keymap 1";

/// A line of a saved keymap that could not be applied. Everything else was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapIssue {
    /// The line, counted from 1.
    pub line: usize,
    pub message: String,
}

impl Keymap {
    /// The user's overrides as text; see the module documentation for the format.
    pub fn save(&self) -> String {
        let mut text = format!("{HEADER}\n");
        for (id, name) in &self.names {
            let Some(bindings) = self.overrides.get(id) else {
                continue;
            };
            if bindings.is_empty() {
                text.push_str(&format!("{name} =\n"));
                continue;
            }
            let mut scopes: Vec<Option<&str>> = Vec::new();
            for binding in bindings {
                if !scopes.contains(&binding.context.as_deref()) {
                    scopes.push(binding.context.as_deref());
                }
            }
            for scope in scopes {
                let chords: Vec<String> = bindings
                    .iter()
                    .filter(|binding| binding.context.as_deref() == scope)
                    .map(|binding| binding.chord.to_string())
                    .collect();
                match scope {
                    Some(scope) => {
                        text.push_str(&format!("{name}@{scope} = {}\n", chords.join(", ")))
                    }
                    None => text.push_str(&format!("{name} = {}\n", chords.join(", "))),
                }
            }
        }
        text
    }

    /// Replace the user's overrides with the ones in `text`. Lines that name an unknown
    /// action or hold text that is not a chord are skipped and reported.
    pub fn load(&mut self, text: &str) -> Vec<KeymapIssue> {
        let by_name: HashMap<&str, Id> = self
            .names
            .iter()
            .map(|(id, name)| (name.as_str(), *id))
            .collect();
        let mut issues = Vec::new();
        let mut loaded: Vec<(Id, Vec<Binding>)> = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut issue = |message: String| {
                issues.push(KeymapIssue {
                    line: index + 1,
                    message,
                })
            };
            let Some((left, right)) = line.split_once('=') else {
                issue("expected `action = chord`".into());
                continue;
            };
            let (name, context) = match left.trim().split_once('@') {
                Some((name, context)) => (name.trim(), Some(context.trim().to_owned())),
                None => (left.trim(), None),
            };
            let Some(&id) = by_name.get(name) else {
                issue(format!("unknown action `{name}`"));
                continue;
            };
            let slot = match loaded.iter().position(|(known, _)| *known == id) {
                Some(slot) => slot,
                None => {
                    loaded.push((id, Vec::new()));
                    loaded.len() - 1
                }
            };
            for text in right
                .split(',')
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                match text.parse::<Chord>() {
                    Ok(chord) => loaded[slot].1.push(Binding {
                        action: id,
                        context: context.clone(),
                        chord,
                    }),
                    Err(error) => issue(format!("`{text}`: {error}")),
                }
            }
        }
        self.overrides = loaded.into_iter().collect();
        self.rebuild();
        issues
    }
}
