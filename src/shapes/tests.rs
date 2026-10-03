use super::{outline::arc_segments, *};
use crate::Vec2;

#[test]
fn gradient_respects_asymmetric_corners_and_linear_colors_at_multiple_scales() {
    let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 40.0));
    for scale in [1.0, 1.5, 2.0] {
        for direction in [
            GradientDirection::Horizontal,
            GradientDirection::Vertical,
            GradientDirection::Diagonal,
        ] {
            let mut mesh = Mesh::default();
            mesh.shape(
                &Shape::Gradient {
                    rect,
                    rounding: CornerRadius {
                        top_left: 20.0,
                        ..CornerRadius::ZERO
                    },
                    gradient: Gradient::new(Color::rgb(255, 0, 0), Color::rgb(0, 0, 255))
                        .direction(direction),
                },
                scale,
            );
            assert_eq!(sample(&mesh, Vec2::new(0.137, 0.137))[3], 0.0);
            assert!(sample(&mesh, Vec2::new(99.137, 0.637))[3] > 0.9);
            let p = Vec2::new(60.137, 20.137);
            let t = match direction {
                GradientDirection::Horizontal => p.x / 100.0,
                GradientDirection::Vertical => p.y / 40.0,
                GradientDirection::Diagonal => (p.x / 100.0 + p.y / 40.0) * 0.5,
            };
            let color = sample(&mesh, p);
            assert!((color[0] - (1.0 - t)).abs() < 1e-4);
            assert!((color[2] - t).abs() < 1e-4);
            assert!((color[3] - 1.0).abs() < 1e-4);
        }
    }
}

#[test]
fn soft_shadow_fades_without_overlapping_alpha_or_invalid_vertices() {
    for scale in [1.0, 1.5, 2.0] {
        let mut mesh = Mesh::default();
        mesh.shape(
            &Shape::Shadow {
                rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 40.0)),
                rounding: CornerRadius::all(5.0),
                shadow: Shadow {
                    offset: Vec2::ZERO,
                    ..Shadow::default()
                },
            },
            scale,
        );
        let alpha = |depth| sample(&mesh, Vec2::new(50.137, depth))[3];
        assert_eq!(alpha(-19.0), 0.0);
        assert!(alpha(-12.137) < alpha(-6.137));
        assert!(alpha(-6.137) < alpha(0.137));
        assert!(alpha(0.137) < alpha(6.137));
        for step in -180..200 {
            assert!(alpha(step as f32 / 10.0 + 0.017) <= 100.0 / 255.0 + 1e-5);
        }
        assert!(mesh.vertices.iter().all(|v| v
            .position
            .iter()
            .chain(&v.color)
            .all(|v| v.is_finite())));
    }
}

fn sample(mesh: &Mesh, point: Vec2) -> [f32; 4] {
    let mut color = [0.0; 4];
    for triangle in mesh.indices.chunks_exact(3) {
        let vertices = [0, 1, 2].map(|i| mesh.vertices[triangle[i] as usize]);
        let [a, b, c] = vertices.map(|v| Vec2::from_array(v.position));
        let area = (b - a).perp_dot(c - a);
        if area.abs() < 1e-8 {
            continue;
        }
        let weights = [
            (b - point).perp_dot(c - point) / area,
            (c - point).perp_dot(a - point) / area,
            (a - point).perp_dot(b - point) / area,
        ];
        if weights.iter().any(|w| *w <= 0.0) {
            continue;
        }
        for (vertex, weight) in vertices.into_iter().zip(weights) {
            for channel in 0..3 {
                color[channel] += vertex.color[channel] * vertex.color[3] * weight;
            }
            color[3] += vertex.color[3] * weight;
        }
    }
    color
}

#[test]
fn translucent_and_subpixel_borders_preserve_coverage_without_overlap() {
    for scale in [1.0, 1.5, 2.0] {
        for width in [0.0, 0.25, 1.0, 24.0] {
            let mut mesh = Mesh::default();
            mesh.shape(
                &Shape::Rect {
                    rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 48.0)),
                    rounding: CornerRadius::all(24.0),
                    fill: Color::rgba(255, 0, 0, 128),
                    border: Border::new(width, Color::rgba(0, 0, 255, 96)),
                },
                scale,
            );
            for step in 0..250 {
                let depth = -1.0 + step as f32 * 0.1 + 0.017;
                let actual = sample(&mesh, Vec2::new(80.137, depth));
                let outer = (0.5 + depth * scale).clamp(0.0, 1.0);
                let inner = if width == 24.0 {
                    0.0
                } else {
                    (0.5 + (depth - width) * scale).clamp(0.0, 1.0)
                };
                let red = inner * 128.0 / 255.0;
                let blue = (outer - inner) * 96.0 / 255.0;
                for (value, expected) in actual.into_iter().zip([red, 0.0, blue, red + blue]) {
                    assert!(
                        (value - expected).abs() < 1e-4,
                        "DPI {scale}, border {width}, depth {depth}: {value} != {expected}"
                    );
                }
            }
        }
    }
    let mut mesh = Mesh::default();
    mesh.shape(
        &Shape::Line {
            start: Vec2::new(0.0, 1.5),
            end: Vec2::new(20.0, 1.5),
            width: 0.25,
            color: Color::WHITE,
        },
        1.0,
    );
    assert!((sample(&mesh, Vec2::new(8.137, 1.517))[3] - 0.25).abs() < 1e-4);
}

#[test]
fn arc_error_stays_below_a_hundredth_of_a_physical_pixel() {
    for radius in [0.5, 3.0, 8.0, 24.0, 100.0, 1024.0, 32768.0] {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let physical_radius = radius * scale + 0.5;
            let segments = arc_segments(physical_radius);
            let angle = std::f64::consts::FRAC_PI_2 / segments as f64;
            let error = f64::from(physical_radius) * (1.0 - (angle * 0.5).cos());
            assert!(error <= 0.01, "radius {radius}, DPI {scale}: {error}");
        }
    }
}

#[test]
fn rounded_and_degenerate_contours_keep_valid_finite_meshes() {
    for scale in [1.0, 1.5, 2.0] {
        for size in [Vec2::new(200.0, 48.0), Vec2::splat(1.0), Vec2::splat(0.25)] {
            for width in [0.0, 0.25, 1.0, 24.0, 100.0] {
                let mut mesh = Mesh::default();
                mesh.shape(
                    &Shape::Rect {
                        rect: Rect::from_min_size(Vec2::ZERO, size),
                        rounding: CornerRadius {
                            top_left: 100.0,
                            top_right: 24.0,
                            bottom_left: 0.0,
                            bottom_right: 12.0,
                        },
                        fill: Color::rgba(200, 50, 20, 128),
                        border: Border::new(width, Color::rgba(20, 80, 230, 96)),
                    },
                    scale,
                );
                assert!(mesh
                    .indices
                    .iter()
                    .all(|i| (*i as usize) < mesh.vertices.len()));
                for vertex in &mesh.vertices {
                    assert!(vertex.position.iter().all(|p| p.is_finite()));
                    assert!(vertex
                        .color
                        .iter()
                        .all(|c| c.is_finite() && (0.0..=1.0).contains(c)));
                    assert!(vertex.color[3] <= 128.0 / 255.0 + 1e-6);
                }
            }
        }
    }
}
