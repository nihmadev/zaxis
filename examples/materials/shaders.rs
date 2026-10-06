//! The two materials of the example, plus one that does not compile.

use zaxis::{Material, ParamKind};

/// A drifting two-colour gradient with soft noise. With an image it tints the picture instead.
/// `amount` mixes the effect in; the clock only matters for draws that opt in.
pub fn aurora() -> Material {
    Material::new(
        "aurora",
        r#"
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
               mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    let flow = vec2<f32>(in.time * p.speed, in.time * p.speed * 0.6);
    let aspect = in.size.x / in.size.y;
    let n = noise(vec2<f32>(in.uv.x * aspect, in.uv.y) * 3.0 + flow);
    let t = clamp(in.uv.x * 0.6 + in.uv.y * 0.4 + (n - 0.5) * 0.6 * p.amount, 0.0, 1.0);
    var color = mix(p.a.rgb, p.b.rgb, t);
    if (p.use_image > 0.5) {
        let picture = widget_color(widget_uv(in));
        let tone = dot(picture.rgb, vec3<f32>(0.299, 0.587, 0.114));
        color = mix(picture.rgb, mix(p.a.rgb, p.b.rgb, tone), p.amount);
    }
    return vec4<f32>(color, 1.0);
}
"#,
    )
    .param("a", ParamKind::Color)
    .param("b", ParamKind::Color)
    .param("amount", ParamKind::F32)
    .param("speed", ParamKind::F32)
    .param("use_image", ParamKind::F32)
    .reads_texture()
    .animated()
}

/// Frosted glass: the blurred backdrop, lightened and tinted, with a sheen along the top edge.
/// It reads what was drawn behind it, so it goes through the backdrop path of blur effects.
pub fn frost() -> Material {
    Material::new(
        "frost",
        r#"
fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    let glass = backdrop_blurred(in);
    let sheen = (1.0 - in.uv.y) * 0.12 * p.strength;
    let tinted = mix(glass.rgb, p.tint.rgb, 0.25 * p.strength) + vec3<f32>(sheen);
    return vec4<f32>(tinted, 1.0);
}
"#,
    )
    .param("tint", ParamKind::Color)
    .param("strength", ParamKind::F32)
    .reads_backdrop()
}

/// A shader with a typo, to show that a rejected material is an error value, not a crash.
pub fn broken() -> Material {
    Material::new(
        "broken",
        "fn material(in: MaterialInput, p: Params) -> vec4<f32> {\n    let c = vec3<f32>(1.0, 0.0, 0.0;\n    return vec4<f32>(c, 1.0);\n}\n",
    )
}
