//! Paint descriptions to cached meshes and this pass's elements.
//!
//! One paint call goes through the same stages every time: routing (`route`), visual
//! materialization (`visual`), identity (a repeated id draws under an alias), resource
//! requests, reuse of the cached mesh (`cache`) or tessellation (`tessellate`), and
//! emission of the element. The data lives in [`PaintState`] (`state`).

mod cache;
mod data;
mod route;
mod state;
mod tessellate;
mod visual;

use super::{Context, Id};
use crate::Rect;
use cache::Reuse;

pub use data::{MaterialImage, Paint};
pub use state::CachedElement;
pub(crate) use state::PaintState;
pub use visual::VisualMesh;

impl Context {
    pub(crate) fn paint(&mut self, id: Id, layer: Id, clip: Rect, paint: Vec<Paint>) {
        let Some(paint) = self.route_paint(id, layer, clip, paint) else {
            return;
        };
        let paint = match visual::flatten(paint) {
            Ok(visual) => return self.paint_visual(id, layer, clip, visual),
            Err(paint) => paint,
        };
        self.paint_state.drop_visual(id);
        let id = self.claim_paint_id(id);
        let paint_scale = self.request_painted_images(&paint, clip);
        let frame = self.frame;
        let reuse =
            self.paint_state
                .reuse(id, &paint, clip, paint_scale, self.scale, &mut self.text);
        match reuse {
            Reuse::Hidden => self.paint_state.touch(id, frame),
            Reuse::Translated(delta) => {
                self.paint_state.translate(id, delta, paint, frame);
                self.stats.reused_elements += 1;
                self.paint_state.emit(id, layer, clip, false);
            }
            Reuse::Unchanged => {
                self.stats.reused_elements += 1;
                self.paint_state.touch(id, frame);
                self.paint_state.emit(id, layer, clip, true);
            }
            Reuse::Rebuild => {
                let mesh = tessellate::mesh(&mut self.text, &paint, self.scale, paint_scale);
                self.paint_state.store(id, paint, paint_scale, mesh, frame);
                self.stats.tessellated_elements += 1;
                self.paint_state.emit(id, layer, clip, true);
            }
        }
    }

    /// The id this paint is drawn under: its own, or an alias when it was already painted
    /// in this pass, which is reported.
    fn claim_paint_id(&mut self, id: Id) -> Id {
        match self.paint_state.claim(id) {
            Ok(id) => id,
            Err(alias) => {
                self.report_paint_collision(id);
                alias
            }
        }
    }
}
