//! Default-theme reference: title fade, page switch, then a glass panel sliding out.
#![forbid(unsafe_code)]
use std::time::{Duration, Instant};
use zaxis::winit::keyboard::KeyCode;
use zaxis::{
    vec2, Animation, AnimationSample, App, Color, Context, Easing, Frame, Id, Rect, RunOptions,
    Shape, Text, Tween, Window,
};

const INTRO: &str = "motion-reference-intro";
const EXIT: &str = "motion-reference-exit";

#[derive(Clone, Debug, PartialEq)]
struct ExitPose {
    down: f32,
    blur: f32,
}

/// Application-defined motion of the whole library panel, not its text.
struct BlurExit {
    duration: Duration,
}
impl Animation<ExitPose> for BlurExit {
    fn sample(&self, elapsed: Duration) -> AnimationSample<ExitPose> {
        if elapsed >= self.duration {
            return AnimationSample::completed(self.finish());
        }
        let t = (elapsed.as_secs_f64() / self.duration.as_secs_f64()) as f32;
        AnimationSample::running(ExitPose {
            down: 600.0 * Easing::SineInOut.sample(((t - 0.2) / 0.8).max(0.0)),
            blur: 18.0 * Easing::QuadOut.sample((t / 0.6).min(1.0)),
        })
    }
    fn finish(&self) -> ExitPose {
        ExitPose {
            down: 600.0,
            blur: 18.0,
        }
    }
}

struct Demo {
    started: Option<Instant>,
    slow: bool,
    reduced: bool,
    smoke: bool,
    initialized: bool,
    completed: usize,
    frames: usize,
    fade_frames: usize,
    iteration: u64,
}
impl Demo {
    fn duration(&self, millis: u64) -> Duration {
        Duration::from_millis(millis * if self.slow { 3 } else { 1 })
    }
    fn intro(&self) -> Tween<f32> {
        Tween::new(0.0, 1.0, self.duration(1200)).easing(Easing::SineInOut)
    }
}
fn alpha(mut color: Color, opacity: f32) -> Color {
    color.0[3] = (f32::from(color.0[3]) * opacity.clamp(0.0, 1.0)).round() as u8;
    color
}
impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        let now = context.frame_time();
        if !self.initialized {
            // Keep every default theme field; only playback settings are optional.
            if self.slow || self.reduced {
                let mut style = context.style().clone();
                style.motion.page.duration = self.duration(280);
                style.motion.reduced_motion = self.reduced;
                context.set_style(style);
            }
            self.initialized = true;
        }
        if context.input().keys_pressed.contains(&KeyCode::Escape) {
            frame.close();
        }
        if context.input().keys_pressed.contains(&KeyCode::Space) {
            self.started = Some(now);
            self.iteration += 1;
            self.completed = 0;
            self.fade_frames = 0;
            context.restart_animation(Id::new(INTRO), self.intro());
            context.cancel_animation(Id::new(EXIT));
        }
        let elapsed = now.duration_since(*self.started.get_or_insert(now));
        let switch_at = self.duration(2200);
        let exit_at = self.duration(4200);
        let page = usize::from(elapsed >= switch_at);
        let fade = if page == 0 {
            context.animate(Id::new(INTRO), || self.intro()).value
        } else {
            1.0
        };
        if elapsed < self.duration(1200) {
            self.fade_frames += 1;
        }
        let mut exit = if elapsed >= exit_at {
            Some(context.animate(Id::new(EXIT), || BlurExit {
                duration: self.duration(1200),
            }))
        } else {
            None
        };
        if exit.as_ref().is_some_and(|motion| {
            motion.running() && 96.0 + motion.value.down >= context.viewport().max.y
        }) {
            // Once the whole panel is outside the viewport, apply the exact end
            // state and stop asking for invisible intermediate frames.
            context.finish_animation(Id::new(EXIT));
            exit = context.sample_animation(Id::new(EXIT));
        }
        if exit.as_ref().is_some_and(|motion| motion.just_completed) {
            self.completed += 1;
        }
        if elapsed < switch_at {
            context.request_repaint_after(switch_at - elapsed);
        } else if elapsed < exit_at {
            context.request_repaint_after(exit_at - elapsed);
        }

        // Neutral geometry behind the panel makes its backdrop blur observable.
        // All colors come from the unchanged default library theme.
        let fill = context.style().button_fill;
        for x in [80.0, 330.0, 580.0] {
            context.paint_background(
                Shape::rect(Rect::from_min_size(vec2(x, 50.0), vec2(72.0, 460.0)), fill)
                    .corner_radius(8.0),
            );
        }
        if !exit.as_ref().is_some_and(|motion| motion.completed()) {
            let (down, blur) = exit
                .as_ref()
                .map_or((0.0, 0.0), |motion| (motion.value.down, motion.value.blur));
            Window::new("Zaxis")
                .id(Id::new("motion-reference-window"))
                .default_position(vec2(125.0, 96.0))
                .default_size(vec2(610.0, 300.0))
                .draggable(false)
                .resizable(false)
                .offset(vec2(0.0, down))
                .blur(blur)
                .show(context, |ui| {
                    let width = ui.available_width();
                    let text_color = ui.style().text_color;
                    ui.tab_pages(
                        ("sequence", self.iteration),
                        page,
                        vec2(width, 220.0),
                        |ui, index| {
                            ui.add_space(62.0);
                            ui.horizontal(|ui| {
                                ui.add_space(64.0);
                                if index == 0 {
                                    // Fixed glyph positions: the intro changes alpha only.
                                    ui.add(
                                        Text::new("Zaxis UI Library")
                                            .size(48.0)
                                            .color(alpha(text_color, fade))
                                            .wrap(false),
                                    );
                                } else {
                                    ui.add(
                                        Text::new("Тестирование анимаций")
                                            .size(36.0)
                                            .color(text_color)
                                            .wrap(false),
                                    );
                                }
                            });
                        },
                    );
                });
        }
        if self.smoke && exit.as_ref().is_some_and(|motion| motion.completed()) {
            assert_eq!(
                exit.unwrap().value,
                BlurExit {
                    duration: self.duration(1200)
                }
                .finish()
            );
            assert_eq!(self.completed, 1);
            println!("motion reference smoke: {} frames, {} fade samples, exact panel exit, one completion event", self.frames, self.fade_frames);
            frame.close();
        }
        self.frames += 1;
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            started: None,
            slow: std::env::args().any(|arg| arg == "--slow"),
            reduced: std::env::args().any(|arg| arg == "--reduced-motion"),
            smoke: std::env::args().any(|arg| arg == "--smoke-test"),
            initialized: false,
            completed: 0,
            frames: 0,
            fade_frames: 0,
            iteration: 0,
        },
        RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("Zaxis animations")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(860.0, 560.0))
                .with_resizable(false),
            ..RunOptions::default()
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_exit_has_exact_endpoints_at_all_frame_rates() {
        let effect = BlurExit {
            duration: Duration::from_millis(1200),
        };
        let start = effect.sample(Duration::ZERO);
        assert_eq!(
            start.value,
            ExitPose {
                down: 0.0,
                blur: 0.0
            }
        );
        assert!(!start.completed);
        for hz in [30, 60, 144] {
            let mut previous = start.value.clone();
            for i in 1..=(hz * 2) {
                let sample = effect.sample(Duration::from_secs_f64(i as f64 / hz as f64));
                assert!(sample.value.down >= previous.down);
                assert!(sample.value.blur >= previous.blur);
                previous = sample.value;
            }
            assert_eq!(previous, effect.finish());
        }
    }
}
