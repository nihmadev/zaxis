//! Drag-and-drop behavior through the public API and the event path.
use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};
use zaxis::Instant;
use zaxis::{
    DragEnd, DragReason, DropZones, Dropped, Insertion, Padding, Rect, Root, ScrollArea, Vec2,
};

mod appearance;
mod autoscroll;
mod integration;
mod keyboard;
mod lifecycle;
mod targets;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Item(pub u32);
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Other;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Inner {
    None,
    Accepts,
    Rejects,
    RejectsPassthrough,
}

/// Everything a test needs from one pass.
#[derive(Default)]
pub(crate) struct Seen {
    pub begins: u32,
    pub ends: Vec<DragEnd>,
    pub drops: Vec<Dropped<Item>>,
    pub source_rect: Rect,
    pub target_rect: Rect,
    pub inner_rect: Rect,
    pub foreign_rect: Rect,
    pub hover: bool,
    pub acceptable: bool,
    pub rejected: bool,
    pub inner_hover: bool,
    pub inner_rejected: bool,
    pub insertion: Option<Insertion>,
    pub clicks: u32,
    pub active: bool,
}

pub(crate) struct Scene {
    pub context: Context,
    pub start: Instant,
    pub step: u64,
    pub seen: Seen,
    /// Everything reported since the scene was created.
    pub log: Seen,
    pub show_source: bool,
    pub source_enabled: bool,
    pub show_target: bool,
    pub target_enabled: bool,
    pub zones: Option<DropZones>,
    pub payload: u32,
    pub inner: Inner,
    /// Target inside a 50px-high scroll area, so its lower part is clipped.
    pub clipped: bool,
    /// A floating window over this rectangle.
    pub cover: Option<Rect>,
    pub text: String,
    pub with_text: bool,
}

pub(crate) fn scene(scale: f64) -> Scene {
    let mut context = Context::new();
    context.set_viewport(
        PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
        scale,
    );
    let mut scene = Scene {
        context,
        start: Instant::now(),
        step: 0,
        seen: Seen::default(),
        log: Seen::default(),
        show_source: true,
        source_enabled: true,
        show_target: true,
        target_enabled: true,
        zones: None,
        payload: 7,
        inner: Inner::None,
        clipped: false,
        cover: None,
        text: String::new(),
        with_text: false,
    };
    scene.frame();
    scene.frame();
    scene
}

impl Scene {
    /// One pass: a button source, a target for `Item` and one for `Other`.
    pub fn frame(&mut self) {
        self.step += 1;
        let now = self.start + Duration::from_millis(self.step * 16);
        self.pass(now);
    }

    pub fn pass(&mut self, now: Instant) {
        let mut seen = Seen::default();
        let (payload, inner, clipped, cover) = (self.payload, self.inner, self.clipped, self.cover);
        let (show_source, enabled, show_target, target_enabled, zones) = (
            self.show_source,
            self.source_enabled,
            self.show_target,
            self.target_enabled,
            self.zones,
        );
        let with_text = self.with_text;
        let text = &mut self.text;
        self.context.run_at(now, |c| {
            Root::new().padding(Padding::all(20.0)).show(c, |ui| {
                if show_source {
                    let out = zaxis::DragSource::new(Id::new("source"), Item(payload))
                        .enabled(enabled)
                        .show(ui, |ui| ui.button("Drag me"));
                    seen.source_rect = out.inner.rect;
                    seen.begins += u32::from(out.started);
                    seen.active = out.active;
                    seen.clicks += u32::from(out.inner.clicked());
                    seen.ends.extend(out.finished);
                }
                if with_text {
                    ui.text_edit(text);
                }
                ui.add_space(60.0);
                if show_target {
                    let mut target =
                        zaxis::DropTarget::new(Id::new("target"), |item: &Item| item.0 == 7)
                            .enabled(target_enabled);
                    if let Some(zones) = zones {
                        target = target.zones(zones);
                    }
                    let content = |ui: &mut zaxis::Ui<'_>| {
                        ui.allocate_space(Vec2::new(300.0, 80.0));
                        if inner != Inner::None {
                            let mut nested =
                                zaxis::DropTarget::new(Id::new("inner"), move |item: &Item| {
                                    inner == Inner::Accepts && item.0 == 7
                                })
                                .passthrough_rejected(inner == Inner::RejectsPassthrough);
                            if inner == Inner::Accepts {
                                nested = nested.zones(DropZones::rows());
                            }
                            let out = nested.show(ui, |ui| {
                                ui.allocate_space(Vec2::new(120.0, 30.0));
                            });
                            seen.inner_rect = out.response.rect;
                            seen.inner_hover = out.hovering;
                            seen.inner_rejected = out.rejected;
                        }
                    };
                    let out = if clipped {
                        let mut shown = None;
                        ScrollArea::vertical()
                            .id_source("clip")
                            .max_height(50.0)
                            .show(ui, |ui| shown = Some(target.show(ui, content)));
                        shown.unwrap()
                    } else {
                        target.show(ui, content)
                    };
                    seen.target_rect = out.response.rect;
                    seen.hover = out.hovering;
                    seen.acceptable = out.acceptable;
                    seen.rejected = out.rejected;
                    seen.insertion = out.insertion;
                    seen.drops.extend(out.dropped);
                }
                ui.add_space(20.0);
                let out = ui.drop_target(
                    Id::new("foreign"),
                    |_: &Other| true,
                    |ui| {
                        ui.allocate_space(Vec2::new(300.0, 40.0));
                    },
                );
                seen.foreign_rect = out.response.rect;
            });
            if let Some(area) = cover {
                zaxis::Window::new("Cover")
                    .default_position(area.min)
                    .default_size(area.size())
                    .show(c, |ui| {
                        ui.label("Over the target");
                    });
            }
        });
        self.log.begins += seen.begins;
        self.log.clicks += seen.clicks;
        self.log.ends.extend(seen.ends.iter().copied());
        self.log.drops.extend(seen.drops.iter().cloned());
        self.seen = seen;
    }

    pub fn move_to(&mut self, p: Vec2) {
        self.context.move_pointer(p);
    }
    pub fn press(&mut self) {
        self.context.primary_button(ElementState::Pressed);
    }
    pub fn release(&mut self) {
        self.context.primary_button(ElementState::Released);
    }
    pub fn key(&mut self, code: KeyCode) {
        self.context.key(code, ElementState::Pressed, false);
        self.context.key(code, ElementState::Released, false);
    }
    /// Press on the source, then travel far enough to start the drag and let
    /// two passes elapse so targets know the payload.
    pub fn begin(&mut self) -> Vec2 {
        let start = self.seen.source_rect.center();
        self.move_to(start);
        self.press();
        self.move_to(start + Vec2::new(0.0, 12.0));
        self.frame();
        self.frame();
        start
    }
    /// Hover `p` for one pass.
    pub fn hover_at(&mut self, p: Vec2) {
        self.move_to(p);
        self.frame();
    }
}
