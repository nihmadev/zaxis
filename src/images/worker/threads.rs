//! Worker threads for desktop targets.

use super::{process, wake, Shared, Workers};
use crate::images::ImageError;
use std::sync::{atomic::Ordering, Arc};

impl Workers {
    pub(super) fn spawn_threads(&mut self) -> Result<(), ImageError> {
        if self.threads.is_empty() {
            for i in 0..2 {
                let shared = self.shared.clone();
                self.threads.push(
                    std::thread::Builder::new()
                        .name(format!("zaxis image {i}"))
                        .spawn(move || run(shared))
                        .map_err(|e| ImageError(format!("image worker: {e}")))?,
                );
            }
        }
        Ok(())
    }
}

fn run(shared: Arc<Shared>) {
    loop {
        let job = {
            let mut jobs = shared.jobs.lock().unwrap();
            while jobs.is_empty() && shared.alive.load(Ordering::Acquire) {
                jobs = shared.changed.wait(jobs).unwrap();
            }
            if !shared.alive.load(Ordering::Acquire) {
                return;
            }
            jobs.pop_front().unwrap()
        };
        let completion = process(job);
        if !shared.alive.load(Ordering::Acquire) {
            return;
        }
        {
            let mut results = shared.results.lock().unwrap();
            results.push(completion);
            shared.ready.store(true, Ordering::Release);
        }
        wake(&shared);
    }
}
