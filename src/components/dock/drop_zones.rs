use crate::{Layout, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockSide {
    Left,
    Right,
    Top,
    Bottom,
}
impl DockSide {
    pub fn axis(self) -> Layout {
        match self {
            Self::Left | Self::Right => Layout::Horizontal,
            Self::Top | Self::Bottom => Layout::Vertical,
        }
    }
    pub(super) fn before(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockDropZone {
    Center,
    Edge(DockSide),
}
impl DockDropZone {
    /// Edges occupy the outer quarter. Corner ties prefer the nearer normalized edge.
    pub fn at(bounds: Rect, pointer: Vec2) -> Option<Self> {
        if !bounds.is_finite() || bounds.is_empty() || !bounds.contains(pointer) {
            return None;
        }
        let p = (pointer - bounds.min) / bounds.size();
        let edges = [
            (p.x, DockSide::Left),
            (1.0 - p.x, DockSide::Right),
            (p.y, DockSide::Top),
            (1.0 - p.y, DockSide::Bottom),
        ];
        let (distance, side) = edges.into_iter().min_by(|a, b| a.0.total_cmp(&b.0))?;
        Some(if distance < 0.25 {
            Self::Edge(side)
        } else {
            Self::Center
        })
    }
    /// Prospective half-panel allocation for an edge drop, whole group for center.
    pub fn preview(self, bounds: Rect) -> Rect {
        let mut rect = bounds;
        match self {
            Self::Center => {}
            Self::Edge(DockSide::Left) => rect.max.x = bounds.center().x,
            Self::Edge(DockSide::Right) => rect.min.x = bounds.center().x,
            Self::Edge(DockSide::Top) => rect.max.y = bounds.center().y,
            Self::Edge(DockSide::Bottom) => rect.min.y = bounds.center().y,
        }
        rect
    }
}
