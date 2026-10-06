//! Material scenarios against the plain-rectangle baseline: many widgets with one material,
//! many materials, animated parameters and materials in a scroll area. Assertions run outside
//! the timed pass and use only the public API.
use std::time::Duration;
use zaxis::winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent},
};
use zaxis::Instant;
use zaxis::{
    vec2, Color, Context, Material, MaterialId, ParamKind, Params, Rect, Root, ScrollArea, Shape,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialCase {
    /// Plain rounded rectangles in the same grid: what a material draw is compared with.
    Baseline,
    /// Every widget draws the same material with the same parameters: one merged command.
    Same,
    /// Widgets cycle through many materials: a command and a pipeline per material.
    Many,
    /// One material, every parameter changing on every step: no tessellation, new uniforms.
    Animated,
    /// Rows of a virtual list, each a material, under the wheel.
    Scroll,
}

impl MaterialCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Baseline => "material_baseline_rects",
            Self::Same => "material_same",
            Self::Many => "material_many",
            Self::Animated => "material_params_animated",
            Self::Scroll => "material_scroll",
        }
    }
    pub fn interactive(self) -> bool {
        matches!(self, Self::Animated | Self::Scroll)
    }
}

const MATERIALS: usize = 32;
const COLUMNS: usize = 60;

pub struct Probe {
    kind: MaterialCase,
    count: usize,
    ids: Vec<MaterialId>,
    now: Instant,
    phase: f32,
    revision: u64,
    tessellations: u64,
    copied: u64,
    step: usize,
}

fn shader(n: usize) -> Material {
    Material::new(
        "bench",
        format!(
            "fn material(in: MaterialInput, p: Params) -> vec4<f32> {{ \
             return vec4<f32>(in.uv * p.phase, {n}.0 / 255.0, 1.0); }}"
        ),
    )
    .param("phase", ParamKind::F32)
}

impl Probe {
    pub fn new(context: &mut Context, kind: MaterialCase, count: usize) -> Self {
        let mut style = context.style().clone();
        style.motion.reduced_motion = true;
        context.set_style(style);
        let materials = if kind == MaterialCase::Many {
            MATERIALS
        } else {
            1
        };
        let ids = (0..materials)
            .map(|n| context.register_material(&shader(n)))
            .collect();
        let mut probe = Self {
            kind,
            count,
            ids,
            now: Instant::now(),
            phase: 0.5,
            revision: 0,
            tessellations: 0,
            copied: 0,
            step: 0,
        };
        for _ in 0..4 {
            probe.build(context);
        }
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.revision = context.draw_data().revision;
        let stats = context.cache_stats();
        self.tessellations = stats.tessellated_elements;
        self.copied = stats.geometry_bytes_copied;
        match self.kind {
            MaterialCase::Animated => self.phase = 0.25 + (step % 7) as f32 * 0.1,
            MaterialCase::Scroll => {
                let scale = f64::from(context.scale_factor());
                let center = context.viewport().center();
                context.on_window_event(&WindowEvent::CursorMoved {
                    device_id: DeviceId::dummy(),
                    position: PhysicalPosition::new(
                        f64::from(center.x) * scale,
                        f64::from(center.y) * scale,
                    ),
                });
                let direction = if step.is_multiple_of(2) { -1.0 } else { 1.0 };
                context.on_window_event(&WindowEvent::MouseWheel {
                    device_id: DeviceId::dummy(),
                    delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                        0.0,
                        direction * 48.0 * scale,
                    )),
                    phase: TouchPhase::Moved,
                });
            }
            _ => {}
        }
    }

    pub fn build(&mut self, context: &mut Context) {
        let (kind, count, phase) = (self.kind, self.count, self.phase);
        let ids = self.ids.clone();
        self.now += Duration::from_millis(1);
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                if kind == MaterialCase::Scroll {
                    ScrollArea::vertical()
                        .id_source("rows")
                        .max_height(360.0)
                        .show_rows(ui, 28.0, count, |ui, n| {
                            let rect = ui.allocate_space(vec2(300.0, 24.0));
                            ui.material(rect, ids[0])
                                .id_source(n)
                                .params(Params::new().f32("phase", 0.5))
                                .corner_radius(6.0)
                                .show(ui);
                        });
                    return;
                }
                let origin = ui.allocate_space(vec2(0.0, 0.0)).min + vec2(0.0, 8.0);
                for n in 0..count {
                    let cell = vec2((n % COLUMNS) as f32 * 14.0, (n / COLUMNS) as f32 * 14.0);
                    let rect = Rect::from_min_size(origin + cell, vec2(12.0, 12.0));
                    if kind == MaterialCase::Baseline {
                        ui.paint(Shape::rect(rect, Color::rgb(90, 140, 220)).corner_radius(3.0));
                    } else {
                        ui.material(rect, ids[n % ids.len()])
                            .id_source(n)
                            .params(Params::new().f32("phase", phase))
                            .corner_radius(3.0)
                            .show(ui);
                    }
                }
            });
        });
    }

    pub fn verify(&self, context: &Context) {
        let data = context.draw_data();
        let commands = data
            .commands
            .iter()
            .filter(|c| c.material.is_some())
            .count();
        let stats = context.cache_stats();
        match self.kind {
            MaterialCase::Baseline => assert_eq!(commands, 0),
            MaterialCase::Same => {
                assert_eq!(commands, 1, "equal draws merge into one command");
                assert_eq!(data.materials.len(), 1);
                assert_eq!(data.revision, self.revision);
                assert_eq!(stats.tessellated_elements, self.tessellations);
                assert!(!context.needs_repaint() && !context.wants_animation_frame());
            }
            MaterialCase::Many => {
                let expected = self.count.min(MATERIALS);
                assert_eq!(data.materials.len(), expected);
                assert!(
                    commands >= expected,
                    "{commands} commands for {expected} materials"
                );
                assert_eq!(stats.tessellated_elements, self.tessellations);
            }
            MaterialCase::Animated => {
                assert_eq!(
                    stats.tessellated_elements, self.tessellations,
                    "no mesh is rebuilt"
                );
                assert_eq!(
                    stats.geometry_bytes_copied, self.copied,
                    "no vertex is copied"
                );
                assert!(
                    data.revision > self.revision,
                    "the new uniforms are a new revision"
                );
                assert_eq!(commands, 1);
            }
            MaterialCase::Scroll => {
                assert!(commands >= 1 && !data.material_uniforms.is_empty());
                assert_eq!(data.materials.len(), 1);
            }
        }
    }
}
