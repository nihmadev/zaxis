use std::hash::Hash;

use super::appearance::Appearance;
use super::ui::Ui;
use crate::{Color, Id, Padding, Rect, Vec2};

/// Retained outer height, so the frame can be painted below its content.
pub struct CardState {
    pub height: f32,
    pub last_frame: u64,
}

/// How a card separates itself from the surface behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardVariant {
    /// Raised fill, hairline border and a faint shadow.
    #[default]
    Surface,
    /// Hairline border only; the parent surface shows through.
    Outline,
}

pub struct CardOutput<R> {
    pub inner: R,
    /// Outer frame in the current layout.
    pub rect: Rect,
}

/// A framed, padded container for related content. Content is laid out
/// vertically; use `ui.horizontal` or a `Grid` inside for other arrangements.
///
/// The frame is painted from the previous pass's content height, so a change
/// in content size settles after one scheduled redraw.
///
/// To assistive technology a card is a group that holds its content. It has no name of
/// its own; wrap the content in [`Ui::accessible_group`] to give the section one.
pub struct Card {
    id: Id,
    width: Option<f32>,
    padding: Option<Padding>,
    variant: CardVariant,
    style: super::theme::CardStyle,
}

impl Card {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            width: None,
            padding: None,
            variant: CardVariant::default(),
            style: Default::default(),
        }
    }
    /// Outer width; clamped to the available layout width. Defaults to all of it.
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = super::sanitize::non_negative("Card::width", width).or(self.width);
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn style(mut self, style: super::theme::CardStyle) -> Self {
        self.style = style;
        self
    }

    pub fn show<R>(self, ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> CardOutput<R> {
        let id = ui.scope.with(("card", self.id));
        let theme = ui.style().clone();
        let mut component = theme.card;
        component.merge(self.style);
        let padding = self
            .padding
            .or(component.padding)
            .unwrap_or(Padding::all(16.0));
        let width = self
            .width
            .unwrap_or(f32::INFINITY)
            .min(ui.available_width());
        let previous = ui.context.cards.get(&id).map_or(0.0, |s| s.height);

        let mut height = 0.0;
        let mut origin = Vec2::ZERO;
        let inner = ui.with_width(width, |card| {
            origin = card.layout.cursor;
            let mut body =
                Appearance::new(Color::TRANSPARENT, crate::Border::NONE, theme.text_color);
            body.rounding = theme.rounding;
            body.opacity = theme.opacity;
            body.apply(component.surface);
            if self.variant == CardVariant::Outline {
                body.fill = crate::Gradient::new(Color::TRANSPARENT, Color::TRANSPARENT);
                body.shadow.color = Color::TRANSPARENT;
            }
            let frame = Rect::from_min_size(origin, Vec2::new(width, previous));
            let mut paint = Vec::new();
            body.paint_shadow(frame, body.rounding, &mut paint);
            body.paint_body(frame, body.rounding, &theme, 0.0, &mut paint);
            card.context
                .paint(id.with("frame"), card.window, card.clip, paint);

            let inner_width = (width - padding.size().x).max(0.0);
            // The group is placed where the frame is painted, in the same layout scope.
            let group = card.a11y_begin(id, crate::AccessRole::Group, |_| {});
            let inner = card.horizontal(|row| {
                row.add_space(padding.left);
                row.with_width(inner_width, |column| {
                    column.vertical(|content| {
                        content.add_space(padding.top);
                        let result = build(content);
                        // Exact bottom of the last item, before the trailing gap.
                        height = content.layout.used.y + padding.bottom.max(0.0);
                        content.add_space(padding.bottom);
                        result
                    })
                })
            });
            let bounds = Rect::from_min_size(origin, Vec2::new(width, height));
            card.a11y_end(group, Some(bounds));
            inner
        });
        if (height - previous).abs() > 0.5 {
            ui.context.request_repaint();
        }
        let frame = ui.context.frame;
        ui.context.cards.insert(
            id,
            CardState {
                height,
                last_frame: frame,
            },
        );
        CardOutput {
            inner,
            rect: Rect::from_min_size(origin, Vec2::new(width, height)),
        }
    }
}
