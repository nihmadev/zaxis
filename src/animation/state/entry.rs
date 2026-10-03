use super::*;
impl<T: Clone + 'static> Entry for Slot<T> {
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn begin_pass(&mut self) {
        self.deadline = None;
        self.continuous = false;
    }
    fn last_frame(&self) -> u64 {
        self.last_frame
    }
    fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
    fn wants_frame(&self) -> bool {
        self.continuous
    }
    fn status(&self) -> AnimationStatus {
        self.status
    }
    fn control(&mut self, action: Control, now: Instant) {
        let was_active = self.track.is_some();
        self.evaluate(now, false);
        match action {
            Control::Pause if self.status == AnimationStatus::Running => {
                self.elapsed = self.elapsed_at(now);
                self.status = AnimationStatus::Paused;
            }
            Control::Resume if self.status == AnimationStatus::Paused => {
                self.started = now;
                self.sampled_at = None;
                self.status = AnimationStatus::Running;
            }
            Control::Cancel if was_active => {
                self.track = None;
                self.status = AnimationStatus::Cancelled;
                self.completion = false;
            }
            Control::Finish if self.track.is_some() => {
                self.complete(self.track.as_ref().unwrap().finish())
            }
            _ => {}
        }
        self.deadline = None;
        self.continuous = false;
    }
    fn reduce_motion(&mut self, now: Instant) {
        self.evaluate(now, true);
    }
}
