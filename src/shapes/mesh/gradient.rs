use super::Mesh;
use crate::{
    shapes::outline::{arc_segments, rounded_outline},
    Color, CornerRadius, Rect, TextureId, Vec2,
};

impl Mesh {
    /// A sampled color grid with the same rounded, antialiased contour as solid shapes.
    pub(crate) fn gradient(
        &mut self,
        rect: Rect,
        rounding: CornerRadius,
        colors: &[Color],
        columns: usize,
        rows: usize,
        scale: f32,
    ) {
        if rect.is_empty() {
            return;
        }
        let colors: Vec<_> = colors.iter().map(|color| color.linear()).collect();
        let sample = |position: Vec2| {
            let t = ((position - rect.min) / rect.size()).clamp(Vec2::ZERO, Vec2::ONE);
            let x = t.x * (columns - 1) as f32;
            let y = t.y * (rows - 1) as f32;
            let column = (x as usize).min(columns - 2);
            let row = (y as usize).min(rows - 2);
            let mut color = [0.0; 4];
            for (channel, value) in color.iter_mut().enumerate() {
                let top = colors[row * columns + column][channel] * (1.0 - (x - column as f32))
                    + colors[row * columns + column + 1][channel] * (x - column as f32);
                let bottom = colors[(row + 1) * columns + column][channel]
                    * (1.0 - (x - column as f32))
                    + colors[(row + 1) * columns + column + 1][channel] * (x - column as f32);
                *value = top * (1.0 - (y - row as f32)) + bottom * (y - row as f32);
            }
            color
        };
        let half_pixel = (0.5 / scale).min(rect.size().min_element() * 0.5);
        let radii = rounding.values(rect);
        let segments = radii.map(|r| arc_segments((r + half_pixel) * scale));
        let outer = rounded_outline(
            Rect::from_min_max(
                rect.min - Vec2::splat(half_pixel),
                rect.max + Vec2::splat(half_pixel),
            ),
            radii.map(|r| r + half_pixel),
            segments,
        );
        let inner_rect = rect.shrink(half_pixel);
        let inner_radii = radii.map(|r| (r - half_pixel).max(0.0));
        let inner_radii = CornerRadius {
            top_left: inner_radii[0],
            top_right: inner_radii[1],
            bottom_right: inner_radii[2],
            bottom_left: inner_radii[3],
        }
        .values(inner_rect);
        let inner_radius = inner_radii.into_iter().fold(0.0_f32, f32::max);
        let inner = rounded_outline(inner_rect, inner_radii, segments);
        let start = self.vertices.len();
        self.ring(&outer, &inner, [0.0; 4], Color::WHITE.linear());
        for (i, vertex) in self.vertices[start..].iter_mut().enumerate() {
            vertex.color = sample(Vec2::from_array(vertex.position));
            if i % 2 == 0 {
                vertex.color[3] = 0.0;
            }
        }
        for row in 0..rows - 1 {
            for column in 0..columns - 1 {
                let min = rect.min
                    + rect.size()
                        * Vec2::new(
                            column as f32 / (columns - 1) as f32,
                            row as f32 / (rows - 1) as f32,
                        );
                let max = rect.min
                    + rect.size()
                        * Vec2::new(
                            (column + 1) as f32 / (columns - 1) as f32,
                            (row + 1) as f32 / (rows - 1) as f32,
                        );
                let cell = Rect::from_min_max(min.max(inner_rect.min), max.min(inner_rect.max));
                if cell.is_empty() {
                    continue;
                }
                // Only corner cells need polygon clipping. Straight edges and interior
                // cells use four vertices without allocating or testing the contour.
                if (cell.min.x >= inner_rect.min.x + inner_radius
                    && cell.max.x <= inner_rect.max.x - inner_radius)
                    || (cell.min.y >= inner_rect.min.y + inner_radius
                        && cell.max.y <= inner_rect.max.y - inner_radius)
                {
                    let start = self.vertices.len();
                    self.quad(
                        cell,
                        Rect::from_min_max(Vec2::splat(0.5), Vec2::splat(0.5)),
                        Color::WHITE.linear(),
                        TextureId::WHITE,
                    );
                    for vertex in &mut self.vertices[start..] {
                        vertex.color = sample(Vec2::from_array(vertex.position));
                    }
                    continue;
                }
                let mut points = vec![
                    cell.min,
                    Vec2::new(cell.max.x, cell.min.y),
                    cell.max,
                    Vec2::new(cell.min.x, cell.max.y),
                ];
                // Clip cells to the rounded contour; interiors share exact grid edges.
                for edge in 0..inner.len() {
                    let a = inner[edge];
                    let b = inner[(edge + 1) % inner.len()];
                    let distance = |p: Vec2| (b - a).perp_dot(p - a);
                    let old = std::mem::take(&mut points);
                    if old.is_empty() {
                        break;
                    }
                    let mut previous = *old.last().unwrap();
                    let mut previous_distance = distance(previous);
                    for point in old {
                        let current_distance = distance(point);
                        if (current_distance >= 0.0) != (previous_distance >= 0.0) {
                            points.push(
                                previous
                                    + (point - previous)
                                        * (previous_distance
                                            / (previous_distance - current_distance)),
                            );
                        }
                        if current_distance >= 0.0 {
                            points.push(point);
                        }
                        previous = point;
                        previous_distance = current_distance;
                    }
                }
                let start = self.vertices.len();
                self.polygon(&points, Color::WHITE.linear());
                for vertex in &mut self.vertices[start..] {
                    vertex.color = sample(Vec2::from_array(vertex.position));
                }
            }
        }
    }
}
