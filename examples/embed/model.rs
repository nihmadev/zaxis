//! The host's state: what the interface edits and the scene draws.

pub struct Model {
    pub rotate: bool,
    pub speed: f32,
    pub size: f32,
    pub blur: f32,
    pub angle: f32,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            rotate: true,
            speed: 0.8,
            size: 1.0,
            blur: 12.0,
            angle: 0.6,
        }
    }
}

impl Model {
    /// Advance the animation by `seconds`.
    pub fn step(&mut self, seconds: f32) {
        if self.rotate {
            self.angle = (self.angle + self.speed * seconds) % std::f32::consts::TAU;
        }
    }
}
