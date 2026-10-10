// Prelude of every user material. The library assembles one module from
//   common.wgsl      viewport, widget texture (group 1), VertexOutput, vs_main
//   material.wgsl    this file: backdrop (group 2), the per-draw block (group 3), helpers
//   <Params struct>  generated from the material's parameter schema
//   <user source>    helper functions and `fn material(in: MaterialInput, p: Params) -> vec4<f32>`
// The binding set is fixed; a material cannot declare resources of its own.
// Names starting with `z_` and the names below are reserved.

@group(2) @binding(0) var z_backdrop_sharp: texture_2d<f32>;
@group(2) @binding(1) var z_backdrop_blurred: texture_2d<f32>;
@group(2) @binding(2) var z_backdrop_sampler: sampler;

// Written by the library for every draw, in front of the material's parameters.
struct MaterialFrame {
    size: vec2<f32>,
    time: f32,
    delta: f32,
    // Origin (xy) and size (zw) of the displayed part of the widget texture, in 0..1.
    texture_rect: vec4<f32>,
}

struct MaterialBlock {
    frame: MaterialFrame,
    params: Params,
}

@group(3) @binding(0) var<uniform> z_block: MaterialBlock;

struct MaterialInput {
    // Position inside the shape: (0, 0) top-left, (1, 1) bottom-right. Mesh contour
    // vertices outside the shape clamp to the nearest edge.
    uv: vec2<f32>,
    // Size of the shape in logical pixels, after visual scale, and in physical pixels.
    size: vec2<f32>,
    size_px: vec2<f32>,
    // Physical pixels per logical pixel of this window.
    scale: f32,
    // Center of this fragment in the window, physical pixels, origin top-left.
    position: vec2<f32>,
    // Top-left corner of the shape in the window, physical pixels: `position - uv * size_px`.
    // Exact inside the shape, up to half a pixel off on its antialiased contour; round it to
    // snap a pattern to the pixel grid.
    origin: vec2<f32>,
    // The same point as a fraction of the window, for sampling the backdrop.
    screen_uv: vec2<f32>,
    // Vertex color exactly as vs_main interpolates it: premultiplied, including coverage
    // and opacity. The library multiplies the result of `material` by `color.a`, so a
    // material normally ignores it; `tint` is the straight color the caller asked for.
    color: vec4<f32>,
    tint: vec3<f32>,
    // Seconds on the pass clock and since the previous pass; zero unless the material is
    // declared animated.
    time: f32,
    delta: f32,
    texture_rect: vec4<f32>,
}

// Premultiplied color of the widget texture at `uv` (0..1 over the whole texture), filtered
// by the sampler the texture was bound with. Sampling is level 0 and safe in any control flow.
fn widget_color(uv: vec2<f32>) -> vec4<f32> {
    let c = textureSampleLevel(image, image_sampler, uv, 0.0);
    return vec4<f32>(c.rgb * c.a, c.a);
}

// Texture coordinate of this fragment inside the displayed part of the widget texture.
fn widget_uv(in: MaterialInput) -> vec2<f32> {
    return in.texture_rect.xy + in.uv * in.texture_rect.zw;
}

// Premultiplied texel `p`, clamped to the texture, without filtering.
fn widget_texel(p: vec2<i32>) -> vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(image));
    let c = textureLoad(image, clamp(p, vec2<i32>(0), dimensions - vec2<i32>(1)), 0);
    return vec4<f32>(c.rgb * c.a, c.a);
}

fn widget_size() -> vec2<f32> {
    return vec2<f32>(textureDimensions(image));
}

// What was drawn behind the shape, premultiplied. Meaningful only for materials declared
// with `reads_backdrop`; any other material sees transparent black.
fn backdrop_sharp(in: MaterialInput) -> vec4<f32> {
    return textureLoad(z_backdrop_sharp, vec2<i32>(in.position), 0);
}

fn backdrop_blurred(in: MaterialInput) -> vec4<f32> {
    return textureSampleLevel(z_backdrop_blurred, z_backdrop_sampler, in.screen_uv, 0.0);
}

fn premultiply(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(c.rgb * c.a, c.a);
}

@fragment
fn fs_material(input: VertexOutput) -> @location(0) vec4<f32> {
    let frame = z_block.frame;
    var m: MaterialInput;
    m.uv = input.uv;
    m.size = frame.size;
    m.scale = viewport.scale;
    m.size_px = frame.size * viewport.scale;
    m.position = input.position.xy - viewport.origin;
    m.origin = m.position - input.uv * m.size_px;
    m.screen_uv = input.screen_uv;
    m.color = input.color;
    m.tint = input.color.rgb / max(input.color.a, 0.00001);
    m.time = frame.time;
    m.delta = frame.delta;
    m.texture_rect = frame.texture_rect;
    // The returned color is premultiplied and fully covered; coverage (contour
    // antialiasing, rounding, opacity) scales it once, so blending never counts alpha twice.
    return material(m, z_block.params) * input.color.a;
}
