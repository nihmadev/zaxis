//! `TabDropZone`: any area of the page (an editor body, a panel) that takes tabs dragged
//! from the strips of its group, and shows where they would land with the same inset
//! outline the strips use.

use std::hash::Hash;

use super::{
    options::{TabDrag, TabMove},
    paint,
    style::Look,
};
use crate::{
    components::{drag_drop::style::DragStyle, Ui},
    DropTarget, Id, Rect,
};

/// A page area that takes dragged tabs. Dropping a tab of its group here reports a
/// [`TabMove`] to the strip `bar` (the `id_source` of that [`TabBar`](crate::TabBar)),
/// at its end; as everywhere, the application moves the tab in its model.
///
/// While a tab of the group is over the area, an inset outline fades in over it. The
/// outline follows `Style::tabs.drop_area`.
pub struct TabDropZone {
    id: Id,
    bar: Id,
    group: Id,
    enabled: bool,
    indicator: bool,
}

/// What a [`TabDropZone`] saw in one pass.
#[derive(Debug)]
pub struct TabDropZoneOutput<R> {
    pub inner: R,
    /// A tab of the group is over the area.
    pub hovering: bool,
    /// A tab was dropped here, once. It is `None` for a tab that already belongs to `bar`.
    pub moved: Option<TabMove>,
}

impl TabDropZone {
    /// `id_source` names the zone; `bar` is the strip that receives the tabs and `group`
    /// the group whose tabs it takes.
    pub fn new(id_source: impl Hash, bar: impl Hash, group: impl Hash) -> Self {
        Self {
            id: Id::new(id_source),
            bar: Id::new(bar),
            group: Id::new(group),
            enabled: true,
            indicator: true,
        }
    }
    /// A disabled zone takes nothing and shows nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Hide the built-in outline when a container supplies its own animated preview.
    pub fn indicator(mut self, indicator: bool) -> Self {
        self.indicator = indicator;
        self
    }

    /// The zone is the area `build` fills.
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> TabDropZoneOutput<R> {
        let group = self.group;
        let out = DropTarget::new(self.id, move |p: &TabDrag| p.group == group)
            .enabled(self.enabled)
            .indicator(false)
            .show(ui, build);
        self.finish(
            ui,
            out.response.rect,
            out.inner,
            out.acceptable,
            out.dropped,
        )
    }

    /// The zone is `rect`, already laid out.
    pub fn at(self, ui: &mut Ui<'_>, rect: Rect) -> TabDropZoneOutput<()> {
        let group = self.group;
        let response = ui.response(ui.scope.with(("tab-zone", self.id)), rect, true);
        let out = DropTarget::new(self.id, move |p: &TabDrag| p.group == group)
            .enabled(self.enabled)
            .indicator(false)
            .attach(ui, response);
        self.finish(ui, rect, (), out.acceptable, out.dropped)
    }

    fn finish<R>(
        self,
        ui: &mut Ui<'_>,
        rect: Rect,
        inner: R,
        hovering: bool,
        dropped: Option<crate::Dropped<TabDrag>>,
    ) -> TabDropZoneOutput<R> {
        let style = ui.style().clone();
        let scale = match ui.context.scale_factor() {
            s if s.is_finite() && s > 0.0 => s,
            _ => 1.0,
        };
        let look = Look::resolve(&style, &super::options::Config::new(self.bar), scale);
        let tween = DragStyle::default().tween(&style);
        let id = ui.scope.with(("tab-zone-area", self.id));
        if self.indicator {
            let amount = ui
                .transition(
                    ("tab-zone", self.id),
                    if hovering && self.enabled {
                        1.0_f32
                    } else {
                        0.0
                    },
                    tween,
                )
                .value;
            paint::drop_area(ui, id, rect, amount, &look);
        }
        let moved = dropped.and_then(|d| {
            (d.payload.bar != self.bar).then_some(TabMove {
                tab: d.payload.tab,
                from_bar: d.payload.bar,
                to_bar: self.bar,
                before: None,
            })
        });
        TabDropZoneOutput {
            inner,
            hovering,
            moved,
        }
    }
}
