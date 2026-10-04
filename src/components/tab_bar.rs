use super::{button::Button, Response, Ui};
use crate::{context::Paint, CornerRadius, Id, Rect, Vec2};
use std::hash::Hash;

impl Ui<'_> {
    /// Tabs with identities derived from their values. They share the width
    /// equally; labels that do not fit that share get their natural width, and
    /// when even those exceed the row the strip scrolls horizontally (wheel,
    /// touchpad, thumb), revealing a tab when it is clicked.
    pub fn tab_bar<T: PartialEq + Hash, L: Into<String>>(
        &mut self,
        selected: &mut T,
        tabs: impl IntoIterator<Item = (T, L)>,
    ) -> Vec<Response> {
        let tabs: Vec<(T, String)> = tabs
            .into_iter()
            .map(|(value, label)| (value, label.into()))
            .collect();
        let style = self.style().clone();
        let tab_style = super::theme::ButtonStyle {
            surface: super::theme::ControlStyle {
                idle: super::theme::SurfaceStyle {
                    foreground: Some(style.muted_text),
                    ..Default::default()
                },
                hover: super::theme::SurfaceStyle {
                    foreground: Some(style.text_color),
                    ..Default::default()
                },
                selected: super::theme::SurfaceStyle {
                    foreground: Some(style.text_color),
                    ..super::theme::SurfaceStyle::fill(crate::Color::TRANSPARENT)
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let height = 36.0_f32.max(style.control_height);
        let weights = style.typography.weights;
        let tab_button = |value: &T, label: &str, active: bool| {
            Button::new(label)
                .id_source(value)
                .variant(super::button_variant::ButtonVariant::Ghost)
                .style(super::theme::ButtonStyle {
                    font_weight: Some(if active {
                        weights.selected()
                    } else {
                        weights.control
                    }),
                    ..tab_style
                })
                .selected(active)
                .corner_radius(CornerRadius::ZERO)
        };
        let count = tabs.len().max(1) as f32;
        let gaps = style.spacing * tabs.len().saturating_sub(1) as f32;
        let available = self.available_width();
        let equal = ((available - gaps) / count).max(0.0);
        let natural: Vec<f32> = tabs
            .iter()
            // Measure with the selected weight so activating a tab never reflows the strip.
            .map(|(value, label)| tab_button(value, label, true).natural_size(self).x)
            .collect();
        let first = tabs.first().map(|(value, _)| Id::new(value));
        let total = natural.iter().sum::<f32>() + gaps;
        let (widths, content_width) = if natural.iter().all(|width| *width <= equal) {
            (vec![equal; tabs.len()], None)
        } else if total <= available {
            let extra = (available - total) / count;
            (natural.iter().map(|width| width + extra).collect(), None)
        } else {
            (natural, Some(total))
        };
        let build = |ui: &mut Ui<'_>| -> Vec<(Response, bool)> {
            let rows: Vec<(Response, bool)> = tabs
                .into_iter()
                .zip(&widths)
                .map(|((value, label), width)| {
                    let active = *selected == value;
                    let mut response = ui.add(
                        tab_button(&value, &label, active).min_size(Vec2::new(*width, height)),
                    );
                    if response.clicked() && *selected != value {
                        *selected = value;
                        response.changed = true;
                    }
                    if response.changed && content_width.is_some() {
                        ui.scroll_to_response(&response);
                    }
                    (response, active)
                })
                .collect();
            // One hairline under the row; the active tab overdraws it with the accent.
            if let (Some((first, _)), Some((last, _))) = (rows.first(), rows.last()) {
                let line = |from: f32, to: f32, top: f32, bottom: f32, color| {
                    Paint::Shape(
                        crate::Shape::rect(
                            Rect::from_min_max(Vec2::new(from, top), Vec2::new(to, bottom)),
                            color,
                        )
                        .into(),
                    )
                };
                let bottom = last.rect.max.y;
                let mut shapes = vec![line(
                    first.rect.min.x,
                    last.rect.max.x,
                    bottom - 1.0,
                    bottom,
                    style.border.color,
                )];
                for (response, active) in &rows {
                    if *active {
                        let r = response.rect;
                        shapes.push(line(r.min.x, r.max.x, bottom - 2.0, bottom, style.accent));
                    }
                }
                ui.context
                    .paint(first.id.with("tab-line"), ui.window, ui.clip, shapes);
            }
            rows
        };
        let rows = match content_width {
            None => self.horizontal(build),
            Some(width) => {
                let mut scroll = style.scroll;
                scroll.padding = crate::Padding::all(0.0);
                scroll.spacing = style.spacing;
                crate::ScrollArea::horizontal()
                    .id_source(("tab-bar", first))
                    .style(scroll)
                    .max_height(height)
                    .content_width(width)
                    .overlay_scrollbars(true)
                    .show(self, build)
                    .inner
            }
        };
        rows.into_iter().map(|(response, _)| response).collect()
    }
}
