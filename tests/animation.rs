use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
use zaxis::{animation::*, vec2, Color, Context, Id, Vec2};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn clock() -> (Context, Instant) {
    let context = Context::new();
    let now = context.frame_time() + ms(1000);
    (context, now)
}
fn read(ctx: &mut Context, now: Instant, id: Id) -> Animated<f32> {
    let mut value = None;
    ctx.run_at(now, |ctx| value = ctx.sample_animation(id));
    value.unwrap()
}

fn panic_track() -> Tween<f32> {
    panic!("factory restarted a retained channel")
}

// This type and spring live entirely outside zaxis; no private enum registration.
#[derive(Clone, Debug, PartialEq)]
struct Dial {
    angle: f32,
    radius: f32,
}
impl Interpolate for Dial {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        Self {
            angle: self.angle.interpolate(&to.angle, t),
            radius: self.radius.interpolate(&to.radius, t),
        }
    }
}
struct Spring {
    target: Dial,
}
impl Animation<Dial> for Spring {
    fn sample(&self, elapsed: Duration) -> AnimationSample<Dial> {
        if elapsed >= ms(1000) {
            return AnimationSample::completed(self.finish());
        }
        let t = elapsed.as_secs_f32();
        AnimationSample::running(Dial {
            angle: self.target.angle * (1.0 - (-8.0 * t).exp() * (12.0 * t).cos()),
            radius: self.target.radius,
        })
    }
    fn finish(&self) -> Dial {
        self.target.clone()
    }
}

#[path = "animation/cases_1.rs"]
mod cases_1;

#[path = "animation/cases_2.rs"]
mod cases_2;
