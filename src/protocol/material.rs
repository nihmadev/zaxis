//! Material references in draw data: what a command draws with and where its uniforms are.
//!
//! A material replaces the color stage of the fragment shader. The geometry, vertex layout,
//! clip rectangle and blending of a command that uses one are those of any other command.

use std::{ops::Range, sync::Arc};

/// Stable key of a registered material: a hash of its source, parameter schema and flags.
///
/// The same description always gets the same id, so registering it every frame is cheap and
/// grows nothing. Ids are only meaningful with the [`SharedResources`](crate::SharedResources)
/// that issued them, and in [`DrawData`](super::DrawData) next to the matching
/// [`MaterialSource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(pub u64);

/// The material and uniform data one command is drawn with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialDraw {
    /// Must have a [`MaterialSource`] in [`DrawData::materials`](super::DrawData::materials).
    pub id: MaterialId,
    /// Bytes of [`DrawData::material_uniforms`](super::DrawData::material_uniforms) this
    /// command binds: the per-draw block followed by the packed parameters. Its length is a
    /// multiple of 16 and at most [`MAX_UNIFORM_BYTES`](crate::MAX_UNIFORM_BYTES); the start
    /// is a multiple of 16. A backend copies it to an offset it can bind
    /// (`min_uniform_buffer_offset_alignment`).
    pub uniforms: Range<u32>,
}

/// A material a frame uses, complete enough to compile it again on a new device. Backends
/// cache pipelines by `id`; a changed `wgsl` under an old id replaces the cached pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSource {
    pub id: MaterialId,
    /// Name for diagnostics.
    pub label: Arc<str>,
    /// The complete, already validated WGSL module with the entry points `vs_main` and
    /// `fs_material`, binding groups 0 to 3 as documented in `docs/renderer`.
    pub wgsl: Arc<str>,
}

impl MaterialId {
    /// Refers to no material: what registering past the registry limit returns. Drawing
    /// with it paints the fallback.
    pub const NONE: Self = Self(0);
}
