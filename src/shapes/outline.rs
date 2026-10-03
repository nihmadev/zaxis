use super::Rect;
use crate::Vec2;

/// Quarter-circle subdivisions with at most 0.01 physical pixels of chord error.
/// The resource cap only applies to radii beyond normal GPU viewport dimensions.
pub(super) fn arc_segments(radius_pixels: f32) -> usize {
    const ERROR: f64 = 0.01;
    let radius = f64::from(radius_pixels);
    if radius <= ERROR {
        return 1;
    }
    let angle = 4.0 * (ERROR / (2.0 * radius)).sqrt().asin();
    (std::f64::consts::FRAC_PI_2 / angle)
        .ceil()
        .clamp(1.0, 4096.0) as usize
}

pub(super) fn rounded_outline(rect: Rect, radii: [f32; 4], segments: [usize; 4]) -> Vec<Vec2> {
    let centers = [
        rect.min + Vec2::splat(radii[0]),
        Vec2::new(rect.max.x - radii[1], rect.min.y + radii[1]),
        rect.max - Vec2::splat(radii[2]),
        Vec2::new(rect.min.x + radii[3], rect.max.y - radii[3]),
    ];
    let mut points = Vec::new();
    for corner in 0..4 {
        let start = std::f32::consts::PI + corner as f32 * std::f32::consts::FRAC_PI_2;
        for i in 0..=segments[corner] {
            let angle = start + std::f32::consts::FRAC_PI_2 * i as f32 / segments[corner] as f32;
            points.push(centers[corner] + Vec2::new(angle.cos(), angle.sin()) * radii[corner]);
        }
    }
    points
}
