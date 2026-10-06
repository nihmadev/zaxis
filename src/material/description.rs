//! The description of a material: source, parameter schema and flags.

use super::ParamKind;
use std::sync::Arc;

/// A user fragment shader and the parameters it takes.
///
/// `fragment` is WGSL: any helper functions and constants, and one function
/// `fn material(in: MaterialInput, p: Params) -> vec4<f32>` that returns the premultiplied
/// color of a fully covered pixel. `Params` is generated from the schema declared with
/// [`param`](Self::param); `MaterialInput` and the helpers (`widget_color`,
/// `backdrop_blurred`, ...) come from the library prelude documented in
/// `docs/content/docs/materials.mdx`. Register a description with
/// [`Context::register_material`](crate::Context::register_material) and draw with
/// [`Ui::material`](crate::Ui::material).
///
/// ```
/// use zaxis::{Material, ParamKind};
///
/// let material = Material::new("pulse", r#"
///     fn material(in: MaterialInput, p: Params) -> vec4<f32> {
///         let wave = 0.5 + 0.5 * sin(in.time * 2.0 + in.uv.x * 6.0);
///         return premultiply(vec4<f32>(mix(p.low.rgb, p.high.rgb, wave), 1.0));
///     }
/// "#)
/// .param("low", ParamKind::Color)
/// .param("high", ParamKind::Color)
/// .animated();
/// assert_eq!(material.label(), "pulse");
/// ```
///
/// A shader that loops forever hangs the GPU for as long as the driver allows; keeping
/// loops bounded is the author's responsibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Material {
    pub(crate) label: Arc<str>,
    pub(crate) fragment: String,
    pub(crate) params: Vec<(String, ParamKind)>,
    pub(crate) reads_texture: bool,
    pub(crate) reads_backdrop: bool,
    pub(crate) animated: bool,
}

impl Material {
    /// A material called `label` (shown in errors and diagnostics only) with the WGSL
    /// `fragment` source. Without further calls it takes no parameters, ignores the widget
    /// texture and the backdrop, and does not depend on time.
    pub fn new(label: impl Into<Arc<str>>, fragment: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            fragment: fragment.into(),
            params: Vec::new(),
            reads_texture: false,
            reads_backdrop: false,
            animated: false,
        }
    }

    /// Declare a parameter. Declaration order is the order of `Params` members.
    pub fn param(mut self, name: impl Into<String>, kind: ParamKind) -> Self {
        self.params.push((name.into(), kind));
        self
    }

    /// The material samples the widget texture (an image given to the draw).
    pub fn reads_texture(mut self) -> Self {
        self.reads_texture = true;
        self
    }

    /// The material reads what was drawn behind it with `backdrop_sharp` or
    /// `backdrop_blurred`. Its draws go through the backdrop path of blur effects: they
    /// cost a copy and a blur of the area behind, and are never merged with other draws.
    pub fn reads_backdrop(mut self) -> Self {
        self.reads_backdrop = true;
        self
    }

    /// The result depends on `in.time`. While a draw of it is visible the window keeps
    /// asking for frames; without this the time inputs are zero and a still window stays
    /// still.
    pub fn animated(mut self) -> Self {
        self.animated = true;
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn fragment(&self) -> &str {
        &self.fragment
    }
    pub fn params(&self) -> &[(String, ParamKind)] {
        &self.params
    }

    /// What identifies the shader: everything except the label.
    pub(crate) fn content_key(&self) -> String {
        let mut key = String::with_capacity(self.fragment.len() + 64);
        key.push_str(&self.fragment);
        key.push('\u{0}');
        for (name, kind) in &self.params {
            key.push_str(name);
            key.push(':');
            key.push_str(&format!("{kind:?}"));
            key.push(';');
        }
        key.push_str(&format!(
            "\u{0}{}{}{}",
            u8::from(self.reads_texture),
            u8::from(self.reads_backdrop),
            u8::from(self.animated)
        ));
        key
    }
}
