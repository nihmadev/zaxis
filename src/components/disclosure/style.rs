use crate::{ControlStyle, Padding, TweenOptions};

/// Shared compact header/row geometry. None inherits current Style tokens.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DisclosureStyle {
    pub surface: ControlStyle,
    pub height: Option<f32>,
    pub padding: Option<Padding>,
    pub chevron_size: Option<f32>,
    pub chevron_stroke: Option<f32>,
    pub icon_size: Option<f32>,
    pub icon_gap: Option<f32>,
    pub font_size: Option<f32>,
    pub motion: Option<TweenOptions>,
}
impl DisclosureStyle {
    pub fn height(mut self, value: f32) -> Self {
        self.height = Some(super::dimension(value));
        self
    }
    pub fn padding(mut self, value: Padding) -> Self {
        self.padding = Some(value);
        self
    }
    pub fn surface(mut self, value: ControlStyle) -> Self {
        self.surface = value;
        self
    }
    pub fn chevron(mut self, size: f32, stroke: f32) -> Self {
        self.chevron_size = Some(super::dimension(size));
        self.chevron_stroke = Some(super::dimension(stroke));
        self
    }
    pub fn icon(mut self, size: f32, gap: f32) -> Self {
        self.icon_size = Some(super::dimension(size));
        self.icon_gap = Some(super::dimension(gap));
        self
    }
    pub fn motion(mut self, value: TweenOptions) -> Self {
        self.motion = Some(value);
        self
    }
    pub(crate) fn merge(&mut self, rhs: &Self) {
        self.surface.merge(rhs.surface);
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f.clone(); })* }; }
        set!(
            height,
            padding,
            chevron_size,
            chevron_stroke,
            icon_size,
            icon_gap,
            font_size,
            motion
        );
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollapsingStyle {
    pub header: DisclosureStyle,
    pub content_padding: Option<Padding>,
    pub spacing: Option<f32>,
    pub animate_height: Option<bool>,
    pub motion: Option<TweenOptions>,
}
impl CollapsingStyle {
    pub fn header(mut self, value: DisclosureStyle) -> Self {
        self.header = value;
        self
    }
    pub fn content_padding(mut self, value: Padding) -> Self {
        self.content_padding = Some(value);
        self
    }
    pub fn spacing(mut self, value: f32) -> Self {
        self.spacing = Some(super::dimension(value));
        self
    }
    pub fn animate_height(mut self, value: bool) -> Self {
        self.animate_height = Some(value);
        self
    }
    pub fn motion(mut self, value: TweenOptions) -> Self {
        self.motion = Some(value);
        self
    }
    pub(crate) fn merge(&mut self, rhs: &Self) {
        self.header.merge(&rhs.header);
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f.clone(); })* }; }
        set!(content_padding, spacing, animate_height, motion);
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeStyle {
    pub row: DisclosureStyle,
    pub indent: Option<f32>,
    /// Optional guide stroke. Some(Border::NONE) explicitly suppresses guides.
    pub guides: Option<crate::Border>,
    pub scroll: Option<crate::ScrollStyle>,
}
impl TreeStyle {
    pub fn row(mut self, value: DisclosureStyle) -> Self {
        self.row = value;
        self
    }
    pub fn indent(mut self, value: f32) -> Self {
        self.indent = Some(super::dimension(value));
        self
    }
    pub fn guides(mut self, value: crate::Border) -> Self {
        self.guides = Some(value);
        self
    }
    pub fn scroll(mut self, value: crate::ScrollStyle) -> Self {
        self.scroll = Some(value);
        self
    }
    pub(crate) fn merge(&mut self, rhs: &Self) {
        self.row.merge(&rhs.row);
        if rhs.indent.is_some() {
            self.indent = rhs.indent;
        }
        if rhs.guides.is_some() {
            self.guides = rhs.guides;
        }
        if rhs.scroll.is_some() {
            self.scroll = rhs.scroll;
        }
    }
}
