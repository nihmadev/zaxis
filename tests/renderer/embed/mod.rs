//! Embedding in a host-owned device: pixels of `EmbeddedRenderer` read back from a real
//! device and compared with an independent recording of the same draw data. Skipped, with a
//! message, when there is no adapter or `ZAXIS_SKIP_GPU_TESTS` is set.

mod blur;
mod harness;
mod lifecycle;
mod pan_zoom;
mod pass;
mod reference;
mod region;
mod scene;
mod shared;
