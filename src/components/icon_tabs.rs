use std::{hash::Hash, time::Duration};

use super::{Response, Sense, Tooltip, Ui, Widget};
use crate::{
    context::Paint, Border, Color, Easing, Id, ImageSource, Rect, Shape, TweenOptions, Vec2,
};

struct IconTab<T> {
    value: T,
    icon: ImageSource,
    label: String,
}

/// A vertical strip of icon buttons bound to a selection, for a sidebar or a rail.
///
/// The selected tab gets a soft fill, a hairline outline and the active icon color;
/// hovering fills it faintly. All of it eases in and out on the shared animation
/// scheduler. Each tab has a tooltip with its label. The widget's [`Response`] is the whole
/// strip and reports `changed()` on the pass that selects a different tab.
///
/// ```no_run
/// # fn tabs(ui: &mut zaxis::Ui<'_>) {
/// # #[derive(PartialEq, Hash, Clone, Copy)] enum Page { Aim, Visuals }
/// # let mut page = Page::Aim;
/// # let icon = || zaxis::ImageSource::encoded(Vec::<u8>::new());
/// ui.add(zaxis::IconTabs::new(&mut page, [
///     (Page::Aim, icon(), "Aim"),
///     (Page::Visuals, icon(), "Visuals"),
/// ]));
/// # }
/// ```
pub struct IconTabs<'a, T> {
    selected: &'a mut T,
    tabs: Vec<IconTab<T>>,
    id: Option<Id>,
    tab_size: Vec2,
    icon_size: f32,
    gap: f32,
    rounding: f32,
    idle: Option<Color>,
    active: Option<Color>,
    fill: Option<Color>,
    border: Option<Color>,
    selected_opacity: f32,
    hover_opacity: f32,
    outline_opacity: f32,
    duration: Duration,
}

impl<'a, T: PartialEq + Hash + Clone> IconTabs<'a, T> {
    /// `(value, icon, label)` per tab. The icon is any [`ImageSource`], such as a bundled
    /// Lucide `&'static Icon`; the label is the tooltip.
    pub fn new<I, L>(selected: &'a mut T, tabs: impl IntoIterator<Item = (T, I, L)>) -> Self
    where
        I: Into<ImageSource>,
        L: Into<String>,
    {
        Self {
            selected,
            tabs: tabs
                .into_iter()
                .map(|(value, icon, label)| IconTab {
                    value,
                    icon: icon.into(),
                    label: label.into(),
                })
                .collect(),
            id: None,
            tab_size: Vec2::new(42.0, 44.0),
            icon_size: 24.0,
            gap: 5.0,
            rounding: 6.0,
            idle: None,
            active: None,
            fill: None,
            border: None,
            selected_opacity: 0.52,
            hover_opacity: 0.18,
            outline_opacity: 0.48,
            duration: Duration::from_millis(160),
        }
    }

    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    /// Size of one tab; 42 × 44.
    pub fn tab_size(mut self, size: Vec2) -> Self {
        self.tab_size = size.max(Vec2::splat(8.0));
        self
    }
    /// Side of the square an icon is drawn in; 24.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size.max(1.0);
        self
    }
    /// Distance between tabs; 5.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.rounding = radius.max(0.0);
        self
    }
    /// Icon color of unselected tabs; `Style::muted_text`.
    pub fn idle_color(mut self, color: Color) -> Self {
        self.idle = Some(color);
        self
    }
    /// Icon color of the selected tab; `Style::accent`.
    pub fn active_color(mut self, color: Color) -> Self {
        self.active = Some(color);
        self
    }
    /// Base color of the selected and hover fills; `Style::selected_fill`.
    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }
    /// Color of the selected tab's outline; `Style::border`.
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }
    /// Opacity of the fill for the selected tab, the hovered tab and the selected
    /// tab's outline; 0.52, 0.18 and 0.48.
    pub fn opacities(mut self, selected: f32, hover: f32, outline: f32) -> Self {
        self.selected_opacity = selected.clamp(0.0, 1.0);
        self.hover_opacity = hover.clamp(0.0, 1.0);
        self.outline_opacity = outline.clamp(0.0, 1.0);
        self
    }
    /// Duration of the color and fill transitions; 160 ms.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

