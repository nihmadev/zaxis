//! A color row that expands into an HSV editor: palette, hue strip and RGB/HEX fields.
//!
//! `show` runs a pass in stages: the row and its toggle (`row`), the editor inline or
//! floating (`panel`), the editor's surfaces and fields (`editor`, `fields`, the fields
//! being real TextEdits), what assistive technology reads and requests (`access`), and the
//! color model with the field drafts (`color`).

use std::{hash::Hash, panic::Location};

use crate::{
    context::{HitAction, HitRegion, Paint, SliderInput},
    Border, Color, CornerRadius, Id, Rect, Shape,
};

use super::{visible_label, HoverStyle, Response, Ui};

mod access;
mod color;
mod editor;
mod fields;
mod panel;
mod row;
mod show;

pub use color::{from_hsv, to_hsv};

/// Where the expanded color editor is displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorPickerType {
    /// Expand below the color row and allocate space in its layout.
    #[default]
    Internal,
    /// Open an independent draggable window without expanding the parent layout.
    Floating,
}

/// A Rayfield-style color row with an HSV palette, hue strip, and RGB/HEX fields.
/// Changes preserve the bound color's alpha channel.
pub struct ColorPicker<'a> {
    color: &'a mut Color,
    text: String,
    id: Option<Id>,
    width: Option<f32>,
    enabled: bool,
    picker_type: ColorPickerType,
    default_open: bool,
    source: &'static Location<'static>,
    hover_style: Option<HoverStyle>,
    style: crate::ColorPickerStyle,
}

impl<'a> ColorPicker<'a> {
    #[track_caller]
    pub fn new(color: &'a mut Color, text: impl Into<String>) -> Self {
        Self {
            color,
            text: text.into(),
            id: None,
            width: None,
            enabled: true,
            picker_type: ColorPickerType::Internal,
            default_open: false,
            source: Location::caller(),
            hover_style: None,
            style: Default::default(),
        }
    }

    pub fn style(mut self, style: crate::ColorPickerStyle) -> Self {
        self.style = style;
        self
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = super::sanitize::positive("ColorPicker::width", width).or(self.width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[deprecated(
        note = "use `.enabled(!disabled)`; `enabled` is the one way to set a control's availability"
    )]
    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }

    pub fn picker_type(mut self, picker_type: ColorPickerType) -> Self {
        self.picker_type = picker_type;
        self
    }

    /// Initial expansion state. Subsequent interaction is retained by widget ID.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    pub fn hover_style(mut self, style: HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
    }
}

/// The paint of a picker's color swatch. Its state is retained while the swatch is painted.
pub(crate) fn swatch_id(id: Id) -> Id {
    id.with("swatch")
}

impl ColorPicker<'_> {
    fn editor_height(&self, style: &crate::Style) -> f32 {
        let c = style.color_picker;
        c.palette_height.unwrap_or(112.0)
            + c.hue_height.unwrap_or(14.0)
            + c.gap.unwrap_or(10.0) * 3.0
            + 4.0
            + style.text_edit_height
            + 6.0
    }
}
pub struct ColorPickerState {
    open: bool,
    /// The editor's hue, saturation and brightness: a hue survives greys and black.
    hsv: [f32; 3],
    last_color: Color,
    /// The text being edited in each channel field, until it is committed or dropped.
    drafts: [Option<color::Draft>; color::FIELDS],
    /// Drafts taken by Enter or assistive technology in this pass, committed with the
    /// others at the end of the fields' stage.
    committing: Vec<color::Draft>,
    serial: u64,
}

fn rounded(rect: Rect, fill: Color, radius: impl Into<CornerRadius>, border: Border) -> Paint {
    Paint::Shape(
        Shape::rect(rect, fill)
            .corner_radius(radius)
            .border(border)
            .into(),
    )
}

fn hit(ui: &mut Ui<'_>, id: Id, rect: Rect, enabled: bool, action: HitAction) {
    ui.context.register_hit(HitRegion {
        id,
        window: ui.window,
        rect,
        clip: ui.clip,
        action: if enabled { action } else { HitAction::Block },
    });
}

impl Ui<'_> {
    #[track_caller]
    pub fn color_picker(&mut self, color: &mut Color, text: impl Into<String>) -> Response {
        self.add(ColorPicker::new(color, text))
    }
}
