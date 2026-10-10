use super::{PanZoom, PanZoomOutput, PanZoomState};
use crate::{
    context::pan_zoom::{CameraInput, CameraTarget},
    AccessRole, Rect, Sense, Ui, Vec2,
};

impl PanZoom {
    /// Build content once in local coordinates; `visible` is the full viewport in those
    /// coordinates. Cull application objects explicitly. Allocates only `size` in the parent.
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        camera: &mut PanZoomState,
        build: impl FnOnce(&mut Ui<'_>, Rect) -> R,
    ) -> PanZoomOutput<R> {
        ui.layout_item(|ui| self.show_inner(ui, camera, build))
    }

    fn show_inner<R>(
        self,
        ui: &mut Ui<'_>,
        camera: &mut PanZoomState,
        build: impl FnOnce(&mut Ui<'_>, Rect) -> R,
    ) -> PanZoomOutput<R> {
        let scope = ui.scope.with(("pan-zoom", self.source));
        let id = scope.with("viewport");
        let viewport = ui.allocate_space(self.size);
        // Ancestor placement/scroll clips are applied at emission in their own units.
        let clip = viewport.intersect(ui.clip);
        let enabled = ui.enabled && self.enabled && !clip.is_empty();
        let previous = ui
            .context
            .camera_routing
            .previous
            .get(&id)
            .map(|target| target.camera);
        let initial = *camera;
        camera.normalize(self.limits.0, self.limits.1);
        let mut panned = false;
        let mut zoomed = false;
        for event in ui.context.take_camera_input(id, ui.window, enabled) {
            let before = *camera;
            match event {
                CameraInput::Pan(delta) if self.pan => {
                    camera.translation += delta;
                    camera.normalize(self.limits.0, self.limits.1);
                    panned |= before != *camera;
                }
                CameraInput::Zoom {
                    anchor,
                    delta,
                    modifiers,
                } if self.wheel != super::ZoomWheel::Disabled => {
                    // The modifiers are part of the addressed event, not today's InputState.
                    let _ = modifiers;
                    let scale = camera.scale * (delta * 0.002).clamp(-20.0, 20.0).exp();
                    camera.zoom_at(scale, anchor, self.limits.0..=self.limits.1);
                    zoomed |= before != *camera;
                }
                _ => {}
            }
        }
        let changed = initial != *camera || previous.is_some_and(|previous| previous != *camera);
        let mut response = ui.response(id, viewport, enabled);
        if changed {
            response.mark_changed();
        }
        ui.context.register_hit(crate::context::HitRegion {
            id,
            window: ui.window,
            rect: viewport,
            clip: ui.clip,
            action: if enabled {
                crate::context::HitAction::Interact(if self.pan {
                    Sense::DRAG
                } else {
                    Sense::HOVER
                })
            } else {
                crate::context::HitAction::Block
            },
        });
        let scroll_parent = ui
            .context
            .scrolling
            .owner(ui.window)
            .map(|scope| ui.context.scrolling.scopes[scope].id);
        if ui
            .context
            .camera_routing
            .current
            .insert(
                id,
                CameraTarget {
                    camera: *camera,
                    viewport,
                    wheel: if enabled {
                        self.wheel
                    } else {
                        super::ZoomWheel::Disabled
                    },
                    pan: enabled && self.pan,
                    scroll_parent,
                },
            )
            .is_some()
        {
            ui.context
                .report(crate::DiagnosticKind::IdCollision, Some(id), None, || {
                    "two PanZoom containers share one source in the same scope".into()
                });
        }
        // AT describes the viewport; descendants keep their own names, actions and focus.
        let access = ui.a11y_begin(id, AccessRole::Group, |node| {
            node.label(self.label).disabled(!enabled)
                .description("Drag the background to pan; Ctrl or Command + wheel to zoom. Camera controls are supplied by the application.");
        });
        let transform = camera.transform(viewport.min);
        let visible = camera
            .transform(Vec2::ZERO)
            .inverse()
            .rect(Rect::from_min_size(Vec2::ZERO, self.size));
        ui.context.begin_placement(ui.window);
        ui.context.visuals.depth += 1;
        ui.context.visuals.clips.push((ui.window, clip));
        // Local origin is zero, independent of the parent column and negative scene bounds.
        let inner = ui.region(
            scope,
            Rect::from_min_size(Vec2::ZERO, self.size / camera.scale),
            Rect::from_min_size(Vec2::splat(-1.0e9), Vec2::splat(2.0e9)),
            enabled,
            |ui| build(ui, visible),
        );
        ui.context.visuals.depth -= 1;
        ui.context.visuals.clips.pop();
        let placement = ui.context.end_placement();
        ui.context
            .place_visual(placement, transform, 1.0, clip, enabled);
        ui.a11y_end(access, Some(viewport));
        if changed {
            ui.context.request_repaint();
        }
        PanZoomOutput {
            inner,
            response,
            viewport,
            visible,
            changed,
            panned,
            zoomed,
            camera: *camera,
        }
    }
}
