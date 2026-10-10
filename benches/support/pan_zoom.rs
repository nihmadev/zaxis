//! Public PanZoom with application culling; assertions are outside timed sections.
use zaxis::testing::Inspect;
use zaxis::winit::{
    event::{ElementState, MouseButton},
    keyboard::ModifiersState,
};
use zaxis::{
    vec2, Color, Context, InputEvent, Padding, PanZoom, PanZoomOutput, PanZoomState, Rect, Root,
    ScrollArea, Shape, Slider, Vec2, WheelDelta,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanZoomCase {
    Idle,
    Pan,
    Zoom,
    Burst,
    Controls,
    Update,
    Lifecycle,
}
impl PanZoomCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "pan_zoom_idle",
            Self::Pan => "pan_zoom_pan",
            Self::Zoom => "pan_zoom_zoom",
            Self::Burst => "pan_zoom_wheel_burst",
            Self::Controls => "pan_zoom_nested_controls",
            Self::Update => "pan_zoom_model_update",
            Self::Lifecycle => "pan_zoom_mount_unmount",
        }
    }
    pub fn interactive(self) -> bool {
        matches!(self, Self::Pan | Self::Zoom | Self::Burst | Self::Controls)
    }
}

pub struct Probe {
    kind: PanZoomCase,
    count: usize,
    camera: PanZoomState,
    before: PanZoomState,
    out: Option<PanZoomOutput<()>>,
    step: usize,
    built: usize,
    consumed: bool,
    anchor: Vec2,
    anchored_local: Vec2,
    tessellations: u64,
    revision: u64,
    slider: f32,
    before_slider: f32,
    scroll: Vec2,
    scroll_id: Option<zaxis::Id>,
    scroll_rect: Rect,
    before_scroll: Vec2,
}
impl Probe {
    pub fn new(kind: PanZoomCase, count: usize) -> Self {
        Self {
            kind,
            count,
            camera: PanZoomState::default(),
            before: PanZoomState::default(),
            out: None,
            step: 0,
            built: 0,
            consumed: true,
            anchor: vec2(350.0, 250.0),
            anchored_local: Vec2::ZERO,
            tessellations: 0,
            revision: 0,
            slider: 0.5,
            before_slider: 0.5,
            scroll: Vec2::ZERO,
            scroll_id: None,
            scroll_rect: Rect::default(),
            before_scroll: Vec2::ZERO,
        }
    }
    fn pointer(c: &mut Context, p: Vec2) -> bool {
        let p = p * c.scale_factor();
        c.on_input(InputEvent::PointerMoved {
            x: p.x as f64,
            y: p.y as f64,
        })
        .consumed
    }
    fn button(c: &mut Context, state: ElementState) -> bool {
        c.on_input(InputEvent::Button {
            button: MouseButton::Left,
            state,
        })
        .consumed
    }
    fn wheel(c: &mut Context, pixels: f32) -> bool {
        c.on_input(InputEvent::Wheel(WheelDelta::Pixels(
            vec2(0.0, pixels) * c.scale_factor(),
        )))
        .consumed
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.before = self.camera;
        self.tessellations = c.cache_stats().tessellated_elements;
        self.revision = c.draw_data().revision;
        self.before_slider = self.slider;
        self.before_scroll = self.scroll;
        self.consumed = true;
        let Some(out) = &self.out else { return };
        self.anchor = out.displayed_viewport(c).min + vec2(350.0, 250.0);
        self.anchored_local = out.screen_to_local(c, self.anchor);
        match self.kind {
            PanZoomCase::Pan => {
                Self::pointer(c, self.anchor);
                self.consumed &= Self::button(c, ElementState::Pressed);
                let delta = if step.is_multiple_of(2) {
                    vec2(12.0, 8.0)
                } else {
                    vec2(-12.0, -8.0)
                };
                self.consumed &= Self::pointer(c, self.anchor + delta);
                self.consumed &= Self::button(c, ElementState::Released);
            }
            PanZoomCase::Zoom | PanZoomCase::Burst => {
                Self::pointer(c, self.anchor);
                c.on_input(InputEvent::Modifiers(ModifiersState::CONTROL));
                if self.kind == PanZoomCase::Burst {
                    for _ in 0..4 {
                        self.consumed &= Self::wheel(c, 60.0);
                        self.consumed &= Self::wheel(c, -60.0);
                    }
                } else {
                    self.consumed &=
                        Self::wheel(c, if step.is_multiple_of(2) { 60.0 } else { -60.0 });
                }
                c.on_input(InputEvent::Modifiers(ModifiersState::empty()));
            }
            PanZoomCase::Controls => {
                let pos = out.local_to_screen(
                    c,
                    vec2(if step.is_multiple_of(2) { 18.0 } else { 165.0 }, 18.0),
                );
                Self::pointer(c, pos);
                self.consumed &= Self::button(c, ElementState::Pressed);
                self.consumed &= Self::button(c, ElementState::Released);
                let pos = out.local_to_screen(c, self.scroll_rect.center());
                Self::pointer(c, pos);
                self.consumed &= Self::wheel(c, if step.is_multiple_of(2) { -20.0 } else { 20.0 });
            }
            _ => {}
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        if self.kind == PanZoomCase::Lifecycle && !self.step.is_multiple_of(2) {
            c.run(|_| {});
            self.out = None;
            self.built = 0;
            return;
        }
        self.built = 0;
        let mut out = None;
        c.run(|c| {
            Root::new().padding(Padding::all(0.0)).show(c, |ui| {
                out = Some(PanZoom::new("bench", vec2(600.0, 400.0)).show(
                    ui,
                    &mut self.camera,
                    |ui, visible| {
                        for i in 0..self.count {
                            let rect = Rect::from_min_size(
                                vec2((i % 32) as f32 * 40.0 - 20.0, (i / 32) as f32 * 40.0 - 20.0),
                                vec2(30.0, 30.0),
                            );
                            if rect.intersect(visible).is_empty() {
                                continue;
                            }
                            self.built += 1;
                            let fill = if self.kind == PanZoomCase::Update
                                && self.step.is_multiple_of(2)
                            {
                                Color::rgb(210, 85, 80)
                            } else {
                                Color::rgb(65, 130, 200)
                            };
                            ui.push_id(i, |ui| ui.paint(Shape::rect(rect, fill)));
                        }
                        if self.kind == PanZoomCase::Controls {
                            ui.at(
                                "controls",
                                Rect::from_min_size(Vec2::ZERO, vec2(200.0, 200.0)),
                                |ui| {
                                    ui.add(Slider::new(&mut self.slider, 0.0..=1.0).width(180.0));
                                    let scroll = ScrollArea::vertical()
                                        .id_source("nested")
                                        .max_height(100.0)
                                        .show(ui, |ui| {
                                            for i in 0..20 {
                                                ui.label(format!("Row {i}"));
                                            }
                                        });
                                    self.scroll = scroll.offset;
                                    self.scroll_id = Some(scroll.id);
                                    self.scroll_rect = scroll.viewport;
                                },
                            );
                        }
                    },
                ));
            })
        });
        self.out = Some(out.unwrap());
    }
    pub fn verify(&self, c: &Context) {
        assert!(
            self.consumed,
            "dispatcher did not consume an addressed gesture"
        );
        let Some(out) = &self.out else {
            assert_eq!(c.probe().camera_routing_counts, (0, 0));
            assert_eq!(c.probe().cache.len(), 0);
            return;
        };
        let expected_built = (0..self.count)
            .filter(|i| {
                let rect = Rect::from_min_size(
                    vec2((i % 32) as f32 * 40.0 - 20.0, (i / 32) as f32 * 40.0 - 20.0),
                    vec2(30.0, 30.0),
                );
                !rect.intersect(out.visible).is_empty()
            })
            .count();
        assert_eq!(self.built, expected_built);
        assert!(
            self.built <= self.count.min(176),
            "application culling did not bound construction"
        );
        assert_eq!(c.probe().camera_routing_counts, (1, 0));
        assert!(self.camera.scale.is_finite() && self.camera.translation.is_finite());
        assert_eq!(self.camera, out.camera);
        match self.kind {
            PanZoomCase::Pan => {
                let expected = if self.step.is_multiple_of(2) {
                    vec2(12.0, 8.0)
                } else {
                    vec2(-12.0, -8.0)
                };
                assert!(
                    (self.camera.translation - self.before.translation - expected).length() < 0.01
                );
                assert!(out.panned && !out.zoomed);
            }
            PanZoomCase::Zoom | PanZoomCase::Burst => {
                assert!(
                    (out.local_to_screen(c, self.anchored_local) - self.anchor).length() < 0.01
                );
                assert!(out.zoomed);
                if self.kind == PanZoomCase::Burst {
                    assert!((self.camera.scale - self.before.scale).abs() < 0.001);
                }
            }
            PanZoomCase::Controls => {
                assert_ne!(self.slider, self.before_slider);
                assert_ne!(self.scroll, self.before_scroll);
                assert!(!out.changed);
            }
            PanZoomCase::Idle => {
                assert_eq!(self.tessellations, c.cache_stats().tessellated_elements);
                assert_eq!(self.revision, c.draw_data().revision);
                assert!(!c.needs_repaint() && !c.wants_animation_frame());
            }
            PanZoomCase::Update => assert!(c.draw_data().revision > self.revision),
            PanZoomCase::Lifecycle => {}
        }
        assert!(c.probe().cache.len() <= self.built + 45);
    }
}
