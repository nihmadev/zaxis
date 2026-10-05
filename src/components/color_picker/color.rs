//! Color conversions, the text of the channel fields and the picker's retained state:
//! the hue it keeps through greys and black, open state, and the drafts of its fields.

use super::ColorPickerState;
use crate::Color;

/// Channel fields: red, green, blue, then hex.
pub(super) const FIELDS: usize = 4;
pub(super) const HEX: usize = 3;
/// Longest draft a field takes, in characters.
pub(super) const FIELD_LIMIT: usize = 16;

/// Characters a channel or hex field takes: printable ASCII.
pub(super) fn field_char(c: char) -> bool {
    c.is_ascii() && !c.is_ascii_control()
}

/// The text a field shows for `color`.
pub(super) fn field_text(color: Color, field: usize) -> String {
    if field == HEX {
        format!("#{:02X}{:02X}{:02X}", color.0[0], color.0[1], color.0[2])
    } else {
        color.0[field].to_string()
    }
}

/// Apply the draft of `field` to `color`. Hex takes six hex digits after an optional
/// `#`; a channel takes an integer, clamped to 0..=255. Alpha is kept. Returns whether the
/// draft was a value; one that is not leaves `color` alone.
fn apply_field(color: &mut Color, field: usize, text: &str) -> bool {
    if field == HEX {
        let text = text.trim().trim_start_matches('#');
        if text.len() != 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return false;
        }
        let rgb = u32::from_str_radix(text, 16).unwrap();
        *color = Color::rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, color.0[3]);
        true
    } else if let Ok(value) = text.trim().parse::<i64>() {
        color.0[field] = value.clamp(0, 255) as u8;
        true
    } else {
        false
    }
}

pub fn to_hsv(color: Color) -> [f32; 3] {
    let [r, g, b] = [color.0[0], color.0[1], color.0[2]].map(|c| f32::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    [hue, if max == 0.0 { 0.0 } else { delta / max }, max]
}

pub fn from_hsv([h, s, v]: [f32; 3], alpha: u8) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let rgb = match h as u32 {
        0 => [c, x, 0.0],
        1 => [x, c, 0.0],
        2 => [0.0, c, x],
        3 => [0.0, x, c],
        4 => [x, 0.0, c],
        _ => [c, 0.0, x],
    }
    .map(|channel| ((channel + v - c) * 255.0).round() as u8);
    Color::rgba(rgb[0], rgb[1], rgb[2], alpha)
}

/// The text being edited in one field, and when its edit began: drafts are committed in
/// the order they began, which is the order the user edited them.
pub(super) struct Draft {
    field: usize,
    began: u64,
    text: String,
}

impl ColorPickerState {
    pub(super) fn new(color: Color, open: bool) -> Self {
        Self {
            open,
            hsv: to_hsv(color),
            last_color: color,
            drafts: Default::default(),
            committing: Vec::new(),
            serial: 0,
        }
    }

    /// The bound color as this pass found it. A change made outside drops the drafts and
    /// moves the editor to it.
    pub(super) fn observe(&mut self, color: Color) {
        if color != self.last_color {
            self.discard_drafts();
        }
        self.sync(color);
    }

    /// Follow `color`, keeping the selected hue for greys and saturation for black.
    pub(super) fn sync(&mut self, color: Color) {
        if self.last_color != color {
            let hsv = to_hsv(color);
            if hsv[1] > 0.0 {
                self.hsv[0] = hsv[0];
            }
            if hsv[2] > 0.0 {
                self.hsv[1] = hsv[1];
            }
            self.hsv[2] = hsv[2];
            self.last_color = color;
        }
    }

    /// The editor surfaces moved the color.
    pub(super) fn set_hsv(&mut self, color: &mut Color) {
        *color = from_hsv(self.hsv, color.0[3]);
    }

    /// The text `field` shows: its draft, or the color.
    pub(super) fn text(&self, field: usize, color: Color) -> String {
        self.drafts[field]
            .as_ref()
            .map_or_else(|| field_text(color, field), |draft| draft.text.clone())
    }

    /// `field` was edited: its draft begins with `text` unless it has one.
    pub(super) fn begin(&mut self, field: usize, text: &str) {
        if self.drafts[field].is_none() {
            self.serial += 1;
            self.drafts[field] = Some(Draft {
                field,
                began: self.serial,
                text: text.to_owned(),
            });
        }
    }

    /// The field's text after it handled this pass's input.
    pub(super) fn update(&mut self, field: usize, text: String) {
        if let Some(draft) = &mut self.drafts[field] {
            draft.text = text;
        }
    }

    /// `text` of `field` is to be committed at the end of the fields' stage; editing the
    /// field afterwards begins a new draft.
    pub(super) fn submit(&mut self, field: usize, text: String) {
        self.begin(field, &text);
        if let Some(mut draft) = self.drafts[field].take() {
            draft.text = text;
            self.committing.push(draft);
        }
    }

    /// Escape: the draft of `field` is dropped.
    pub(super) fn cancel(&mut self, field: usize) {
        self.drafts[field] = None;
    }

    /// Every draft is to be committed, as when the editor closes.
    pub(super) fn submit_all(&mut self) {
        for field in 0..FIELDS {
            if let Some(draft) = self.drafts[field].take() {
                self.committing.push(draft);
            }
        }
    }

    /// Commit, in the order their edits began, the drafts submitted in this pass and those
    /// whose field `focused` says lost focus. Returns whether anything was committed.
    pub(super) fn settle_drafts(
        &mut self,
        color: &mut Color,
        focused: impl Fn(usize) -> bool,
    ) -> bool {
        for field in 0..FIELDS {
            if self.drafts[field].is_some() && !focused(field) {
                self.committing.extend(self.drafts[field].take());
            }
        }
        let mut drafts = std::mem::take(&mut self.committing);
        drafts.sort_by_key(|draft| draft.began);
        for draft in &drafts {
            apply_field(color, draft.field, &draft.text);
            self.sync(*color);
        }
        !drafts.is_empty()
    }

    /// Commit every draft now, in the order the edits began.
    pub(super) fn commit(&mut self, color: &mut Color) {
        self.submit_all();
        self.settle_drafts(color, |_| true);
    }

    /// Drafts are dropped without changing the color.
    pub(super) fn discard_drafts(&mut self) {
        self.drafts = Default::default();
        self.committing.clear();
    }
}
