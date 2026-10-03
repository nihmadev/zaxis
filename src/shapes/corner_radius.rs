use super::Rect;

/// Independent corner radii, clockwise from the top left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadius {
    pub const ZERO: Self = Self::all(0.0);
    pub const fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub(crate) fn values(self, rect: Rect) -> [f32; 4] {
        let mut r = [
            self.top_left.max(0.0),
            self.top_right.max(0.0),
            self.bottom_right.max(0.0),
            self.bottom_left.max(0.0),
        ];
        // CSS-style normalization preserves the relative size of asymmetric corners.
        let size = rect.size();
        let factor = [
            (size.x, r[0] + r[1]),
            (size.x, r[3] + r[2]),
            (size.y, r[0] + r[3]),
            (size.y, r[1] + r[2]),
        ]
        .into_iter()
        .fold(
            1.0_f32,
            |f, (side, sum)| if sum > 0.0 { f.min(side / sum) } else { f },
        );
        for radius in &mut r {
            *radius *= factor;
        }
        r
    }
}

impl From<f32> for CornerRadius {
    fn from(radius: f32) -> Self {
        Self::all(radius)
    }
}
