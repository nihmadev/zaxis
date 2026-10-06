//! Materials in a pass: registration, the clock of animated ones, how a drawn material
//! reaches its element, and the repaint it asks for while it is visible.

use super::{Context, DiagnosticKind, Id};
use crate::{
    material::{block, FrameData, MaterialProgram},
    time::Instant,
    Material, MaterialError, MaterialId, Vec2,
};
use std::sync::Arc;

/// Seconds after which `in.time` starts again at zero: keeps f32 precision for long sessions.
const TIME_WRAP: f64 = 3600.0;

/// A material applied to an element, with everything that varies per frame. Compared by
/// value between passes: a change rebuilds the draw commands, never the mesh.
#[derive(Clone, Debug)]
pub struct MaterialUse {
    pub(crate) program: Arc<MaterialProgram>,
    /// Packed parameters, as long as the material's layout says.
    pub(crate) params: Arc<[u8]>,
    /// Size of the shape in logical pixels, after the scale of visuals around it.
    pub(crate) size: Vec2,
    /// Displayed part of the widget texture: origin and size in 0..1.
    pub(crate) texture_rect: [f32; 4],
    pub(crate) time: f32,
    pub(crate) delta: f32,
    /// This draw reads the clock: it keeps frames coming while it is visible.
    pub(crate) animated: bool,
}

impl PartialEq for MaterialUse {
    fn eq(&self, other: &Self) -> bool {
        self.program.source.id == other.program.source.id
            && self.params == other.params
            && self.size == other.size
            && self.texture_rect == other.texture_rect
            && self.time == other.time
            && self.delta == other.delta
            && self.animated == other.animated
    }
}

impl MaterialUse {
    pub fn id(&self) -> MaterialId {
        self.program.source.id
    }
    pub fn size(&self) -> Vec2 {
        self.size
    }
    /// The packed parameters of this draw.
    pub fn params(&self) -> &[u8] {
        &self.params
    }

    /// The uniform block of a draw: frame data, then the parameters.
    pub(crate) fn block(&self) -> Vec<u8> {
        block(
            &FrameData {
                size: self.size.to_array(),
                time: self.time,
                delta: self.delta,
                texture_rect: self.texture_rect,
            },
            &self.params,
        )
    }

    /// Scale the shape by a visual transform around it.
    pub(crate) fn scale(&mut self, factor: f32) {
        self.size *= factor;
    }
}

/// The clock animated materials read.
#[derive(Default)]
pub(crate) struct MaterialState {
    epoch: Option<Instant>,
    /// Time of the previous pass, for `in.delta`.
    previous: Option<Instant>,
    /// A visible animated material was drawn in the last finished pass.
    pub(crate) animating: bool,
}

impl Context {
    /// Register `material` and return its id; see
    /// [`SharedResources::register_material`](crate::SharedResources::register_material).
    /// A rejected description is reported once through [`Self::diagnostics`] and gets an id
    /// whose draws use their fallback.
    pub fn register_material(&mut self, material: &Material) -> MaterialId {
        if let Err(error) = self.shared.try_register_material(material) {
            self.report_material_error(&error);
        }
        self.shared.register_material(material)
    }

    /// Register `material`, returning why it is invalid instead of reporting it.
    pub fn try_register_material(
        &mut self,
        material: &Material,
    ) -> Result<MaterialId, MaterialError> {
        self.shared.try_register_material(material)
    }

    /// Why the material `id` cannot be drawn, if it cannot.
    pub fn material_error(&self, id: MaterialId) -> Option<MaterialError> {
        self.shared.material_error(id)
    }

    pub(crate) fn report_material_error(&mut self, error: &MaterialError) {
        // One report per distinct material and message, whatever the pass.
        let id = Id::new(("material-error", error.label(), error.message()));
        self.report(DiagnosticKind::InvalidShader, Some(id), None, || {
            error.to_string()
        });
    }

    /// `in.time` and `in.delta` for a draw now: zeros unless it is animated, and when motion is
    /// reduced.
    pub(crate) fn material_clock(&mut self, animated: bool) -> (f32, f32) {
        if !animated || self.style.motion.reduced_motion {
            return (0.0, 0.0);
        }
        let now = self.frame_time;
        let epoch = *self.materials.epoch.get_or_insert(now);
        let time = now.saturating_duration_since(epoch).as_secs_f64() % TIME_WRAP;
        let delta = self.materials.previous.map_or(0.0, |p| {
            now.saturating_duration_since(p).as_secs_f64().min(1.0)
        });
        (time as f32, delta as f32)
    }

    /// Make the element just painted under `id` a draw of `material`, wherever it went. A
    /// backdrop material is also a backdrop effect of `backdrop` sigma.
    pub(crate) fn mark_material(&mut self, id: Id, material: MaterialUse, backdrop: Option<f32>) {
        if let Some(paint) = self
            .placements
            .stack
            .last_mut()
            .and_then(|p| p.paints.last_mut())
            .filter(|p| p.id == id)
        {
            paint.material = Some(material);
            paint.blur = backdrop;
        } else if let Some(paint) = self.scrolling.pending.last_mut().filter(|p| p.id == id) {
            paint.material = Some(material);
            paint.blur = backdrop;
        } else if let Some(element) = self.paint_state.last_element_mut(id) {
            element.material = Some(material);
            element.blur = backdrop;
        }
    }

    /// End of pass: ask for the next frame while an animated material is on screen. Only
    /// elements that were emitted count, so a material scrolled out of its clip or hidden
    /// with its window stops asking.
    pub(super) fn finish_materials(&mut self) {
        let animating = !self.style.motion.reduced_motion
            && self
                .paint_state
                .elements
                .iter()
                .any(|e| e.material.as_ref().is_some_and(|m| m.animated));
        self.materials.animating = animating;
        self.materials.previous = Some(self.frame_time);
        if animating {
            let interval = self
                .style
                .motion
                .frame_interval
                .max(std::time::Duration::from_millis(1));
            self.request_repaint_after(interval);
        }
    }
}
