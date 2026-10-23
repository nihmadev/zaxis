// Built-in fragment entry points. Viewport, bindings of groups 0 and 1, VertexOutput and
// vs_main come from common.wgsl, which is concatenated in front of this file.
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

// The same two entry points for payloads whose fourth byte is unused (`PixelFormat::Rgbx8Srgb`,
// `Bgrx8Srgb`): every texel is opaque, so there is no alpha to premultiply or to interpolate.
@fragment
fn fs_main_opaque(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(image, image_sampler, input.uv);
    return vec4<f32>(texel.rgb * input.color.rgb, input.color.a);
}

fn opaque_load(p: vec2<i32>, dimensions: vec2<i32>) -> vec3<f32> {
    return textureLoad(image, clamp(p, vec2<i32>(0), dimensions - vec2<i32>(1)), 0).rgb;
}

@fragment
fn fs_image_linear_opaque(input: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = vec2<i32>(textureDimensions(image));
    let p = input.uv * vec2<f32>(dimensions) - vec2<f32>(0.5);
    let origin = vec2<i32>(floor(p));
    let fraction = fract(p);
    let top = mix(opaque_load(origin, dimensions), opaque_load(origin + vec2<i32>(1,0), dimensions), fraction.x);
    let bottom = mix(opaque_load(origin + vec2<i32>(0,1), dimensions), opaque_load(origin + vec2<i32>(1,1), dimensions), fraction.x);
    return vec4<f32>(mix(top, bottom, fraction.y) * input.color.rgb, input.color.a);
}
