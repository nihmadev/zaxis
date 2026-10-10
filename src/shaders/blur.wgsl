// Fullscreen filter passes of the backdrop effect. Everything works on premultiplied linear
// light and addresses texels by their window position, never by interpolated uv, so the
// sample grid is fixed to the screen and a panel that moves cannot make its backdrop shimmer.
//
//   fs_down      one pyramid step: a 6-tap binomial (1 5 10 10 5 1) / 32 per axis, centred
//                between source texels, as three bilinear taps per axis
//   fs_gaussian  an exact separable Gaussian through bilinear pair taps; radius <= 16
//   fs_resolve   cubic B-spline upsampling of a pyramid level to full resolution
//   fs_copy      the canvas to the surface
// spline.wgsl is concatenated after this file.

@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

struct Filter {
    // Gaussian: one texel along the axis, as a fraction of the texture.
    step: vec2<f32>,
    sigma: f32,
    radius: f32,
    // Resolve: level texels per window pixel (2^-level).
    inverse_scale: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
}
@group(1) @binding(0) var<uniform> params: Filter;

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
    return textureSampleLevel(image, image_sampler, input.uv, 0.0);
}

@fragment
fn fs_down(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(image));
    // The centre of destination texel i is the boundary of source texels 2i and 2i + 1.
    let centre = position.xy * 2.0;
    var color = vec4<f32>(0.0);
    for (var y = 0; y < 3; y += 1) {
        let wy = select(0.625, 0.1875, y != 1);
        let oy = (f32(y) - 1.0) * 1.6666667;
        for (var x = 0; x < 3; x += 1) {
            let wx = select(0.625, 0.1875, x != 1);
            let ox = (f32(x) - 1.0) * 1.6666667;
            color += textureSampleLevel(image, image_sampler, (centre + vec2<f32>(ox, oy)) / size, 0.0) * (wx * wy);
        }
    }
    return color;
}

@fragment
fn fs_gaussian(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = position.xy / vec2<f32>(textureDimensions(image));
    let s = max(params.sigma, 0.0001);
    let radius = i32(params.radius);
    var color = textureSampleLevel(image, image_sampler, uv, 0.0);
    var total = 1.0;
    // Taps i and i + 1 merge into one bilinear fetch between them: (radius + 1) / 2 fetches
    // per side with exactly the weights of the full kernel.
    for (var i = 1; i <= radius; i += 2) {
        let w1 = exp(-0.5 * f32(i * i) / (s * s));
        var w2 = 0.0;
        if (i + 1 <= radius) {
            w2 = exp(-0.5 * f32((i + 1) * (i + 1)) / (s * s));
        }
        let w = w1 + w2;
        let offset = params.step * ((f32(i) * w1 + f32(i + 1) * w2) / w);
        color += (textureSampleLevel(image, image_sampler, uv + offset, 0.0)
            + textureSampleLevel(image, image_sampler, uv - offset, 0.0)) * w;
        total += 2.0 * w;
    }
    return color / total;
}

@fragment
fn fs_resolve(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // An unreduced level is already at window resolution: copy it exactly.
    if (params.inverse_scale >= 1.0) {
        return textureLoad(image, vec2<i32>(position.xy), 0);
    }
    return spline(position.xy * params.inverse_scale);
}
