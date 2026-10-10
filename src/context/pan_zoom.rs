//! Addressed camera input. Owners and geometry come from the last published pass.
//! No camera model is retained here; declarations, ordered input and ordinary capture only.
use super::{Context, HitAction, Id};
use crate::{PanZoomState, Rect, Vec2, ZoomWheel};
use std::collections::HashMap;

const WHEEL_QUEUE_LIMIT: usize = 256;

#[derive(Clone, Copy)]
pub(crate) struct CameraTarget {
    pub camera: PanZoomState,
    pub viewport: Rect,
    pub wheel: ZoomWheel,
    pub pan: bool,
    /// The enclosing published scroll tree; scrolling descendants leaves this viewport still.
    pub scroll_parent: Option<Id>,
}

#[derive(Clone, Copy)]
pub(crate) enum CameraInput {
    /// Delta in the owner's parent-layout units, with ancestor scale removed.
    Pan(Vec2),
    /// Anchor in viewport logical units. Modifiers/position are sampled at dispatch.
    Zoom {
        anchor: Vec2,
        delta: f32,
        modifiers: winit::keyboard::ModifiersState,
    },
}

#[derive(Default)]
pub(crate) struct CameraRouting {
    pub current: HashMap<Id, CameraTarget>,
    pub previous: HashMap<Id, CameraTarget>,
    input: Vec<(Id, CameraInput)>,
}

impl CameraRouting {
    pub(super) fn stationary_in_scroll(
        &self,
        id: Id,
        area: Id,
        states: &HashMap<Id, super::scroll::ScrollState>,
    ) -> bool {
        let Some(target) = self.previous.get(&id) else {
            return false;
        };
        let mut parent = target.scroll_parent;
        while let Some(id) = parent {
            if id == area {
                return false;
            }
            parent = states.get(&id).and_then(|state| state.parent);
        }
        true
    }

    pub(super) fn begin_pass(&mut self) {
        self.current.clear();
    }
    pub(super) fn finish_pass(&mut self) {
        self.previous = std::mem::take(&mut self.current);
        // The build had its one chance to take input. Hidden/removed owners drop it too.
        self.input.clear();
    }
    pub(crate) fn take(&mut self, id: Id) -> Vec<CameraInput> {
        let mut result = Vec::new();
        self.input.retain(|(owner, event)| {
            if *owner == id {
                result.push(*event);
                false
            } else {
                true
            }
        });
        result
    }
    pub(super) fn cancel(&mut self) {
        self.input.clear();
    }
    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.previous.len(), self.input.len())
    }
    pub(super) fn pan_has_room(&self, id: Id) -> bool {
        self.input.len() <= WHEEL_QUEUE_LIMIT
            || self
                .input
                .last()
                .is_some_and(|(owner, event)| *owner == id && matches!(event, CameraInput::Pan(_)))
    }
}

impl Context {
    pub(crate) fn take_camera_input(
        &mut self,
        id: Id,
        window: Id,
        enabled: bool,
    ) -> Vec<CameraInput> {
        let input = self.camera_routing.take(id);
        if enabled && self.key_layer_open(window) {
            input
        } else {
            Vec::new()
        }
    }

    pub(super) fn cancel_camera_input(&mut self) {
        if self
            .interaction
            .capture
            .is_some_and(|capture| self.camera_routing.previous.contains_key(&capture.hit.id))
        {
            self.interaction.capture = None;
            self.gesture_cancel();
        }
        self.camera_routing.cancel();
        self.camera_routing.previous.clear();
    }
    pub(super) fn route_camera_wheel(&mut self, pointer: Vec2, window: Id, delta: Vec2) -> bool {
        if !self.input.focused || !delta.is_finite() || delta.y == 0.0 {
            return false;
        }
        let mods = self.input.modifiers;
        let matched = |wheel| match wheel {
            ZoomWheel::Primary => {
                let primary = if self.action_platform() == crate::Platform::Mac {
                    mods.super_key() && !mods.control_key()
                } else {
                    mods.control_key() && !mods.super_key()
                };
                primary && !mods.alt_key() && !mods.shift_key()
            }
            ZoomWheel::Unmodified => mods.is_empty(),
            ZoomWheel::Disabled => false,
        };
        let found = self.interaction.previous_hits.iter().rev().find_map(|hit| {
            let target = self.camera_routing.previous.get(&hit.id)?;
            (hit.window == window
                && hit.action != HitAction::Block
                && hit.rect.contains(pointer)
                && hit.clip.contains(pointer)
                && matched(target.wheel))
            .then_some((hit.id, *target))
        });
        let Some((id, target)) = found else {
            return false;
        };
        if self.camera_routing.input.len() >= WHEEL_QUEUE_LIMIT {
            self.report(super::DiagnosticKind::InvalidUsage, Some(id), None, || {
                "PanZoom input queue full; wheel returned to normal scroll/host routing".into()
            });
            return false;
        }
        let parent = self.visuals.to_local(id).point(pointer);
        self.camera_routing.input.push((
            id,
            CameraInput::Zoom {
                anchor: parent - target.viewport.min,
                delta: -delta.y,
                modifiers: mods,
            },
        ));
        true
    }

    /// Called by the shared primary gesture after its drag threshold, preserving event order.
    pub(super) fn route_camera_pan(&mut self, id: Id, delta: Vec2) {
        if !self
            .camera_routing
            .previous
            .get(&id)
            .is_some_and(|target| target.pan)
        {
            return;
        }
        let delta = self.visuals.to_local(id).vector(delta);
        if !delta.is_finite() {
            return;
        }
        if let Some((owner, CameraInput::Pan(last))) = self.camera_routing.input.last_mut() {
            if *owner == id {
                *last += delta;
                return;
            }
        }
        // One extra slot is reserved for the sole captured pan, even when wheel is full.
        if !self.camera_routing.pan_has_room(id) {
            return;
        }
        self.camera_routing
            .input
            .push((id, CameraInput::Pan(delta)));
    }
}
