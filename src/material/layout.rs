//! Parameter schema and its placement in the uniform block.
//!
//! The block a draw binds is `MaterialFrame` (32 bytes, written by the library) followed by
//! the generated `Params` struct. Offsets follow the WGSL rules for the uniform address
//! space, which for these types are the std140 rules, except that `mat2x2<f32>` is 8-byte
//! aligned: a `vec3` takes 12 bytes and is 16-aligned, a scalar may sit in the tail of the
//! `vec3` before it, and arrays are arrays of `vec4`.

use super::{MaterialError, MaterialErrorKind};

/// Bytes the library writes in front of the parameters of every draw.
pub const FRAME_BYTES: usize = 32;

/// Largest uniform block of one draw, library data and parameters together. Chosen far
/// below the 16 KiB every backend including WebGL2 guarantees, so a dynamic-offset binding
/// of this size is valid everywhere and a draw costs one aligned slot in the frame buffer.
pub const MAX_UNIFORM_BYTES: usize = 1024;

/// Bytes available to the parameters of one material.
pub const MAX_PARAM_BYTES: usize = MAX_UNIFORM_BYTES - FRAME_BYTES;

/// Type of a material parameter, and the type it has in the generated `Params` struct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParamKind {
    /// `f32`
    F32,
    /// `i32`
    I32,
    /// `u32`
    U32,
    /// `vec2<f32>`
    Vec2,
    /// `vec3<f32>`: 12 bytes, 16-aligned.
    Vec3,
    /// `vec4<f32>`
    Vec4,
    /// `vec4<f32>` holding the linear, straight-alpha color of a [`Color`](crate::Color).
    Color,
    /// `mat2x2<f32>`, column major.
    Mat2,
    /// `mat3x3<f32>`, column major, columns 16 bytes apart.
    Mat3,
    /// `mat4x4<f32>`, column major.
    Mat4,
    /// `array<vec4<f32>, N>` with `N` at least one.
    Vec4Array(usize),
}

impl ParamKind {
    pub(crate) fn align(self) -> usize {
        match self {
            Self::F32 | Self::I32 | Self::U32 => 4,
            Self::Vec2 | Self::Mat2 => 8,
            _ => 16,
        }
    }

    pub(crate) fn size(self) -> usize {
        match self {
            Self::F32 | Self::I32 | Self::U32 => 4,
            Self::Vec2 => 8,
            Self::Vec3 => 12,
            Self::Vec4 | Self::Color | Self::Mat2 => 16,
            Self::Mat3 => 48,
            Self::Mat4 => 64,
            Self::Vec4Array(n) => n.saturating_mul(16),
        }
    }

    pub(crate) fn wgsl(self) -> String {
        match self {
            Self::F32 => "f32".into(),
            Self::I32 => "i32".into(),
            Self::U32 => "u32".into(),
            Self::Vec2 => "vec2<f32>".into(),
            Self::Vec3 => "vec3<f32>".into(),
            Self::Vec4 | Self::Color => "vec4<f32>".into(),
            Self::Mat2 => "mat2x2<f32>".into(),
            Self::Mat3 => "mat3x3<f32>".into(),
            Self::Mat4 => "mat4x4<f32>".into(),
            Self::Vec4Array(n) => format!("array<vec4<f32>, {n}>"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParamField {
    pub name: String,
    pub kind: ParamKind,
    /// Offset inside the parameters, not counting the library's frame data.
    pub offset: usize,
}

/// Placement of the declared parameters.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ParamLayout {
    pub(crate) fields: Vec<ParamField>,
    size: usize,
}

impl ParamLayout {
    /// Place `params` in declaration order. Names must be WGSL identifiers that do not start
    /// with `z_`, be unique and fit [`MAX_PARAM_BYTES`] together.
    pub fn new(params: &[(String, ParamKind)]) -> Result<Self, MaterialError> {
        let fail = |message: String| MaterialError::new(MaterialErrorKind::Schema, message);
        let mut fields: Vec<ParamField> = Vec::with_capacity(params.len());
        let mut end: usize = 0;
        for (name, kind) in params {
            if !is_identifier(name) || name.starts_with("z_") {
                return Err(fail(format!(
                    "parameter name `{name}` must be a WGSL identifier not starting with `z_`"
                )));
            }
            if fields.iter().any(|f| f.name == *name) {
                return Err(fail(format!("parameter `{name}` is declared twice")));
            }
            if *kind == ParamKind::Vec4Array(0) {
                return Err(fail(format!("array parameter `{name}` has no elements")));
            }
            let offset = end.next_multiple_of(kind.align());
            end = offset.saturating_add(kind.size());
            if end > MAX_PARAM_BYTES {
                return Err(fail(format!(
                    "parameters need more than {MAX_PARAM_BYTES} bytes at `{name}` \
                     (a draw's uniform block is limited to {MAX_UNIFORM_BYTES} bytes, {FRAME_BYTES} of them used by the library)"
                )));
            }
            fields.push(ParamField {
                name: name.clone(),
                kind: *kind,
                offset,
            });
        }
        Ok(Self {
            fields,
            size: end.next_multiple_of(16),
        })
    }

    /// Bytes the parameters occupy, a multiple of 16.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Bytes of the whole uniform block of a draw: frame data and parameters.
    pub fn block_size(&self) -> usize {
        FRAME_BYTES + self.size
    }

    /// Offset of the parameter `name` inside the parameters.
    pub fn offset_of(&self, name: &str) -> Option<usize> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.offset)
    }

    pub(crate) fn field(&self, name: &str) -> Option<&ParamField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name != "_"
        && !name.starts_with("__")
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