impl<T: PartialEq + Hash + Clone> Widget for IconTabs<'_, T> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let style = ui.style().clone();
        let count = self.tabs.len();
        let strip = Vec2::new(
            self.tab_size.x,
            self.tab_size.y * count as f32 + self.gap * count.saturating_sub(1) as f32,
        );
        let bounds = ui.allocate_space(strip);
        let id = ui
            .scope
            .with(("icon-tabs", self.id.unwrap_or_else(|| Id::new(count))));
        let mut whole = ui.response(id, bounds, false);
        let idle = self.idle.unwrap_or(style.muted_text);
        let active = self.active.unwrap_or(style.accent);
        let fill = self.fill.unwrap_or(style.selected_fill);
        let outline = self.border.unwrap_or(style.border.color);
        let ease = || TweenOptions::new(self.duration).easing(Easing::QuadOut);
        for (index, tab) in self.tabs.into_iter().enumerate() {
            let key = Id::new(&tab.value);
            let rect = Rect::from_min_size(
                bounds.min + Vec2::new(0.0, (self.tab_size.y + self.gap) * index as f32),
                self.tab_size,
            );
            let mut response = ui.interact(rect, ("icon-tab", key), Sense::CLICK | Sense::FOCUS);
            let selected = *self.selected == tab.value;
            if response.clicked() && !selected {
                *self.selected = tab.value.clone();
                whole.changed = true;
                response.changed = true;
            }
            let selected = *self.selected == tab.value;
            let fill_opacity = if selected {
                self.selected_opacity
            } else if response.hovered && response.enabled {
                self.hover_opacity
            } else {
                0.0
            };
            let fill_opacity = ui.transition(("fill", key), fill_opacity, ease()).value;
            let outline_opacity = ui
                .transition(
                    ("outline", key),
                    if selected { self.outline_opacity } else { 0.0 },
                    ease(),
                )
                .value;
            let icon = ui
                .transition(("icon", key), if selected { active } else { idle }, ease())
                .value;
            let icon = if response.enabled {
                icon
            } else {
                style.disabled_text
            };
            let mut shapes = Vec::new();
            if fill_opacity > 0.0 {
                shapes.push(Paint::Shape(
                    Shape::rect(rect, fill.with_opacity(fill_opacity))
                        .corner_radius(self.rounding)
                        .into(),
                ));
            }
            let ring = if response.focus_visible {
                Some(style.focus_border)
            } else if outline_opacity > 0.0 {
                Some(Border::new(1.0, outline.with_opacity(outline_opacity)))
            } else {
                None
            };
            if let Some(border) = ring {
                shapes.push(Paint::Shape(
                    Shape::rect(rect, Color::TRANSPARENT)
                        .corner_radius(self.rounding)
                        .border(border)
                        .into(),
                ));
            }
            if !shapes.is_empty() {
                ui.context
                    .paint(response.id.with("surface"), ui.window, ui.clip, shapes);
            }
            let glyph = Rect::from_min_size(
                rect.center() - Vec2::splat(self.icon_size * 0.5),
                Vec2::splat(self.icon_size),
            );
            ui.paint_image_in(response.id.with("icon"), tab.icon, glyph, icon);
            Tooltip::new(tab.label).show(ui, response);
        }
        whole
    }
}

impl Ui<'_> {
    /// Shorthand for `ui.add(IconTabs::new(selected, tabs))`.
    pub fn icon_tabs<T, I, L>(
        &mut self,
        selected: &mut T,
        tabs: impl IntoIterator<Item = (T, I, L)>,
    ) -> Response
    where
        T: PartialEq + Hash + Clone,
        I: Into<ImageSource>,
        L: Into<String>,
    {
        self.add(IconTabs::new(selected, tabs))
    }
}
