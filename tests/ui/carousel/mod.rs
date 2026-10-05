//! `Carousel`: both variants driven through the same events a native window delivers.
use crate::prelude::*;
use std::time::Duration;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::{Card, Carousel, CarouselOutput, Id, Rect, ScrollArea, Ui, Window};

mod gestures;
mod keys_wheel;
mod model;
mod render;
mod vertical;

pub const SIZE: Vec2 = Vec2::new(420.0, 260.0);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Host {
    Window,
    Card,
    Scroll,
}

/// A carousel in a window plus the clock that drives it deterministically.
pub struct Rig {
    pub c: Context,
    pub now: Instant,
    pub page: usize,
    pub pages: usize,
    pub looping: bool,
    pub images: bool,
    pub vertical: bool,
    pub autoplay: Option<Duration>,
    pub arrows: bool,
    pub host: Host,
    pub broken: bool,
    /// One source per page, kept between frames as the image cache expects.
    pub photos: Vec<ImageSource>,
    pub out: Option<CarouselOutput>,
    /// Page indices whose content closure ran in the last pass.
    pub built: Vec<usize>,
    /// `Ui::is_enabled` as seen by each content closure of the last pass.
    pub looked: Vec<bool>,
    pub changes: usize,
}

impl Rig {
    pub fn new(pages: usize) -> Self {
        Self::with_scale(pages, 1.0)
    }
    pub fn with_scale(pages: usize, scale: f64) -> Self {
        let mut c = Context::new();
        c.set_viewport(
            PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
            scale,
        );
        let mut rig = Self {
            c,
            now: Instant::now(),
            page: 0,
            pages,
            looping: false,
            images: false,
            vertical: false,
            autoplay: None,
            arrows: false,
            host: Host::Window,
            broken: false,
            photos: (0..64)
                .map(|i| ImageSource::rgba([2, 2], vec![(i * 4) as u8; 16]))
                .collect(),
            out: None,
            built: Vec::new(),
            looked: Vec::new(),
            changes: 0,
        };
        rig.pass();
        rig.pass();
        rig
    }
    pub fn reduced(&mut self) {
        let mut style = self.c.style().clone();
        style.motion.reduced_motion = true;
        self.c.set_style(style);
    }
    pub fn pass(&mut self) -> CarouselOutput {
        let (mut page, now) = (self.page, self.now);
        let mut built = Vec::new();
        let mut looked = Vec::new();
        let mut out = None;
        let mut carousel = Carousel::new("c")
            .pages(self.pages)
            .size(SIZE)
            .looping(self.looping)
            .arrows(self.arrows);
        if self.vertical {
            carousel = carousel.vertical();
        }
        if let Some(interval) = self.autoplay {
            carousel = carousel.autoplay(interval);
        }
        let (images, broken) = (self.images, self.broken);
        let show = |ui: &mut Ui<'_>| {
            if images {
                let photos = &self.photos;
                let source = |i: usize| {
                    if broken {
                        ImageSource::path("assets/images/missing-file.png")
                    } else {
                        photos[i].clone()
                    }
                };
                carousel.show_images_with(ui, &mut page, source, |ui, i| {
                    built.push(i);
                    ui.label(format!("Photo {i}"));
                })
            } else {
                carousel.show(ui, &mut page, |ui, i| {
                    built.push(i);
                    looked.push(ui.is_enabled());
                    ui.button(format!("Card {i}"));
                })
            }
        };
        let host = self.host;
        self.c.run_at(now, |c| {
            Window::new("Carousel")
                .default_size(Vec2::new(700.0, 560.0))
                .show(c, |ui| match host {
                    Host::Window => out = Some(show(ui)),
                    Host::Card => {
                        Card::new("host").show(ui, |ui| out = Some(show(ui)));
                    }
                    Host::Scroll => {
                        ScrollArea::vertical()
                            .id_source("host")
                            .max_height(320.0)
                            .show(ui, |ui| {
                                ui.label("Above");
                                out = Some(show(ui));
                                for i in 0..30 {
                                    ui.label(format!("Below {i}"));
                                }
                            });
                    }
                });
        });
        self.page = page;
        self.built = built;
        self.looked = looked;
        let out = out.unwrap();
        self.changes += usize::from(out.changed);
        self.out = Some(out);
        out
    }
    /// Advance the clock and run a pass.
    pub fn tick(&mut self, ms: u64) -> CarouselOutput {
        self.now += Duration::from_millis(ms);
        self.pass()
    }
    /// Run frames until nothing is scheduled any more; returns the number of frames.
    pub fn settle(&mut self) -> usize {
        for frames in 0..400 {
            if self.images {
                // Decoding happens on worker threads in real time.
                std::thread::sleep(Duration::from_millis(3));
            }
            self.tick(16);
            if !self.c.needs_repaint_at(self.now) && self.c.next_repaint().is_none() {
                return frames;
            }
        }
        panic!("carousel never settled");
    }
    /// Frames until the position rests, ignoring timers such as autoplay.
    pub fn rest(&mut self) {
        for _ in 0..400 {
            self.tick(16);
            let state = &self.c.probe().carousels[&self.id()];
            if state.settled && state.drag.is_none() {
                return;
            }
        }
        panic!("the position never came to rest");
    }
    pub fn rect(&self) -> Rect {
        self.out.unwrap().response.rect
    }
    /// A point on the front card that no control covers.
    pub fn empty(&self) -> Vec2 {
        if self.images {
            // The slides of the photo slider fill the middle; the sides show neighbours.
            let shift = if self.vertical {
                Vec2::new(60.0, 0.0)
            } else {
                Vec2::new(0.0, 50.0)
            };
            return self.rect().center() + shift;
        }
        self.rect().max - Vec2::new(60.0, 70.0)
    }
    pub fn position(&self) -> f32 {
        self.c
            .probe()
            .carousels
            .values()
            .next()
            .map_or(f32::NAN, |s| s.position)
    }
    pub fn state_pages(&self) -> usize {
        self.c.probe().carousels.len()
    }
    pub fn press(&mut self, at: Vec2) {
        self.c.move_pointer(at);
        self.c.primary_button(ElementState::Pressed);
    }
    pub fn release(&mut self) {
        self.c.primary_button(ElementState::Released);
    }
    /// Drag by `delta` in `steps` frames of `ms` each, starting from `from`; stays pressed.
    pub fn drag(&mut self, from: Vec2, delta: Vec2, steps: u32, ms: u64) {
        self.press(from);
        self.pass();
        for step in 1..=steps {
            self.c
                .move_pointer(from + delta * (step as f32 / steps as f32));
            self.tick(ms);
        }
    }
    pub fn click(&mut self, at: Vec2) -> CarouselOutput {
        self.press(at);
        self.release();
        self.tick(16)
    }
    pub fn key(&mut self, code: KeyCode) -> CarouselOutput {
        self.c.on_key_event(code, ElementState::Pressed, false);
        self.c.on_key_event(code, ElementState::Released, false);
        self.tick(16)
    }
    /// Give the carousel keyboard focus the way a user would: press it, release without moving.
    pub fn focus(&mut self) {
        let at = self.empty();
        self.click(at);
        self.rest();
        assert!(
            self.out.unwrap().response.has_focus,
            "the carousel takes focus"
        );
    }
    pub fn id(&self) -> Id {
        *self
            .c
            .probe()
            .carousels
            .keys()
            .next()
            .expect("a carousel is shown")
    }
}
