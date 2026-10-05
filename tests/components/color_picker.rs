use zaxis::components::color_picker::{from_hsv, to_hsv};
use zaxis::Color;

#[test]
fn hsv_roundtrips_srgb_and_preserves_alpha() {
    for r in (0..=255).step_by(17) {
        for g in (0..=255).step_by(17) {
            for b in (0..=255).step_by(17) {
                let color = Color::rgba(r, g, b, 73);
                assert_eq!(from_hsv(to_hsv(color), 73), color);
            }
        }
    }
    assert_eq!(from_hsv([1.0, 1.0, 1.0], 255), Color::rgb(255, 0, 0));
}
