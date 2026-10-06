//! A material module must be accepted by every backend's shader translation, not only by
//! the WGSL front end: WebGL2 consumes GLSL ES 3.00, WebGPU the WGSL itself. Runs without a
//! graphics adapter.

use zaxis::wgpu::naga::{
    back::glsl::{self, PipelineOptions, Version, WriterFlags},
    front::wgsl,
    proc::BoundsCheckPolicies,
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage,
};
use zaxis::{vec2, Context, Material, MaterialSource, ParamKind, Rect, Window};

const PLAIN: &str = "fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    return premultiply(vec4<f32>(in.uv, p.amount, 1.0)) * in.size.x;
}";
const IMAGE: &str = "fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    let uv = widget_uv(in);
    let block = floor(uv * widget_size() / 4.0) * 4.0 / widget_size();
    return widget_color(block) + widget_texel(vec2<i32>(1, 1)) * p.amount;
}";
const BACKDROP: &str = "fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    let glass = mix(backdrop_sharp(in), backdrop_blurred(in), p.amount);
    return glass * 0.9 + vec4<f32>(in.tint, 1.0) * 0.1;
}";

fn sources() -> Vec<MaterialSource> {
    let mut context = Context::new();
    context.set_viewport(zaxis::winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let materials = [
        Material::new("plain", PLAIN).param("amount", ParamKind::F32),
        Material::new("image", IMAGE)
            .param("amount", ParamKind::F32)
            .reads_texture(),
        Material::new("backdrop", BACKDROP)
            .param("amount", ParamKind::F32)
            .reads_backdrop(),
    ];
    let ids: Vec<_> = materials
        .iter()
        .map(|m| context.register_material(m))
        .collect();
    context.run(|context| {
        Window::new("w").show(context, |ui| {
            for (n, id) in ids.iter().enumerate() {
                let rect =
                    Rect::from_min_size(vec2(10.0 + 70.0 * n as f32, 40.0), vec2(60.0, 60.0));
                ui.material(rect, *id).show(ui);
            }
        });
    });
    let sources = context.draw_data().materials.clone();
    assert_eq!(sources.len(), 3);
    sources
}

#[test]
fn every_material_module_validates_and_translates_to_webgl2_glsl() {
    for source in sources() {
        let module = wgsl::parse_str(&source.wgsl)
            .unwrap_or_else(|e| panic!("{}: {}", source.label, e.emit_to_string(&source.wgsl)));
        let info = Validator::new(ValidationFlags::all(), Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{}: {e:?}", source.label));
        let options = glsl::Options {
            version: Version::Embedded {
                version: 300,
                is_webgl: true,
            },
            writer_flags: WriterFlags::ADJUST_COORDINATE_SPACE,
            ..Default::default()
        };
        for (stage, entry) in [
            (ShaderStage::Vertex, "vs_main"),
            (ShaderStage::Fragment, "fs_material"),
        ] {
            let pipeline = PipelineOptions {
                shader_stage: stage,
                entry_point: entry.to_owned(),
                multiview: None,
            };
            let mut text = String::new();
            let mut writer = glsl::Writer::new(
                &mut text,
                &module,
                &info,
                &options,
                &pipeline,
                BoundsCheckPolicies::default(),
            )
            .unwrap_or_else(|e| panic!("{} {entry}: {e}", source.label));
            writer
                .write()
                .unwrap_or_else(|e| panic!("{} {entry}: {e}", source.label));
            assert!(text.starts_with("#version 300 es"), "{}", source.label);
            // One uniform block with the frame data and parameters, far below the 16 KiB
            // every WebGL2 context offers, and no feature GLSL ES 3.00 lacks.
            assert!(!text.contains("imageLoad") && !text.contains("textureGather"));
        }
    }
}

#[test]
fn the_block_binding_is_fixed_for_every_material() {
    for source in sources() {
        let module = wgsl::parse_str(&source.wgsl).unwrap();
        let mut bindings: Vec<_> = module
            .global_variables
            .iter()
            .filter_map(|(_, v)| v.binding.as_ref().map(|b| (b.group, b.binding)))
            .collect();
        bindings.sort_unstable();
        assert_eq!(
            bindings,
            [(0, 0), (1, 0), (1, 1), (2, 0), (2, 1), (2, 2), (3, 0)],
            "{}",
            source.label
        );
    }
}
