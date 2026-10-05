//! Placement of the modal surface. Pure functions of the viewport and the
//! previous pass's measurements, so resizing and DPI changes only re-run these.
use crate::{CornerRadius, Padding, Rect, Transform, Vec2};

/// Where the surface sits. Edge anchors give a sheet that slides in from that
/// edge; they use the same layers, input blocking and focus handling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModalAnchor {
    #[default]
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

impl ModalAnchor {
    fn edge(self) -> Option<Vec2> {
        match self {
            Self::Center => None,
            Self::Left => Some(Vec2::new(-1.0, 0.0)),
            Self::Right => Some(Vec2::new(1.0, 0.0)),
            Self::Top => Some(Vec2::new(0.0, -1.0)),
            Self::Bottom => Some(Vec2::new(0.0, 1.0)),
        }
    }
}

/// Sizes of the previous pass. Retained per modal until it has fully closed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measure {
    pub width: f32,
    pub header: f32,
    pub body: f32,
    pub footer: f32,
    pub parts: u8,
    pub last_frame: u64,
}

pub(crate) struct Metrics {
    pub anchor: ModalAnchor,
    pub width: Option<f32>,
    pub max_height: Option<f32>,
    pub min_width: f32,
    pub max_width: f32,
    pub margin: f32,
    pub padding: Padding,
    pub gap: f32,
}

pub(crate) struct Geometry {
    pub surface: Rect,
    /// Space the scrolling body may occupy; header and actions keep theirs.
    pub body_height: f32,
    pub rounding: CornerRadius,
}

pub(crate) fn geometry(
    viewport: Rect,
    m: &Metrics,
    measure: &Measure,
    rounding: CornerRadius,
) -> Geometry {
    let edge = m.anchor.edge();
    let margin = if edge.is_some() { 0.0 } else { m.margin };
    let avail = (viewport.size() - Vec2::splat(2.0 * margin)).max(Vec2::splat(1.0));
    let pad = m.padding.size();
    let natural = if measure.width > 0.0 {
        measure.width + pad.x
    } else {
        m.max_width
    };
    let width = m
        .width
        .unwrap_or(natural.clamp(m.min_width, m.max_width.max(m.min_width)))
        .min(avail.x)
        .max(1.0);
    let height_cap = m.max_height.unwrap_or(f32::INFINITY).min(avail.y);
    // Gaps exist between the parts that are present.
    let gaps = f32::from(measure.parts.saturating_sub(1)) * m.gap;
    let fixed = pad.y + measure.header + measure.footer + gaps;
    let body_height = measure.body.min((height_cap - fixed).max(0.0));
    let height = match edge {
        Some(e) if e.x != 0.0 => avail.y,
        // Only the body gives way; header and actions keep their natural height.
        _ => (fixed + body_height).max(1.0),
    };
    let width = match edge {
        Some(e) if e.y != 0.0 => avail.x,
        _ => width,
    };
    let center = viewport.center();
    let mut min = Vec2::new(center.x - width * 0.5, center.y - height * 0.5);
    match edge {
        Some(e) => {
            if e.x < 0.0 {
                min.x = viewport.min.x;
            } else if e.x > 0.0 {
                min.x = viewport.max.x - width;
            }
            if e.y < 0.0 {
                min.y = viewport.min.y;
            } else if e.y > 0.0 {
                min.y = viewport.max.y - height;
            }
        }
        None => {
            // Stay inside the margins; when even that is impossible keep the
            // bottom (the actions) on screen and let the top overflow.
            min.x = min.x.clamp(
                viewport.min.x + margin,
                (viewport.max.x - margin - width).max(viewport.min.x),
            );
            let low = viewport.min.y + margin;
            let high = viewport.max.y - margin - height;
            min.y = if high >= low {
                min.y.clamp(low, high)
            } else {
                viewport.max.y - margin - height
            };
        }
    }
    let mut rounding_corners = rounding;
    if let Some(e) = edge {
        if e.x < 0.0 {
            rounding_corners.top_left = 0.0;
            rounding_corners.bottom_left = 0.0;
        } else if e.x > 0.0 {
            rounding_corners.top_right = 0.0;
            rounding_corners.bottom_right = 0.0;
        } else if e.y < 0.0 {
            rounding_corners.top_left = 0.0;
            rounding_corners.top_right = 0.0;
        } else {
            rounding_corners.bottom_left = 0.0;
            rounding_corners.bottom_right = 0.0;
        }
    }
    Geometry {
        surface: Rect::from_min_size(min, Vec2::new(width, height)),
        body_height,
        rounding: rounding_corners,
    }
}

/// Hidden pose to visible pose. `t` is 0 hidden, 1 shown.
pub(crate) fn pose(
    anchor: ModalAnchor,
    surface: Rect,
    t: f32,
    scale: f32,
    offset: f32,
) -> Transform {
    let hidden = 1.0 - t.clamp(0.0, 1.0);
    match anchor.edge() {
        Some(e) => Transform::translation(e * surface.size() * hidden),
        None => Transform::around(
            surface.center(),
            1.0 + (scale - 1.0) * hidden,
            Vec2::new(0.0, offset * hidden),
        ),
    }
}
