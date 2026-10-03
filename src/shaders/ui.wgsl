struct Viewport {
    size: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
@group(2) @binding(0) var backdrop_image: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(linear, sample) color: vec4<f32>,
    @location(2) screen_uv: vec2<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position.x / viewport.size.x * 2.0 - 1.0, 1.0 - position.y / viewport.size.y * 2.0, 0.0, 1.0);
    output.uv = uv;
    // Interpolate premultiplied colors so transparent contour vertices and
    // translucent border/fill transitions retain their intended coverage.
    output.color = vec4<f32>(color.rgb * color.a, color.a);
    output.screen_uv = position / viewport.size;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(image, image_sampler, input.uv);
    return vec4<f32>(texel.rgb * texel.a * input.color.rgb, texel.a * input.color.a);
}

fn premultiplied_load(p: vec2<i32>, dimensions: vec2<i32>) -> vec4<f32> {
    let c = textureLoad(image, clamp(p, vec2<i32>(0), dimensions - vec2<i32>(1)), 0);
    return vec4<f32>(c.rgb * c.a, c.a);
}

// Straight RGBA is the public payload contract. Interpolate samples after
// premultiplication in linear light, preventing transparent-border dark halos.
@fragment
fn fs_image_linear(input: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(image));
    let p = input.uv * vec2<f32>(dimensions) - vec2<f32>(0.5);
    let origin = vec2<i32>(floor(p));
    let fraction = fract(p);
    let top = mix(premultiplied_load(origin, dimensions), premultiplied_load(origin + vec2<i32>(1,0), dimensions), fraction.x);
    let bottom = mix(premultiplied_load(origin + vec2<i32>(0,1), dimensions), premultiplied_load(origin + vec2<i32>(1,1), dimensions), fraction.x);
    let c = mix(top, bottom, fraction.y);
    return vec4<f32>(c.rgb * input.color.rgb, c.a * input.color.a);
}

@fragment
fn fs_backdrop(input: VertexOutput) -> @location(0) vec4<f32> {
    let original = textureLoad(backdrop_image, vec2<i32>(input.position.xy), 0);
    let blurred = textureSample(image, image_sampler, input.screen_uv);
    return mix(original, blurred, clamp(input.color.a, 0.0, 1.0));
}
