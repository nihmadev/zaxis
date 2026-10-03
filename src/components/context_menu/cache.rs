use super::{
    paint::{self, Metrics},
    ContextMenuItem, ContextMenuStyle, Ui,
};
use crate::Id;

pub(super) struct MenuCache {
    items: Vec<ContextMenuItem>,
    dimensions: [f32; 4],
    pub metrics: Metrics,
    pub enabled: Vec<Id>,
    pub offsets: Vec<f32>,
}
impl MenuCache {
    fn dimensions(style: &ContextMenuStyle) -> [f32; 4] {
        [
            style.min_width,
            style.row_height,
            style.separator_height,
            style.font_size,
        ]
    }
    pub fn matches(&self, items: &[ContextMenuItem], style: &ContextMenuStyle) -> bool {
        self.dimensions == Self::dimensions(style) && self.items == items
    }
    pub fn new(ui: &mut Ui<'_>, items: &[ContextMenuItem], style: &ContextMenuStyle) -> Self {
        let mut offsets = Vec::with_capacity(items.len() + 1);
        offsets.push(0.0);
        let mut enabled = Vec::new();
        for item in items {
            offsets.push(
                offsets.last().unwrap()
                    + if item.id.is_some() {
                        style.row_height
                    } else {
                        style.separator_height
                    },
            );
            if item.enabled {
                enabled.extend(item.id);
            }
        }
        Self {
            items: items.to_vec(),
            dimensions: Self::dimensions(style),
            metrics: paint::measure(ui, items, style),
            enabled,
            offsets,
        }
    }
}
