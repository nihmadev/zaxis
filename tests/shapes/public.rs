use zaxis::winit::dpi::PhysicalSize;
use zaxis::{vec2, Border, Color, Context, CornerRadius, Shape, Window};

#[test]
fn asymmetric_rounding_and_thick_borders_keep_fill_inside_the_inner_rect() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut inner = None;
    context.run(|context| {
        Window::new("Shapes").show(context, |ui| {
            let rect = ui.allocate_space(vec2(100.0, 100.0));
            inner = Some(rect.shrink(10.0));
            ui.paint(
                Shape::rect(rect, Color::rgb(200, 0, 0))
                    .corner_radius(CornerRadius {
                        top_left: 100.0,
                        ..CornerRadius::ZERO
                    })
                    .border(Border::new(10.0, Color::WHITE)),
            );
        });
    });
    let inner = inner.unwrap();
    let fill_vertices: Vec<_> = context
        .draw_data()
        .vertices
        .iter()
        .filter(|v| v.color[0] > 0.0 && v.color[1] == 0.0 && v.color[2] == 0.0)
        .collect();
    assert!(!fill_vertices.is_empty());
    for vertex in fill_vertices {
        assert!(
            vertex.position[0] >= inner.min.x - 0.001 && vertex.position[0] <= inner.max.x + 0.001
        );
        assert!(
            vertex.position[1] >= inner.min.y - 0.001 && vertex.position[1] <= inner.max.y + 0.001
        );
    }
}
