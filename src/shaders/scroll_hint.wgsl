// Procedural scroll-edge shadow. Shared VertexOutput/vs_main come from ui.wgsl.
// The hint is a single quad, with UV.y increasing towards the viewport edge.
// Smootherstep gives both ends zero slope: no bright strip or hard leading seam.
@fragment
fn fs_scroll_hint(input: VertexOutput) -> @location(0) vec4<f32> {
    let t = clamp(input.uv.y, 0.0, 1.0);
    let fade = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    return input.color * fade;
}
