// Cubic B-spline interpolation of `image` at `q`, in texels of that texture (a texel centre
// is at half-integers), through four bilinear fetches. The result is C2-continuous, so
// magnifying a smooth pyramid level shows neither blocks nor creases; the kernel adds a
// variance of 1/3 texel^2, which the level selection accounts for. The sampler clamps, which
// repeats the edge texels at the screen border.
fn spline(q: vec2<f32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(image));
    let x = q - vec2<f32>(0.5);
    let i = floor(x);
    let f = x - i;
    let f2 = f * f;
    let f3 = f2 * f;
    let w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
    let w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
    let w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
    let w3 = f3 / 6.0;
    let g0 = w0 + w1;
    let g1 = w2 + w3;
    let h0 = (i - vec2<f32>(1.0) + w1 / g0 + vec2<f32>(0.5)) / size;
    let h1 = (i + vec2<f32>(1.0) + w3 / g1 + vec2<f32>(0.5)) / size;
    return g0.y * (g0.x * textureSampleLevel(image, image_sampler, vec2<f32>(h0.x, h0.y), 0.0)
                 + g1.x * textureSampleLevel(image, image_sampler, vec2<f32>(h1.x, h0.y), 0.0))
         + g1.y * (g0.x * textureSampleLevel(image, image_sampler, vec2<f32>(h0.x, h1.y), 0.0)
                 + g1.x * textureSampleLevel(image, image_sampler, vec2<f32>(h1.x, h1.y), 0.0));
}
