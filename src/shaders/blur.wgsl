@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@group(1) @binding(0) var<uniform> step: vec4<f32>;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Output {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var output: Output;
    output.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    output.uv = uv;
    return output;
}

@fragment
fn fs_copy(input: Output) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, input.uv);
}

@fragment
fn fs_blur(input: Output) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0.0);
    var total = 0.0;
    // A separable Gaussian truncated at three sigma. Sampling occurs in linear RGB.
    let samples = i32(step.z);
    for (var i = -samples; i <= samples; i += 1) {
        let distance = f32(i) * 3.0 / f32(samples);
        let weight = exp(-0.5 * distance * distance);
        color += textureSample(image, image_sampler, input.uv + step.xy * distance) * weight;
        total += weight;
    }
    return color / total;
}
