//! The parallax material shared by every card: one WGSL source, one `MaterialId`, one pipeline.
//!
//! The shape is cut into square blocks of `block` physical pixels, snapped to whole pixels at any
//! DPI. Three layers are shifted against the pointer by different amounts: the back layer (the card's
//! texture or a procedural gradient) a little, scattered mid blocks of twice the size more, and a
//! few front blocks of three times the size the most. Everything is evaluated per pixel from
//! uniforms, so moving the pointer rewrites 64 bytes and tessellates nothing.

use zaxis::{Color, Material, ParamKind, Params, Vec2};

const SOURCE: &str = r#"
fn hash(cell: vec2<f32>, seed: f32) -> f32 {
    var q = fract(cell * vec2<f32>(0.1031, 0.1030) + seed * 0.0137);
    q += dot(q, q.yx + 33.33);
    return fract((q.x + q.y) * q.x);
}

// Color of the back layer at pixel `q` (physical, relative to the snapped corner of the card),
// quantized to blocks of `b` pixels. A block takes the texture or gradient at its centre.
fn back_color(in: MaterialInput, p: Params, q: vec2<f32>, b: f32) -> vec3<f32> {
    let cell = floor(q / b);
    let uv = clamp((cell + 0.5) * b / in.size_px, vec2<f32>(0.0), vec2<f32>(1.0));
    let grain = (hash(cell, p.seed) - 0.5) * 0.2 * p.hover;
    let t = clamp(0.65 * uv.x + 0.35 * uv.y + grain, 0.0, 1.0);
    var color = mix(p.tint_a.rgb, p.tint_b.rgb, t);
    if (p.use_texture > 0.5) {
        let texel = widget_color(in.texture_rect.xy + uv * in.texture_rect.zw);
        color = color * (1.0 - texel.a) + texel.rgb;
    }
    return color;
}

fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    let hover = clamp(p.hover, 0.0, 1.0);
    // The pattern is anchored to a whole pixel of the window, so blocks never straddle pixels.
    let local = in.position - round(in.origin);
    // Pixelation grows with the hover through whole-pixel sizes: 1 is the plain image.
    let b = max(1.0, round(mix(1.0, max(p.block, 1.0), hover)));
    let away = -(p.pointer - vec2<f32>(0.5)) * 2.0 * p.depth * hover;
    let wobble = vec2<f32>(sin(in.time * 1.3), cos(in.time * 0.9)) * 5.0 * p.drift * hover;
    let back_shift = round(away * 0.2 + wobble * 0.2);
    let mid_shift = round(away * 0.55 + wobble * 0.55);
    let front_shift = round(away + wobble);

    var color = back_color(in, p, local - back_shift, b) * (1.0 - 0.28 - 0.22 * hover);

    let mid_size = b * 2.0;
    let mid_cell = floor((local - mid_shift) / mid_size);
    if (hash(mid_cell, p.seed + 17.0) > 0.8) {
        let centre = (mid_cell + 0.5) * mid_size + mid_shift;
        let glint = mix(back_color(in, p, centre - back_shift, b), vec3<f32>(1.0), 0.35);
        color = mix(color, glint, 0.6 * hover);
    }

    let front_size = b * 3.0;
    let front_cell = floor((local - front_shift) / front_size);
    if (hash(front_cell, p.seed + 41.0) > 0.93) {
        color = mix(color, mix(p.tint_b.rgb, vec3<f32>(1.0), 0.7), 0.75 * hover);
    }
    return vec4<f32>(color, 1.0);
}
"#;

/// The material, declared once. It reads the card's texture when a draw gives one, and the
/// clock only for draws that opt in, so a card at rest asks for no frames.
pub fn material() -> Material {
    Material::new("pixel parallax", SOURCE)
        .param("pointer", ParamKind::Vec2)
        .param("hover", ParamKind::F32)
        .param("block", ParamKind::F32)
        .param("depth", ParamKind::F32)
        .param("seed", ParamKind::F32)
        .param("use_texture", ParamKind::F32)
        .param("drift", ParamKind::F32)
        .param("tint_a", ParamKind::Color)
        .param("tint_b", ParamKind::Color)
        .reads_texture()
        .animated()
}

/// What a card passes for one draw.
pub struct Look {
    /// Pointer in the card's local UV, 0.5 at rest.
    pub pointer: Vec2,
    /// 0 at rest, 1 under the pointer or keyboard focus.
    pub hover: f32,
    /// Block size in physical pixels at full hover.
    pub block: f32,
    /// Shift of the front layer in physical pixels at full hover.
    pub depth: f32,
    pub seed: f32,
    pub textured: bool,
    pub drift: bool,
    pub tints: [Color; 2],
}

impl Look {
    pub fn params(&self) -> Params {
        Params::new()
            .vec2("pointer", self.pointer)
            .f32("hover", self.hover)
            .f32("block", self.block)
            .f32("depth", self.depth)
            .f32("seed", self.seed)
            .f32("use_texture", f32::from(u8::from(self.textured)))
            .f32("drift", f32::from(u8::from(self.drift)))
            .color("tint_a", self.tints[0])
            .color("tint_b", self.tints[1])
    }
}
