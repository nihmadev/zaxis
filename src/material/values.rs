//! Parameter values of one draw and their packing into the uniform block.

use super::{layout::FRAME_BYTES, MaterialError, MaterialErrorKind, ParamKind, ParamLayout};
use crate::{Color, Vec2};

/// A value of a [`ParamKind`].
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    F32(f32),
    I32(i32),
    U32(u32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    /// Linear, straight alpha.
    Color([f32; 4]),
    /// Columns.
    Mat2([[f32; 2]; 2]),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
    Vec4Array(Vec<[f32; 4]>),
}

impl ParamValue {
    fn matches(&self, kind: ParamKind) -> bool {
        match (self, kind) {
            (Self::F32(_), ParamKind::F32)
            | (Self::I32(_), ParamKind::I32)
            | (Self::U32(_), ParamKind::U32)
            | (Self::Vec2(_), ParamKind::Vec2)
            | (Self::Vec3(_), ParamKind::Vec3)
            | (Self::Vec4(_), ParamKind::Vec4)
            | (Self::Color(_), ParamKind::Color)
            | (Self::Mat2(_), ParamKind::Mat2)
            | (Self::Mat3(_), ParamKind::Mat3)
            | (Self::Mat4(_), ParamKind::Mat4) => true,
            (Self::Vec4Array(values), ParamKind::Vec4Array(n)) => values.len() <= n,
            _ => false,
        }
    }

    /// Append the value as `f32`/`i32`/`u32` words, columns padded to their stride.
    fn write(&self, out: &mut [u8]) {
        let mut at = 0;
        let mut put = |bits: [u8; 4]| {
            out[at..at + 4].copy_from_slice(&bits);
            at += 4;
        };
        let mut floats = |values: &[f32]| values.iter().for_each(|v| put(finite(*v).to_ne_bytes()));
        match self {
            Self::F32(v) => floats(&[*v]),
            Self::I32(v) => put(v.to_ne_bytes()),
            Self::U32(v) => put(v.to_ne_bytes()),
            Self::Mat2([v, _]) if false => floats(v),
            Self::Vec2(v) => floats(v),
            Self::Vec3(v) => floats(v),
            Self::Vec4(v) | Self::Color(v) => floats(v),
            Self::Mat2(m) => m.iter().for_each(|c| floats(c)),
            // Columns of a mat3 are 16 bytes apart: the fourth float of each is padding.
            Self::Mat3(m) => m.iter().for_each(|c| {
                floats(c);
                floats(&[0.0]);
            }),
            Self::Mat4(m) => m.iter().for_each(|c| floats(c)),
            Self::Vec4Array(values) => values.iter().for_each(|v| floats(v)),
        }
    }
}

fn finite(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// The values one draw gives to the parameters of its material, by name.
///
/// A parameter that is not set is zero. Setting a name the material does not declare, or a
/// value of another type, makes the draw fail with a diagnostic and use its fallback;
/// non-finite floats become zero.
///
/// ```
/// use zaxis::{Color, Params, vec2};
/// let params = Params::new().f32("hover", 0.5).vec2("pointer", vec2(0.25, 0.75)).color("tint", Color::WHITE);
/// assert_eq!(params.len(), 3);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    values: Vec<(&'static str, ParamValue)>,
}

macro_rules! setter {
    ($(#[$doc:meta])* $name:ident, $variant:ident, $ty:ty) => {
        $(#[$doc])*
        pub fn $name(self, name: &'static str, value: $ty) -> Self {
            self.set(name, ParamValue::$variant(value))
        }
    };
}

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `name` to `value`, replacing an earlier value of the same name.
    pub fn set(mut self, name: &'static str, value: ParamValue) -> Self {
        match self.values.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 = value,
            None => self.values.push((name, value)),
        }
        self
    }

    setter!(f32, F32, f32);
    setter!(i32, I32, i32);
    setter!(u32, U32, u32);
    setter!(vec3, Vec3, [f32; 3]);
    setter!(vec4, Vec4, [f32; 4]);
    setter!(
        /// Columns of a 2×2 matrix.
        mat2, Mat2, [[f32; 2]; 2]
    );
    setter!(
        /// Columns of a 3×3 matrix.
        mat3, Mat3, [[f32; 3]; 3]
    );
    setter!(
        /// Columns of a 4×4 matrix.
        mat4, Mat4, [[f32; 4]; 4]
    );

    pub fn vec2(self, name: &'static str, value: Vec2) -> Self {
        self.set(name, ParamValue::Vec2(value.to_array()))
    }

    /// A color as the linear, straight-alpha `vec4<f32>` the renderer blends with.
    pub fn color(self, name: &'static str, value: Color) -> Self {
        self.set(name, ParamValue::Color(value.linear()))
    }

    /// At most as many elements as the declared array has; the rest stay zero.
    pub fn array(self, name: &'static str, values: &[[f32; 4]]) -> Self {
        self.set(name, ParamValue::Vec4Array(values.to_vec()))
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The bytes of the parameters laid out as `layout` says, `layout.size()` long.
    pub fn pack(&self, layout: &ParamLayout) -> Result<Vec<u8>, MaterialError> {
        let fail = |message: String| MaterialError::new(MaterialErrorKind::Schema, message);
        let mut bytes = vec![0u8; layout.size()];
        for (name, value) in &self.values {
            let field = layout
                .field(name)
                .ok_or_else(|| fail(format!("the material has no parameter `{name}`")))?;
            if !value.matches(field.kind) {
                return Err(fail(format!(
                    "parameter `{name}` is {:?} but was given {value:?}",
                    field.kind
                )));
            }
            value.write(&mut bytes[field.offset..field.offset + field.kind.size()]);
        }
        Ok(bytes)
    }
}

/// The complete block of a draw: frame data, then `params` (already packed).
pub(crate) fn block(frame: &FrameData, params: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME_BYTES + params.len());
    for v in [frame.size[0], frame.size[1], frame.time, frame.delta]
        .into_iter()
        .chain(frame.texture_rect)
    {
        out.extend_from_slice(&finite(v).to_ne_bytes());
    }
    out.extend_from_slice(params);
    out
}

/// What the library writes in front of the parameters: `MaterialFrame` in the prelude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameData {
    pub size: [f32; 2],
    pub time: f32,
    pub delta: f32,
    pub texture_rect: [f32; 4],
}
