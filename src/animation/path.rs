use super::{Animation, AnimationSample, Easing, Repeat, Tween, TweenOptions};
use crate::Vec2;
use std::time::Duration;

/// Segments each curve is flattened into. Curves are cheap to follow and the
/// polyline error is far below a pixel for UI-sized paths.
const CURVE_STEPS: usize = 32;

/// A 2D path of lines and Bézier curves, flattened once into a polyline with
/// cumulative arc lengths. Positions are parameterised by **distance**, so a
/// linear fraction moves at constant speed however the control points are placed.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    points: Vec<Vec2>,
    /// `lengths[i]` is the distance from the start to `points[i]`.
    lengths: Vec<f32>,
}

impl Path {
    pub fn new(start: Vec2) -> Self {
        assert!(start.is_finite(), "path points must be finite");
        Self {
            points: vec![start],
            lengths: vec![0.0],
        }
    }
    fn push(&mut self, point: Vec2) {
        assert!(point.is_finite(), "path points must be finite");
        let last = *self.points.last().unwrap();
        let step = point.distance(last);
        // Repeated points would only create zero-length segments with no direction.
        if step > 0.0 {
            self.lengths.push(self.lengths.last().unwrap() + step);
            self.points.push(point);
        }
    }
    pub fn line_to(mut self, to: Vec2) -> Self {
        self.push(to);
        self
    }
    pub fn quad_to(mut self, control: Vec2, to: Vec2) -> Self {
        let from = *self.points.last().unwrap();
        for i in 1..=CURVE_STEPS {
            let t = i as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            self.push(from * (u * u) + control * (2.0 * u * t) + to * (t * t));
        }
        self
    }
    pub fn cubic_to(mut self, control_a: Vec2, control_b: Vec2, to: Vec2) -> Self {
        let from = *self.points.last().unwrap();
        for i in 1..=CURVE_STEPS {
            let t = i as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            self.push(
                from * (u * u * u)
                    + control_a * (3.0 * u * u * t)
                    + control_b * (3.0 * u * t * t)
                    + to * (t * t * t),
            );
        }
        self
    }
    pub fn length(&self) -> f32 {
        *self.lengths.last().unwrap()
    }
    /// The flattened polyline, for drawing the whole path.
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }
    /// Index of the segment containing distance `d` and the position within it.
    fn locate(&self, fraction: f32) -> (usize, f32) {
        let d = fraction.clamp(0.0, 1.0) * self.length();
        let end = self.lengths.partition_point(|&l| l < d).max(1);
        let end = end.min(self.points.len() - 1);
        let (start_length, end_length) = (self.lengths[end - 1], self.lengths[end]);
        (
            end - 1,
            ((d - start_length) / (end_length - start_length)).clamp(0.0, 1.0),
        )
    }
    /// Position after `fraction` (0 to 1) of the total length.
    pub fn point_at(&self, fraction: f32) -> Vec2 {
        if self.points.len() < 2 {
            return self.points[0];
        }
        let (segment, along) = self.locate(fraction);
        self.points[segment].lerp(self.points[segment + 1], along)
    }
    /// Direction of travel in radians (`atan2(dy, dx)`); 0 for a single point.
    pub fn angle_at(&self, fraction: f32) -> f32 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let (segment, _) = self.locate(fraction);
        let direction = self.points[segment + 1] - self.points[segment];
        direction.y.atan2(direction.x)
    }
    pub fn pose_at(&self, fraction: f32) -> PathPose {
        PathPose {
            position: self.point_at(fraction),
            angle: self.angle_at(fraction),
        }
    }
    /// The part of the path between two fractions as a polyline, for stroke
    /// "draw-on" effects: animate `to` from 0 to 1 and draw the result.
    pub fn trimmed(&self, from: f32, to: f32) -> Vec<Vec2> {
        let (from, to) = (from.clamp(0.0, 1.0), to.clamp(0.0, 1.0));
        let mut out = vec![self.point_at(from)];
        if to <= from || self.points.len() < 2 {
            return out;
        }
        let (low, high) = (from * self.length(), to * self.length());
        for (point, length) in self.points.iter().zip(&self.lengths) {
            if *length > low && *length < high {
                out.push(*point);
            }
        }
        out.push(self.point_at(to));
        out
    }
}

/// Where something following a [`Path`] is, and which way it faces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPose {
    pub position: Vec2,
    /// Direction of travel in radians.
    pub angle: f32,
}
crate::impl_interpolate!(PathPose { position, angle });

/// Moves along a path over time with any tween easing, delay, repeat or
/// auto-reverse; completes exactly on the pose at the end of the path.
pub struct PathFollow {
    path: Path,
    progress: Tween<f32>,
}
impl PathFollow {
    pub fn new(path: Path, duration: Duration) -> Self {
        Self::with_options(path, TweenOptions::new(duration))
    }
    pub fn with_options(path: Path, options: TweenOptions) -> Self {
        Self {
            path,
            progress: Tween::with_options(0.0, 1.0, options),
        }
    }
    pub fn easing(mut self, easing: Easing) -> Self {
        self.progress = self.progress.easing(easing);
        self
    }
    pub fn delay(mut self, delay: Duration) -> Self {
        self.progress = self.progress.delay(delay);
        self
    }
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.progress = self.progress.repeat(repeat);
        self
    }
    pub fn auto_reverse(mut self, enabled: bool) -> Self {
        self.progress = self.progress.auto_reverse(enabled);
        self
    }
}
impl Animation<PathPose> for PathFollow {
    fn sample(&self, elapsed: Duration) -> AnimationSample<PathPose> {
        let sample = self.progress.sample(elapsed);
        AnimationSample {
            value: self.path.pose_at(sample.value),
            completed: sample.completed,
            wake: sample.wake,
        }
    }
    fn finish(&self) -> PathPose {
        self.path.pose_at(self.progress.finish())
    }
    fn duration(&self) -> Option<Duration> {
        self.progress.duration()
    }
}
