// Composite of a backdrop blur. common.wgsl and spline.wgsl are concatenated in front of this
// file: the blurred pyramid level is `image` (group 1) and `spline` reads it.
@group(2) @binding(0) var backdrop_image: texture_2d<f32>;

struct Level {
    // level texels per window pixel, 0 = exact texel of an unreduced level, 1 = spline
    level: vec4<f32>,
}
@group(3) @binding(0) var<uniform> effect: Level;

fn backdrop_at(position: vec2<f32>) -> vec4<f32> {
    if (effect.level.y < 0.5) {
        return textureLoad(image, vec2<i32>(floor(position)), 0);
    }
    return spline(position * effect.level.x);
}

@fragment
fn fs_composite(input: VertexOutput) -> @location(0) vec4<f32> {
    let original = textureLoad(backdrop_image, vec2<i32>(input.position.xy), 0);
    let blurred = backdrop_at(input.position.xy);
    return mix(original, blurred, clamp(input.color.a, 0.0, 1.0));
}
