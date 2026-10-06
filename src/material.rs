//! Custom fragment shaders for widgets.
//!
//! A [`Material`] is WGSL for the color stage of a shape, with a declared parameter schema.
//! The library supplies the vertex stage, the vertex layout, clipping, blending and the
//! inputs (local UV, size, scale, window position, time, the widget texture and the
//! backdrop); see `docs/content/docs/materials.mdx` for the shader contract.
//!
//! 1. Register the description once with [`Context::register_material`](crate::Context::register_material)
//!    or [`SharedResources::try_register_material`](crate::SharedResources::try_register_material).
//!    The same source always gives the same [`MaterialId`]; registering again compiles
//!    nothing. Invalid WGSL is reported as a [`MaterialError`] with a line and column in
//!    the author's source, and never panics.
//! 2. Draw with [`Ui::material`](crate::Ui::material), passing [`Params`] by name. Changing
//!    parameters does not tessellate anything: the cached mesh is reused and only the
//!    uniform block of the draw changes.
//!
//! ```
//! use zaxis::{vec2, Color, Context, Material, ParamKind, Params, Rect, Window};
//!
//! let mut context = Context::new();
//! let glow = context.register_material(
//!     &Material::new("glow", r#"
//!         fn material(in: MaterialInput, p: Params) -> vec4<f32> {
//!             let d = distance(in.uv, p.center);
//!             return premultiply(vec4<f32>(p.color.rgb, 1.0) * smoothstep(0.7, 0.0, d));
//!         }
//!     "#)
//!     .param("center", ParamKind::Vec2)
//!     .param("color", ParamKind::Color),
//! );
//! context.run(|context| {
//!     Window::new("Glow").show(context, |ui| {
//!         let rect = Rect::from_min_size(vec2(16.0, 16.0), vec2(120.0, 80.0));
//!         ui.material(rect, glow)
//!             .params(Params::new().vec2("center", vec2(0.5, 0.5)).color("color", Color::rgb(90, 170, 255)))
//!             .corner_radius(12.0)
//!             .show(ui);
//!     });
//! });
//! ```

mod assemble;
mod compile;
mod description;
mod error;
mod layout;
mod registry;
mod values;

pub use description::Material;
pub use error::{MaterialError, MaterialErrorKind};
pub use layout::{ParamKind, ParamLayout, FRAME_BYTES, MAX_PARAM_BYTES, MAX_UNIFORM_BYTES};
pub use registry::MAX_MATERIALS;
pub use values::{ParamValue, Params};

pub(crate) use registry::{MaterialProgram, MaterialRegistry};
pub(crate) use values::{block, FrameData};
