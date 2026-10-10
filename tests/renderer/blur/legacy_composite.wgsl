// The composite of the pre-pyramid backdrop blur, kept so tests can measure the baseline.
// common.wgsl is concatenated in front of this file.
@group(2) @binding(0) var backdrop_image: texture_2d<f32>;

@fragment
fn fs_backdrop(input: VertexOutput) -> @location(0) vec4<f32> {
    let original = textureLoad(backdrop_image, vec2<i32>(input.position.xy), 0);
    let blurred = textureSample(image, image_sampler, input.screen_uv);
    return mix(original, blurred, clamp(input.color.a, 0.0, 1.0));
}
