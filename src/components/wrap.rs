//! Items in reading order that continue on the next line when the width runs out.
use super::{columns::dimension, ui::WrapRule, Ui};
use crate::{layout::LayoutCursor, Align, Id, Layout, Rect, Vec2};
use std::hash::Hash;

/// A horizontal flow of arbitrary content that wraps.
///
/// The closure receives an ordinary [`Ui`]; every `ui.add(..)`, `ui.horizontal(..)`,
/// `ui.vertical(..)`, [`Ui::item`] or `Grid` is one indivisible item, shown in the order it
/// was added. Each item is built **once**, against the full width of the wrap, so a button, a
/// text field or text that wraps itself never shrinks to what the previous line left over.
/// Then the items are packed into lines and placed with their paint, input regions, scroll
/// scopes, popups, IME area and accessibility geometry together.
///
/// An item joins the current line when the line, the gap and the item fit in the width
/// (landing exactly on the edge fits); otherwise it starts the next line. A line is as tall
/// as its tallest item, and the gap between lines applies only between lines.
///
/// * **Too wide.** An item wider than the wrap gets a line of its own at the leading edge,
///   is not shrunk or clipped, and widens the area the wrap reports ([`WrapOutput::rect`]),
///   so an enclosing [`ScrollArea`](crate::ScrollArea) can scroll to it. Clip it yourself with
///   [`Ui::with_max_width`] to limit it.
/// * **`fill` and `spacer`.** There is no leftover space to share. `ui.fill(..)` becomes an
///   item as wide as the wrap (so on a line of its own); `ui.spacer()` is ignored and reported
///   as a [`DiagnosticKind::InvalidUsage`](crate::DiagnosticKind::InvalidUsage). Inside an item,
///   `horizontal` with `fill` and `spacer` works as usual.
/// * **`add_space`.** Adds its width between two items of one line; it disappears at the
///   start or end of a line.
/// * **Ids.** An item's ids come from the order of `ui.add` calls, not from its line. Give
///   items of a list that can be reordered or filtered a stable key with [`Ui::item`] or
///   [`Ui::push_id`].
///
/// The wrap adds no focus stop and no accessibility node; its items keep their own, in
/// declaration order.
pub struct Wrap {
    id: Id,
    width: Option<f32>,
    gap: Option<Vec2>,
    lines: Align,
    items: Align,
}

/// Geometry of a [`Wrap`], in the coordinates of the [`Ui`] it was shown in. Like
/// [`GridOutput`](crate::GridOutput), these are build-time rectangles: ancestors that place
/// content later (scroll areas, windows, `Ui::visual`) move them. Map one to the screen with
/// [`Context::visual_rect`](crate::Context::visual_rect) once the enclosing placement is done.
pub struct WrapOutput<R> {
    /// The value returned by the closure.
    pub inner: R,
    /// The area the wrap occupies: its width (wider for a too-wide item) and the height of
    /// all lines. Zero height without items.
    pub rect: Rect,
    /// Each line's rectangle, after the lines are aligned.
    pub lines: Vec<Rect>,
    /// Each item in declaration order. `add_space` is not an item; an item with no extent
    /// is a zero rectangle where the next item would start.
    pub items: Vec<Rect>,
}

impl Wrap {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            width: None,
            gap: None,
            lines: Align::Start,
            items: Align::Start,
        }
    }

    /// Wrap at this width instead of the available one.
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(dimension(width));
        self
    }

    /// The same gap between items and between lines. The default is the spacing of the
    /// [`Ui`], the one between the items of a `horizontal` row.
    #[track_caller]
    pub fn gap(self, gap: f32) -> Self {
        self.spacing(Vec2::splat(gap))
    }

    /// The gap between items of a line (`x`) and between lines (`y`).
    #[track_caller]
    pub fn spacing(mut self, spacing: Vec2) -> Self {
        self.gap = Some(Vec2::new(dimension(spacing.x), dimension(spacing.y)));
        self
    }

    /// Where each line sits in the width: at the start, centered or at the end. A line wider
    /// than the width stays at the start.
    pub fn align_lines(mut self, align: Align) -> Self {
        self.lines = align;
        self
    }

    /// Where each item sits in the height of its line.
    pub fn align_items(mut self, align: Align) -> Self {
        self.items = align;
        self
    }

    /// Show the wrap. The closure runs once.
    pub fn show<R>(self, ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> WrapOutput<R> {
        ui.layout_item(|ui| self.show_content(ui, build))
    }

    fn show_content<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> WrapOutput<R> {
        let scope = ui.scope.with(("wrap", self.id));
        let origin = ui.layout.cursor;
        let width = self
            .width
            .unwrap_or_else(|| ui.available_width())
            .clamp(0.0, crate::components::ui::MAX_WRAP_EXTENT);
        let spacing = ui.layout.spacing.max(0.0);
        let gap = self.gap.unwrap_or(Vec2::splat(spacing));
        let height = ui.available_height();
        let mut child = Ui {
            context: &mut *ui.context,
            window: ui.window,
            scope,
            sequence: 0,
            clip: ui.clip,
            layout: LayoutCursor::new(
                Rect::from_min_size(origin, Vec2::new(width, height)),
                Layout::Horizontal,
                spacing,
            ),
            enabled: ui.enabled,
            backdrop_blur: ui.backdrop_blur,
            hover_style: ui.hover_style,
            flow: None,
            local_style: ui.local_style.clone(),
            local_style_revision: ui.local_style_revision,
        };
        child.begin_wrap(WrapRule {
            width,
            gap,
            lines: self.lines,
            items: self.items,
        });
        let inner = build(&mut child);
        let layout = child.finish_wrap();
        ui.allocate_space(layout.size);
        WrapOutput {
            inner,
            rect: Rect::from_min_size(origin, layout.size),
            lines: layout.lines,
            items: layout.items,
        }
    }
}

impl Ui<'_> {
    /// Show a [`Wrap`] with the default gap and alignment.
    pub fn wrap<R>(
        &mut self,
        source: impl Hash,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> WrapOutput<R> {
        Wrap::new(source).show(self, build)
    }

    /// Group controls into one indivisible layout item, laid out in a row, under the stable
    /// id scope `source`: a wrap never splits it across lines, and reordering or filtering
    /// items keyed by `source` keeps each one's focus, text and state.
    pub fn item<R>(&mut self, source: impl Hash, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.push_id(source, |ui| ui.horizontal(build))
    }
}
