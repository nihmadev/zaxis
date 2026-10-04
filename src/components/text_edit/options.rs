//! Settings of the multi-line mode.
use super::TextEdit;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AreaHeight {
    /// The theme's minimum height (`TextEditStyle::area_min_height`), content scrolls.
    Default,
    /// A fixed number of text lines plus padding.
    Rows(f32),
    /// Grow with the content between a minimum and maximum number of lines.
    Auto { min: Option<f32>, max: Option<f32> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AreaOptions {
    pub wrap: bool,
    pub height: AreaHeight,
    pub tab_indent: bool,
    pub tab_size: u16,
    pub submit_on_ctrl_enter: bool,
    pub fill_width: bool,
}

impl Default for AreaOptions {
    fn default() -> Self {
        Self {
            wrap: true,
            height: AreaHeight::Default,
            tab_indent: false,
            tab_size: 4,
            submit_on_ctrl_enter: false,
            fill_width: false,
        }
    }
}

impl TextEdit<'_> {
    /// Edit several lines: Enter inserts a line break, long lines wrap at word
    /// boundaries, and the content scrolls vertically with the standard scroll bars.
    pub fn multiline(mut self) -> Self {
        self.area.get_or_insert_with(AreaOptions::default);
        self
    }
    fn area_mut(&mut self) -> &mut AreaOptions {
        self.area.get_or_insert_with(AreaOptions::default)
    }
    /// Wrap lines at the field width (the default), or keep them whole and scroll
    /// horizontally. Implies [`Self::multiline`].
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.area_mut().wrap = wrap;
        self
    }
    /// Show exactly `rows` text lines (plus padding); longer content scrolls.
    /// Implies [`Self::multiline`].
    #[track_caller]
    pub fn rows(mut self, rows: f32) -> Self {
        if let Some(rows) = super::super::sanitize::positive("TextEdit::rows", rows) {
            self.area_mut().height = AreaHeight::Rows(rows);
        }
        self
    }
    /// Grow with the content from `min_rows` to `max_rows` lines, then scroll.
    /// Implies [`Self::multiline`].
    #[track_caller]
    pub fn auto_height(mut self, min_rows: f32, max_rows: f32) -> Self {
        let min = super::super::sanitize::positive("TextEdit::auto_height min", min_rows);
        let max = super::super::sanitize::positive("TextEdit::auto_height max", max_rows);
        let max = match (min, max) {
            (Some(min), Some(max)) => Some(max.max(min)),
            (_, max) => max,
        };
        self.area_mut().height = AreaHeight::Auto { min, max };
        self
    }
    /// Tab inserts a tab character instead of moving focus (Shift+Tab always moves
    /// focus back, so the keyboard is never trapped). Implies [`Self::multiline`].
    pub fn tab_indent(mut self, indent: bool) -> Self {
        self.area_mut().tab_indent = indent;
        self
    }
    /// Width of a tab character in space widths (at least 1; default 4).
    pub fn tab_size(mut self, spaces: u16) -> Self {
        self.area_mut().tab_size = spaces.max(1);
        self
    }
    /// Ctrl+Enter (Cmd+Enter) confirms the text and sets `Response::submitted` instead
    /// of inserting a line break. Plain Enter always inserts one.
    pub fn submit_on_ctrl_enter(mut self, submit: bool) -> Self {
        self.area_mut().submit_on_ctrl_enter = submit;
        self
    }
    /// Take the whole available width even when the layout has a preferred width.
    pub fn fill_width(mut self) -> Self {
        self.area_mut().fill_width = true;
        self
    }
}
