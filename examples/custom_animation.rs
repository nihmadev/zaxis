//! Application-defined value, interpolation, easing and analytic spring.
#![forbid(unsafe_code)]
use std::time::Duration;
use zaxis::Instant;
use zaxis::{
    vec2, App, Border, Color, Context, Easing, Frame, Shape, SpringOptions, SpringState,
    SpringValue, TweenOptions, Vec2, Window,
};

#[derive(Clone, Debug, PartialEq)]
struct Pose {
    offset: Vec2,
    angle: f32,
}
zaxis::impl_interpolate!(Pose { offset, angle });
zaxis::impl_spring_value!(Pose { offset, angle });

struct Demo {
    reverse: bool,
    visible: bool,
    completions: usize,
    started: Option<Instant>,
    smoke: bool,
}
impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        let now = context.frame_time();
        let elapsed = now.duration_since(*self.started.get_or_insert(now));
        let mut reduced = context.style().motion.reduced_motion;
        Window::new("Application spring")
            .default_position(vec2(28.0, 24.0))
            .default_size(vec2(700.0, 490.0))
            .show(context, |ui| {
                ui.label("Application Pose extends Interpolate and SpringValue; the engine preserves velocity.");
                ui.checkbox(&mut reduced, "Reduced motion");
                ui.checkbox(&mut self.visible, "Show animated preview");
                if self.visible {
                    let id = ui.animation_id("spring");
                    let origin = Pose {
                        offset: Vec2::ZERO,
                        angle: 0.0,
                    };
                    let target = Pose {
                        offset: vec2(200.0, 0.0),
                        angle: std::f32::consts::PI,
                    };
                    let motion = ui.spring_transition_from("spring",SpringState {
                        value: origin.clone(),velocity: Pose::zero(),
                    }, if self.reverse { origin } else { target },SpringOptions::default());
                    if motion.just_completed {
                        self.completions += 1;
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Retarget").clicked() {
                            self.reverse = !self.reverse;
                        }
                        if ui.button("Pause").clicked() {
                            ui.context().pause_animation(id);
                        }
                        if ui.button("Resume").clicked() {
                            ui.context().resume_animation(id);
                        }
                        if ui.button("Cancel").clicked() {
                            ui.context().cancel_animation(id);
                        }
                    });
                    let color = ui
                        .transition(
                            "spring-color",
                            if self.reverse {
                                Color::rgb(191, 118, 87)
                            } else {
                                Color::rgb(87, 146, 206)
                            },
                            TweenOptions::new(Duration::from_millis(160))
                                .easing(Easing::custom(|t| t * t * (3.0 - 2.0 * t))),
                        )
                        .value;
                    let rect = ui.allocate_space(vec2(500.0, 170.0));
                    let center = rect.min + vec2(100.0, 85.0) + motion.value.value.offset;
                    ui.paint(Shape::Circle {
                        center,
                        radius: 30.0,
                        fill: color,
                        border: Border::NONE,
                    });
                    let direction = vec2(motion.value.value.angle.cos(), motion.value.value.angle.sin()) * 22.0;
                    ui.paint(Shape::Line {
                        start: center,
                        end: center + direction,
                        width: 3.0,
                        color: Color::WHITE,
                    });
                    let status = ui.context().animation_status(id).unwrap();
                    ui.label(format!(
                        "{status:?}; completion events: {}",
                        self.completions
                    ));
                }
            });
        if reduced != context.style().motion.reduced_motion {
            let mut style = context.style().clone();
            style.motion.reduced_motion = reduced;
            context.set_style(style);
        }
        if self.smoke && elapsed >= Duration::from_millis(1000) {
            assert_eq!(self.completions, 1);
            println!("custom_animation smoke: exact spring finish, one completion event");
            frame.close();
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run(Demo {
        reverse: false,
        visible: true,
        completions: 0,
        started: None,
        smoke: std::env::args().any(|arg| arg == "--smoke-test"),
    })
}
