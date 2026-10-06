//! Custom materials through the public API: registration, parameters, draw commands and repaint.

mod animated;
mod draw;
mod params;
mod registry;

use crate::prelude::*;
use winit::dpi::PhysicalSize;

/// A shader with two parameters, used by most tests.
pub const GLOW: &str = r#"
    fn material(in: MaterialInput, p: Params) -> vec4<f32> {
        let d = distance(in.uv, p.center);
        return premultiply(vec4<f32>(p.color.rgb, 1.0) * smoothstep(0.7, 0.0, d));
    }
"#;

pub fn glow() -> Material {
    Material::new("glow", GLOW)
        .param("center", ParamKind::Vec2)
        .param("color", ParamKind::Color)
}

pub fn setup(scale: f64) -> Context {
    let mut c = Context::new();
    c.set_viewport(
        PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
        scale,
    );
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    c
}

pub fn rect() -> Rect {
    Rect::from_min_size(vec2(20.0, 20.0), vec2(120.0, 80.0))
}

pub fn params(x: f32) -> Params {
    Params::new()
        .vec2("center", vec2(x, 0.5))
        .color("color", Color::rgb(90, 170, 255))
}

/// One pass at `now` with `build` inside a window.
pub fn frame(c: &mut Context, now: Instant, build: impl FnOnce(&mut Ui<'_>)) {
    c.run_at(now, |c| {
        Window::new("Materials")
            .default_position(vec2(0.0, 0.0))
            .default_size(vec2(600.0, 500.0))
            .show(c, build);
    });
}

pub fn material_commands(c: &mut Context) -> Vec<DrawCommand> {
    c.draw_data()
        .commands
        .iter()
        .filter(|command| command.material.is_some())
        .cloned()
        .collect()
}
