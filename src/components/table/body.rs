use super::{Column, GridRow, Id, Rect, TableStyle, Ui};
use crate::{
    context::{HitAction, HitRegion},
    Grid, GridStyle, Padding, Shape, Vec2,
};
use std::{collections::HashSet, hash::Hash};

pub(super) struct BodySetup {
    pub id: Id,
    pub columns: Vec<Column>,
    pub widths: Vec<f32>,
    pub style: TableStyle,
    pub selected: Option<Id>,
    pub selectable: bool,
    pub clicked: Option<Id>,
    pub measured: Vec<f32>,
    pub rows: Vec<(Id, Rect)>,
    /// Rows in the whole data set when the body is virtualized.
    pub total: Option<usize>,
    pub seen: HashSet<Id>,
    pub origin: Option<Vec2>,
    pub drag: bool,
    pub moved: Option<crate::RowMove>,
}
pub struct TableBody<'a, 'ctx> {
    ui: &'a mut Ui<'ctx>,
    setup: &'a mut BodySetup,
    height: Option<f32>,
    pub(super) index: usize,
}
impl<'a, 'ctx> TableBody<'a, 'ctx> {
    pub(super) fn new(ui: &'a mut Ui<'ctx>, setup: &'a mut BodySetup, height: Option<f32>) -> Self {
        let index = setup.rows.len();
        if setup.origin.is_none() {
            setup.origin = ui
                .context
                .scrolling
                .stack
                .last()
                .map(|&n| ui.context.scrolling.scopes[n].origin);
        }
        Self {
            ui,
            setup,
            height,
            index,
        }
    }
    pub fn row<R>(
        &mut self,
        source: impl Hash,
        build: impl FnOnce(&mut GridRow<'_, '_, '_>) -> R,
    ) -> R {
        self.row_id(Id::new(source), build)
    }
    /// Accept an already constructed Id without hashing it again.
    pub fn row_id<R>(&mut self, row: Id, build: impl FnOnce(&mut GridRow<'_, '_, '_>) -> R) -> R {
        if !self.setup.seen.insert(row) {
            self.ui
                .context
                .report(crate::DiagnosticKind::IdCollision, Some(row), None, || {
                    "duplicate id: two Table rows share one row id".into()
                });
        }
        let id = self.setup.id.with(("selection", row));
        self.ui.context.reserve_hit_order(id);
        // The header is row zero. A click request is the click that selects the row.
        let line = self.index.saturating_add(1).min(u32::MAX as usize) as u32;
        let (selectable, chosen) = (self.setup.selectable, self.setup.selected == Some(row));
        let access = self.ui.a11y_begin(id, crate::AccessRole::Row, |node| {
            node.table().row = Some(line);
            if selectable {
                node.selected(chosen).clicks(id);
            }
        });
        let old_scope = self.ui.scope;
        self.ui.scope = self.setup.id.with("cells");
        self.ui.context.begin_placement(self.ui.window);
        let style = self.setup.style;
        let selected = self.setup.selected == Some(row);
        let mut foreground = style.text_color;
        if selected {
            if let Some(v) = style.row.selected.foreground {
                foreground = v;
            }
        }
        if let Some(v) = style.row.idle.foreground {
            foreground = v;
        }
        let patch = crate::StyleOverrides {
            text_color: Some(foreground),
            ..Default::default()
        };
        let output = self.ui.with_style(&patch, |ui| {
            Grid::new(row)
                .columns(self.setup.columns.iter().cloned())
                .style(GridStyle {
                    surface: Default::default(),
                    padding: Padding::all(0.0),
                    cell_padding: self.setup.style.cell_padding,
                    spacing: Vec2::ZERO,
                    row_min_height: self.setup.style.row_min_height,
                    component_spacing: self.setup.style.component_spacing,
                    fill: crate::Color::TRANSPARENT,
                    rounding: 0.0,
                })
                .resolved(self.setup.widths.to_vec(), self.height)
                .access_row(line)
                .show(ui, |grid| grid.row(row, build))
        });
        let placement = self.ui.context.end_placement();
        self.ui.scope = old_scope;
        let rect = output.rect;
        let response = self.ui.response(id, rect, self.setup.selectable);
        if response.clicked {
            self.setup.selected = Some(row);
            self.setup.clicked = Some(row);
            self.ui.context.request_repaint();
        }
        let style = self.setup.style;
        let fill = if self.setup.selected == Some(row) {
            style.selected_fill
        } else if response.hovered && self.setup.selectable {
            style.hovered_fill
        } else if style.striped && self.index % 2 == 1 {
            style.alternate_fill
        } else {
            style.fill
        };
        let effective = self.ui.style().clone();
        let mut base = crate::components::appearance::Appearance::new(
            fill,
            crate::Border::NONE,
            style.text_color,
        );
        base.opacity = effective.opacity;
        let mut appearance = self.ui.animate_control(
            response,
            crate::HoverStyle::NONE,
            false,
            style.row,
            crate::ControlState::from_response(response, self.setup.selected == Some(row)),
            base,
            style.hovered_fill,
        );
        appearance.ring = None;
        let mut paint = Vec::new();
        appearance.paint_shadow(rect, appearance.rounding, &mut paint);
        appearance.paint_body(
            rect,
            appearance.rounding,
            &effective,
            appearance.blur,
            &mut paint,
        );
        self.ui.context.paint(
            id.with("fill"),
            self.ui.window,
            self.ui.clip.intersect(rect),
            paint,
        );
        if self.setup.selectable && self.ui.enabled {
            self.ui.context.register_hit(HitRegion {
                id,
                window: self.ui.window,
                rect,
                clip: self.ui.clip.intersect(rect),
                action: HitAction::Activate,
            });
        }
        self.ui
            .context
            .place(placement, Vec2::ZERO, self.ui.clip.intersect(rect));
        self.ui.a11y_end(access, Some(rect));
        if self.setup.drag && self.ui.enabled {
            let mut handle = response;
            handle.enabled = true;
            let payload = crate::RowDrag {
                owner: self.setup.id,
                row,
            };
            let owner = self.setup.id;
            crate::DragSource::new(row, payload).attach(self.ui, handle);
            let target = crate::DropTarget::new(row, move |p: &crate::RowDrag| p.owner == owner)
                .zones(crate::DropZones::rows())
                .attach(self.ui, handle);
            if let Some(drop) = target.dropped {
                self.setup.moved = Some(crate::RowMove {
                    row: drop.payload.row,
                    target: row,
                    position: drop.insertion.unwrap_or(crate::Insertion::After),
                });
            }
        }
        if style.separators && style.separator_width > 0.0 {
            let mut x = rect.min.x;
            for (i, &width) in self.setup.widths.iter().enumerate() {
                x += width;
                if i + 1 < self.setup.widths.len() {
                    self.ui.paint(Shape::rect(
                        Rect::from_min_size(
                            Vec2::new(x - style.separator_width, rect.min.y),
                            Vec2::new(style.separator_width, rect.size().y),
                        ),
                        style.separator_color,
                    ));
                }
            }
            self.ui.paint(Shape::rect(
                Rect::from_min_size(
                    Vec2::new(rect.min.x, rect.max.y - style.separator_width),
                    Vec2::new(rect.size().x, style.separator_width),
                ),
                style.separator_color,
            ));
        }
        for (measured, &new) in self.setup.measured.iter_mut().zip(&output.measured_widths) {
            *measured = measured.max(new);
        }
        self.setup.rows.push((row, rect));
        self.index += 1;
        output.inner
    }
}
