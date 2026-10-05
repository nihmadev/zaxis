use winit::dpi::PhysicalSize;
use zaxis::renderer::viewport::scissor;
use zaxis::vec2;
use zaxis::Rect;

#[test]
fn fractional_clips_never_include_pixels_outside_the_panel() {
    let viewport = PhysicalSize::new(800, 600);
    let rect = Rect::from_min_max(vec2(10.25, 20.5), vec2(50.75, 60.25));
    assert_eq!(scissor(rect, 1.25, viewport), Some([13, 26, 50, 49]));
    let clipped = Rect::from_min_max(vec2(-5.0, -8.0), vec2(900.0, 700.0));
    assert_eq!(scissor(clipped, 1.25, viewport), Some([0, 0, 800, 600]));
    let thin = Rect::from_min_max(vec2(10.25, 0.0), vec2(10.5, 5.0));
    assert_eq!(scissor(thin, 1.25, viewport), None);
}
