//! Actions and keymap through the public API: registration, events, chords, contexts,
//! conflicts, saved bindings, layouts and the widgets bound to actions.

#[cfg(feature = "accesskit")]
mod access;
mod chord;
mod context;
mod keymap;
mod layout;
mod rebind;
mod registry;
mod support;
mod trigger;
mod widgets;
