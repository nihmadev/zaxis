//! What a key box edits, and how it records a shortcut.

use super::{KeyBinding, KeyBox, KeyCapture};
use crate::{
    actions::{is_modifier, MAX_STROKES},
    Chord, Context, Kbd, Mods, Stroke,
};
use winit::keyboard::KeyCode;

/// A recorded sequence is two strokes long, the way editors bind them.
const SEQUENCE_STROKES: usize = 2;

pub(super) enum Target<'a> {
    Key(&'a mut KeyBinding),
    Chord {
        chord: &'a mut Chord,
        sequence: bool,
    },
}

impl Target<'_> {
    pub(super) fn label(&self, context: &Context) -> String {
        match self {
            Self::Key(binding) => binding.label(),
            Self::Chord { chord, .. } if chord.is_empty() => "None".to_owned(),
            Self::Chord { chord, .. } => Kbd::new(chord)
                .platform(context.action_platform())
                .to_string(),
        }
    }

    /// The caption while the box listens: dots, or the strokes recorded so far.
    pub(super) fn capturing_label(&self, context: &Context) -> String {
        let strokes = &context.key_capture.strokes;
        if strokes.is_empty() {
            return "...".to_owned();
        }
        let chord = Chord::new(strokes.iter().copied());
        format!("{} …", Kbd::new(&chord).platform(context.action_platform()))
    }
}

impl KeyBox<'_> {
    /// Read this pass's input while listening. `Some(changed)` ends the capture.
    pub(super) fn poll(&mut self, context: &mut Context, over_button: bool) -> Option<bool> {
        let allow_left = self.allow_left;
        match &mut self.target {
            Target::Key(binding) => {
                let next = Self::capture(**binding, allow_left, context, over_button)?;
                let changed = **binding != next;
                **binding = next;
                Some(changed)
            }
            Target::Chord { chord, sequence } => poll_chord(chord, *sequence, context, over_button),
        }
    }
}

/// Strokes pressed since the last pass, as shortcuts count them: the letter a key
/// produces and the modifiers held when it came.
fn pressed_strokes(context: &Context) -> Vec<Stroke> {
    let mut codes: Vec<KeyCode> = context
        .input()
        .keys_pressed
        .iter()
        .copied()
        .filter(|code| !is_modifier(*code))
        .collect();
    codes.sort_by_key(|code| format!("{code:?}"));
    let held = Mods::from_state(context.input().modifiers);
    codes
        .into_iter()
        .map(|code| {
            let (key, mods) = context.actions.pressed_key(code).unwrap_or((code, held));
            Stroke::new(mods, key)
        })
        .collect()
}

fn poll_chord(
    chord: &mut Chord,
    sequence: bool,
    context: &mut Context,
    over_button: bool,
) -> Option<bool> {
    let now = context.frame_time();
    let new = pressed_strokes(context);
    let plain = |code: KeyCode, new: &[Stroke]| {
        new.iter()
            .any(|s| s.mods.is_empty() && s.code() == Some(code))
    };
    if plain(KeyCode::Escape, &new) || context.input().primary_pressed && !over_button {
        return Some(false);
    }
    if context.key_capture.strokes.is_empty()
        && (plain(KeyCode::Backspace, &new) || plain(KeyCode::Delete, &new))
    {
        let changed = !chord.is_empty();
        *chord = Chord::default();
        return Some(changed);
    }
    let timeout = context.chord_timeout();
    let capture: &mut KeyCapture = &mut context.key_capture;
    let before = capture.strokes.len();
    capture.strokes.extend(new);
    capture.strokes.truncate(MAX_STROKES);
    if capture.strokes.is_empty() {
        return None;
    }
    let grew = capture.strokes.len() > before;
    let expired = !grew && capture.deadline.is_some_and(|deadline| now >= deadline);
    if grew {
        capture.deadline = now.checked_add(timeout);
    }
    let complete = !sequence || capture.strokes.len() >= SEQUENCE_STROKES;
    if !(complete || expired) {
        if grew {
            context.request_repaint_after(timeout);
        }
        return None;
    }
    let next = Chord::new(capture.strokes.iter().copied());
    let changed = *chord != next;
    *chord = next;
    Some(changed)
}
