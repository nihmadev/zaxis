//! The host's own scene (a triangle and a grid) and the interfaces drawn over it.

use super::harness::{Host, Spec, DEPTH};
use zaxis::{
    vec2, Blur, Border, Button, Checkbox, Color, Context, CornerRadius, DrawData, Rect, Shape,
    Slider,
};

const SHADER: &str = "
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec3<f32> }
@vertex fn triangle(@builtin(vertex_index) i: u32) -> Out {
    var p = array<vec2<f32>, 3>(vec2(-0.7, -0.7), vec2(0.7, -0.5), vec2(-0.1, 0.8));
    var c = array<vec3<f32>, 3>(vec3(1.0, 0.1, 0.1), vec3(0.1, 1.0, 0.1), vec3(0.2, 0.3, 1.0));
    return Out(vec4(p[i], 0.5, 1.0), c[i]);
}
@vertex fn grid(@builtin(vertex_index) i: u32) -> Out {
    let line = f32(i / 2u) / 4.0 * 2.0 - 1.0;
    let end = f32(i % 2u) * 2.0 - 1.0;
    var p = vec2(line, end);
    if (i >= 10u) { p = vec2(end, line - 2.5); }
    return Out(vec4(p, 0.4, 1.0), vec3(0.8, 0.8, 0.2));
}
@fragment fn paint(in: Out) -> @location(0) vec4<f32> { return vec4(in.color, 1.0); }
";

/// Draw the host's scene into a pass with the attachments of `spec`.
pub fn host_scene(host: &Host, pass: &mut wgpu::RenderPass<'_>, spec: &Spec) {
    let module = host
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
    let make = |vertex: &str, topology| {
        host.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("paint"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: spec.view_format(),
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: spec.depth.then(|| wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: spec.samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            })
    };
    pass.set_pipeline(&make("grid", wgpu::PrimitiveTopology::LineList));
    pass.draw(0..20, 0..1);
    pass.set_pipeline(&make("triangle", wgpu::PrimitiveTopology::TriangleList));
    pass.draw(0..3, 0..1);
}

/// A context for a surface of `size` physical pixels at `scale`.
pub fn context(size: [u32; 2], scale: f32) -> Context {
    sharing(&zaxis::SharedResources::new(), size, scale)
}

/// Like [`context`], on the glyph atlas and image cache of `resources`.
pub fn sharing(resources: &zaxis::SharedResources, size: [u32; 2], scale: f32) -> Context {
    let mut context = Context::with_shared(resources);
    context.set_viewport(
        zaxis::winit::dpi::PhysicalSize::new(size[0], size[1]),
        f64::from(scale),
    );
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    context
}

/// Text, a button, a slider, a checkbox and a translucent rounded card.
pub fn controls(size: [u32; 2], scale: f32) -> DrawData {
    controls_in(&mut see_through(size, scale), "Embedded interface")
}

/// The same controls in `context`, headed by `title`.
pub fn controls_in(context: &mut Context, title: &str) -> DrawData {
    let mut value = 0.4;
    let mut checked = true;
    bare(context, &mut |ui| {
        ui.label(title);
        ui.add(Button::new("Button"));
        ui.add(Slider::new(&mut value, 0.0..=1.0).width(120.0));
        ui.add(Checkbox::new(&mut checked, "Checkbox"));
        let card = ui.allocate_space(vec2(110.0, 40.0));
        ui.paint(Shape::Rect {
            rect: card,
            rounding: CornerRadius::all(10.0),
            fill: Color::rgba(40, 160, 255, 140),
            border: Border::NONE,
        });
    })
}

/// A context whose windows have no fill, so that what the host drew shows through.
pub fn see_through(size: [u32; 2], scale: f32) -> Context {
    let mut context = context(size, scale);
    let mut style = context.style().clone();
    style.window_fill = Color::TRANSPARENT;
    context.set_style(style);
    context
}

/// `build` in the root area of the context, which covers the surface.
pub fn bare(context: &mut Context, build: &mut dyn FnMut(&mut zaxis::Ui<'_>)) -> DrawData {
    for _ in 0..2 {
        context.run(|context| {
            zaxis::Root::new().show(context, &mut *build);
        });
    }
    copy(context.draw_data())
}

/// A blurred panel over `rect` (logical pixels), nothing else.
pub fn blur_panel(size: [u32; 2], scale: f32, rect: Rect, radius: f32) -> DrawData {
    let mut context = see_through(size, scale);
    bare(&mut context, &mut |ui| {
        Blur::new(rect).radius(radius).show(ui);
    })
}

/// A copy of the frame, to present while the context goes on.
pub fn copy(data: &DrawData) -> DrawData {
    DrawData {
        vertices: data.vertices.clone(),
        indices: data.indices.clone(),
        commands: data.commands.clone(),
        logical_size: data.logical_size,
        scale_factor: data.scale_factor,
        revision: data.revision,
        source: data.source,
        textures: data.textures.clone(),
        texture_options: data.texture_options.clone(),
        materials: data.materials.clone(),
        material_uniforms: data.material_uniforms.clone(),
        ..DrawData::new(data.logical_size, data.scale_factor)
    }
}
