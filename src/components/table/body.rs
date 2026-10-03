use super::{Column, GridRow, Id, Rect, TableStyle, Ui};
use crate::{
    context::{HitAction, HitRegion, Paint},
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
    pub seen: HashSet<Id>,
    pub origin: Option<Vec2>,
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
        assert!(self.setup.seen.insert(row), "duplicate table row ID");
        let id = self.setup.id.with(("selection", row));
        let old_scope = self.ui.scope;
        self.ui.scope = self.setup.id.with("cells");
        self.ui.context.begin_placement(self.ui.window);
        let output = Grid::new(row)
            .columns(self.setup.columns.iter().cloned())
            .style(GridStyle {
                padding: Padding::all(0.0),
                cell_padding: self.setup.style.cell_padding,
                spacing: Vec2::ZERO,
                row_min_height: self.setup.style.row_min_height,
                component_spacing: self.setup.style.component_spacing,
                fill: crate::Color::TRANSPARENT,
                rounding: 0.0,
            })
            .resolved(self.setup.widths.to_vec(), self.height)
            .show(self.ui, |grid| grid.row(row, build));
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
        self.ui.context.paint(
            id.with("fill"),
            self.ui.window,
            self.ui.clip.intersect(rect),
            vec![Paint::Shape(Shape::rect(rect, fill).into())],
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
