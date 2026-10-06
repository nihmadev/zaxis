//! Which target takes the files. The regions are those the last pass published, so layers,
//! popups, scroll clips and transforms apply as for any input, and the winner follows the
//! rules of in-window drag and drop: the topmost window or popup under the position, the most
//! deeply nested target, the later one at equal depth, and a rejecting target ends the search
//! unless it hands the drop on.

use super::FileTarget;
use crate::{
    context::{drag::state::Hover, Context, HitAction},
    files::PickedFile,
    Rect, Vec2,
};

impl Context {
    pub(super) fn file_hover_target(&self) -> Option<Hover> {
        if self.files.hovered.is_empty() {
            return None;
        }
        self.file_resolve(&self.files.hovered, self.files.hover_pos)
    }

    pub(super) fn file_drop_target(&self) -> Option<Hover> {
        if self.files.dropped.is_empty() {
            return None;
        }
        self.file_resolve(&self.files.dropped, self.files.drop_pos)
    }

    /// The target `files` at `position` belong to. Without a position (the system gave none
    /// and the pointer never entered the window) only a lone accepting target can be named:
    /// anything else would be a guess.
    fn file_resolve(&self, files: &[PickedFile], position: Option<Vec2>) -> Option<Hover> {
        let window = match position {
            Some(pointer) => Some(self.top_window(pointer)?),
            None => self
                .popups
                .current
                .as_ref()
                .map(|popup| popup.id)
                .or_else(|| self.top_modal_id()),
        };
        let mut candidates: Vec<(u16, usize, Rect, &FileTarget)> = Vec::new();
        for (index, hit) in self.interaction.previous_hits.iter().enumerate() {
            let HitAction::FileDrop { slot } = hit.action else {
                continue;
            };
            if window.is_some_and(|window| hit.window != window)
                || position.is_some_and(|p| !hit.rect.contains(p) || !hit.clip.contains(p))
            {
                continue;
            }
            if let Some(info) = self.files.last_targets.get(slot as usize) {
                candidates.push((info.depth, index, hit.rect, info));
            }
        }
        let accepts = |info: &FileTarget| accepted(info, files) > 0;
        let local = |rect: Rect| position.map_or(Vec2::ZERO, |p| p - rect.min);
        if position.is_none() {
            let mut taking = candidates.iter().filter(|(_, _, _, info)| accepts(info));
            return match (taking.next(), taking.next()) {
                (Some(&(_, _, rect, info)), None) => Some(hover_of(info, rect, local(rect), true)),
                _ => None,
            };
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let mut rejected = None;
        for (_, _, rect, info) in candidates {
            if accepts(info) {
                return Some(hover_of(info, rect, local(rect), true));
            }
            rejected.get_or_insert_with(|| hover_of(info, rect, local(rect), false));
            if !info.passthrough {
                break;
            }
        }
        rejected
    }

    /// The files of a drop that `id` takes: those its filter passes, up to its limit.
    pub(crate) fn file_claim(&mut self, id: crate::Id) -> Option<Vec<PickedFile>> {
        let target = self.files.drop_target.filter(|t| t.id == id && t.accepts)?;
        if self.files.claimed {
            return None;
        }
        let info = self.files.last_targets.iter().find(|t| t.id == target.id)?;
        let (filter, max) = (info.filter.clone(), info.max);
        self.files.claimed = true;
        let files = std::mem::take(&mut self.files.dropped);
        Some(
            files
                .into_iter()
                .filter(|file| file.name().is_empty() || filter.accepts(file))
                .take(max)
                .collect(),
        )
    }
}

/// How many of `files` the target takes, up to its limit. A file still known only as a
/// placeholder (a browser drag) counts: its name arrives with the drop.
fn accepted(info: &FileTarget, files: &[PickedFile]) -> usize {
    files
        .iter()
        .filter(|file| file.name().is_empty() || info.filter.accepts(file))
        .count()
        .min(info.max)
}

fn hover_of(info: &FileTarget, rect: Rect, local: Vec2, accepts: bool) -> Hover {
    Hover {
        id: info.id,
        key: info.key,
        accepts,
        insertion: None,
        local,
        rect,
        effect: None,
    }
}
