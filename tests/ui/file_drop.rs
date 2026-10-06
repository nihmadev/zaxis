//! Files dragged in from the system: hover, drop, targets, priority, modals and windows.
use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::WindowEvent};
use zaxis::{DropTarget, FileFilter, Modal, Padding, Rect, Root};

mod lifecycle;
mod targets;

pub(crate) fn names(files: &[PickedFile]) -> Vec<String> {
    files.iter().map(|f| f.name().to_owned()).collect()
}

pub(crate) fn file(name: &str) -> PickedFile {
    PickedFile::from_memory(name, vec![1_u8, 2, 3])
}

/// What one pass saw of a scene with an outer zone, an optional inner zone and an
/// optional modal over both.
#[derive(Default)]
pub(crate) struct Seen {
    pub outer: Vec<String>,
    pub inner: Vec<String>,
    pub global: Vec<String>,
    pub outer_rect: Rect,
    pub inner_rect: Rect,
    pub outer_hover: bool,
    pub inner_hover: bool,
    pub outer_acceptable: bool,
    pub outer_dropped_flag: bool,
    pub global_hovered: Vec<String>,
}

pub(crate) struct Scene {
    pub context: Context,
    pub seen: Seen,
    pub inner: bool,
    pub modal: bool,
    pub outer_filter: Option<FileFilter>,
    pub outer_max: usize,
    step: u64,
    start: Instant,
}

pub(crate) fn scene() -> Scene {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let mut scene = Scene {
        context,
        seen: Seen::default(),
        inner: false,
        modal: false,
        outer_filter: None,
        outer_max: usize::MAX,
        step: 0,
        start: Instant::now(),
    };
    scene.frame();
    scene.frame();
    scene
}

impl Scene {
    pub fn frame(&mut self) {
        self.step += 1;
        let now = self.start + Duration::from_millis(self.step * 16);
        let mut seen = Seen::default();
        let (inner_on, max) = (self.inner, self.outer_max);
        let filter = self.outer_filter.clone();
        let mut open = self.modal;
        self.context.run_at(now, |c| {
            seen.global_hovered = names(c.hovered_files());
            Root::new().padding(Padding::all(20.0)).show(c, |ui| {
                let mut target = DropTarget::files(Id::new("outer"));
                if let Some(filter) = &filter {
                    target = target.accepts_files(filter.clone());
                }
                let out = target.max_files(max).show(ui, |ui| {
                    ui.label("outer zone with some padding text to give it size");
                    if inner_on {
                        let inner = DropTarget::files(Id::new("inner")).show(ui, |ui| {
                            ui.label("inner zone");
                        });
                        seen.inner = names(&inner.dropped_files);
                        seen.inner_rect = inner.response.rect;
                        seen.inner_hover = inner.files_hovering;
                    }
                });
                seen.outer = names(&out.dropped_files);
                seen.outer_rect = out.response.rect;
                seen.outer_hover = out.files_hovering;
                seen.outer_acceptable = out.files_acceptable;
                seen.outer_dropped_flag = out.response.files_dropped();
                Modal::new("m").show(ui, &mut open, |ui| {
                    ui.label("modal content");
                });
            });
            seen.global = names(&c.take_dropped_files());
        });
        self.seen = seen;
    }

    pub fn point_at(&mut self, p: Vec2) {
        Driver::move_pointer(&mut self.context, p);
    }

    pub fn drop_files(&mut self, files: Vec<PickedFile>) {
        self.context.simulate_drop_files(files);
        self.frame();
    }

    /// The pointer left the window, as it does for a drag that comes from outside.
    pub fn pointer_away(&mut self) {
        self.context.on_window_event(&WindowEvent::CursorLeft {
            device_id: winit::event::DeviceId::dummy(),
        });
    }
}

/// A path that is not valid UTF-8, where the platform allows one.
pub(crate) fn odd_path() -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(vec![b'b', 0xff, 0xfe, b'.', b't']).into()
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        std::ffi::OsString::from_wide(&[0x62, 0xD800, 0x2E, 0x74]).into()
    }
}

#[test]
fn winit_events_feed_the_same_state() {
    let mut s = scene();
    s.point_at(Vec2::new(700.0, 500.0));
    let a = WindowEvent::HoveredFile("a.txt".into());
    let b = WindowEvent::HoveredFile("b.txt".into());
    assert!(s.context.on_window_event(&a).repaint);
    s.context.on_window_event(&b);
    assert_eq!(names(s.context.hovered_files()), ["a.txt", "b.txt"]);
    for name in ["a.txt", "b.txt"] {
        s.context
            .on_window_event(&WindowEvent::DroppedFile(name.into()));
    }
    assert!(
        s.context.hovered_files().is_empty(),
        "a drop ends the hover"
    );
    s.frame();
    assert_eq!(s.seen.global, ["a.txt", "b.txt"]);
}
