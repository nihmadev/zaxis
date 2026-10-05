use crate::{features, footer, hero, reveal::Reveal, scroll::SmoothScroll};
use zaxis::{vec2, App, Context, Frame, Padding, Root, ScrollArea, Ui};

/// Page margin on narrow screens; wide ones center a column of at most `MAX_WIDTH`.
const MARGIN: f32 = 24.0;
const MAX_WIDTH: f32 = 1040.0;

pub struct Landing {
    scroll: SmoothScroll,
    reveal: Reveal,
    smoke: Option<u32>,
}

impl Landing {
    pub fn new(smoke: bool) -> Self {
        Self {
            scroll: SmoothScroll::default(),
            reveal: Reveal::default(),
            smoke: smoke.then_some(0),
        }
    }

    fn page(&mut self, ui: &mut Ui<'_>) {
        let height = ui.available_height();
        let mut area = ScrollArea::vertical()
            .id_source("landing")
            .max_height(height);
        if let Some(offset) = self.scroll.offset(ui) {
            area = area.scroll_offset(vec2(0.0, offset));
        }
        let Self { scroll, reveal, .. } = self;
        let output = area.show(ui, |ui| {
            column(ui, |ui| {
                let explore = hero::show(ui, height);
                let features = features::show(ui, reveal);
                footer::show(ui);
                (explore, features)
            })
        });
        let (explore, features_y) = output.inner;
        if explore {
            // The heading's screen position at the current offset, as content coordinates.
            let target = features_y - output.viewport.min.y + output.offset.y;
            let farthest = (output.content_size.y - output.viewport.size().y).max(0.0);
            scroll.start(output.offset.y, target.clamp(0.0, farthest));
            ui.context().request_repaint();
        }
    }
}

/// A centered column with side margins, filling the scroll area's width.
fn column<R>(ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
    let available = ui.available_width();
    let width = (available - 2.0 * MARGIN).clamp(0.0, MAX_WIDTH);
    let side = ((available - width) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(side);
        ui.vertical(|ui| ui.with_width(width, build))
    })
}

impl App for Landing {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Root::new()
            .padding(Padding::all(0.0))
            .show(context, |ui| self.page(ui));
        if let Some(passes) = &mut self.smoke {
            *passes += 1;
            if *passes >= 3 {
                frame.close();
            } else {
                context.request_repaint();
            }
        }
    }
}
