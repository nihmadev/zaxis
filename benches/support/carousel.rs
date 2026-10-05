//! Real `Carousel` paths: idle reuse, keyboard turns, swipes, jumps, photos and lifecycle.
//! Assertions run outside the timed regions and check the resulting page and geometry.
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
};
use zaxis::Instant;
use zaxis::{Carousel, CarouselOutput, Context, ImageSource, Root, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarouselCase {
    Cached,
    Keys,
    Swipe,
    Jump,
    Photos,
    Lifecycle,
}
impl CarouselCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cached => "carousel_cached",
            Self::Keys => "carousel_keys",
            Self::Swipe => "carousel_swipe",
            Self::Jump => "carousel_jump",
            Self::Photos => "carousel_photos",
            Self::Lifecycle => "carousel_lifecycle",
        }
    }
    pub fn interactive(self) -> bool {
        !matches!(self, Self::Cached | Self::Lifecycle)
    }
}

pub struct Probe {
    kind: CarouselCase,
    pages: usize,
    page: usize,
    photos: Vec<ImageSource>,
    out: Option<CarouselOutput>,
    now: Instant,
    step: usize,
    built: usize,
    changes: usize,
    tessellations: u64,
}

impl Probe {
    pub fn new(kind: CarouselCase, count: usize) -> Self {
        let pages = count.clamp(3, 2000);
        let photos = if kind == CarouselCase::Photos {
            (0..pages)
                .map(|i| ImageSource::rgba([4, 4], vec![(i % 256) as u8; 64]))
                .collect()
        } else {
            Vec::new()
        };
        Self {
            kind,
            pages,
            page: 0,
            photos,
            out: None,
            now: Instant::now(),
            step: 0,
            built: 0,
            changes: 0,
            tessellations: 0,
        }
    }
    fn pointer(c: &mut Context, p: Vec2) {
        let s = c.scale_factor();
        c.on_window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(f64::from(p.x * s), f64::from(p.y * s)),
        });
    }
    fn button(c: &mut Context, state: ElementState) {
        c.on_window_event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state,
            button: MouseButton::Left,
        });
    }
    fn press(c: &mut Context, key: KeyCode) {
        c.on_key_event(key, ElementState::Pressed, false);
        c.on_key_event(key, ElementState::Released, false);
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(16);
        self.tessellations = c.cache_stats().tessellated_elements;
        let Some(out) = self.out else { return };
        match self.kind {
            CarouselCase::Keys | CarouselCase::Photos => {
                c.request_focus(out.response.id);
                Self::press(c, KeyCode::ArrowRight);
            }
            CarouselCase::Jump => {
                c.request_focus(out.response.id);
                Self::press(
                    c,
                    if step.is_multiple_of(2) {
                        KeyCode::End
                    } else {
                        KeyCode::Home
                    },
                );
            }
            CarouselCase::Swipe => {
                let from = out.response.rect.center() + Vec2::new(120.0, 60.0);
                Self::pointer(c, from);
                Self::button(c, ElementState::Pressed);
                Self::pointer(c, from - Vec2::new(40.0, 0.0));
                Self::pointer(c, from - Vec2::new(160.0, 0.0));
                Self::button(c, ElementState::Released);
            }
            CarouselCase::Cached | CarouselCase::Lifecycle => {}
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        self.built = 0;
        if self.kind == CarouselCase::Lifecycle && self.step % 2 == 1 {
            c.run_at(self.now, |_| {});
            self.out = None;
            return;
        }
        let (photos, built, page) = (&self.photos, &mut self.built, &mut self.page);
        let carousel = Carousel::new("bench")
            .pages(self.pages)
            .size(Vec2::new(360.0, 220.0))
            .looping(true);
        let mut out = None;
        c.run_at(self.now, |c| {
            Root::new().show(c, |ui| {
                out = Some(if photos.is_empty() {
                    carousel.show(ui, page, |ui, i| {
                        *built += 1;
                        ui.label(format!("Card {i}"));
                    })
                } else {
                    carousel.show_images(ui, page, |i| photos[i].clone())
                });
            })
        });
        let out = out.unwrap();
        self.changes += usize::from(out.changed);
        self.out = Some(out);
    }
    pub fn verify(&self, c: &Context) {
        let Some(out) = &self.out else {
            assert!(!c.wants_animation_frame());
            return;
        };
        assert!(out.page < self.pages && out.page == self.page);
        assert!(self.built <= 3, "built {} pages", self.built);
        match self.kind {
            CarouselCase::Cached if self.step > 12 => {
                assert!(!c.needs_repaint() && !c.wants_animation_frame());
                assert_eq!(self.tessellations, c.cache_stats().tessellated_elements);
                assert_eq!(self.changes, 0);
            }
            CarouselCase::Keys | CarouselCase::Photos if self.step > 12 => {
                assert!(self.changes > 0, "arrow keys turned no page");
            }
            CarouselCase::Swipe if self.step > 12 => {
                assert!(self.changes > 0, "swipes turned no page");
            }
            CarouselCase::Jump if self.step > 12 => {
                assert!(self.changes > 0, "End/Home turned no page");
                assert!(self.built <= 2, "a jump built {} cards", self.built);
            }
            _ => {}
        }
    }
}
