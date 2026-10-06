// Vertex stage, viewport and widget-texture bindings shared by every pipeline: the built-in
// fragment entry points (ui.wgsl, scroll_hint.wgsl) and user materials (material.wgsl).
// `viewport.scale` is the number of physical pixels per logical pixel.
struct Viewport {
    size: vec2<f32>,
    scale: f32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
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
    // translucent border/fill transitions retain their intended coverage. The vertex
    // w is 1, so the default perspective-correct interpolation is exactly linear, and
    // it is the only interpolation WebGL2 (GLSL ES 3.00) supports.
    output.color = vec4<f32>(color.rgb * color.a, color.a);
    output.screen_uv = position / viewport.size;
    return output;
}
