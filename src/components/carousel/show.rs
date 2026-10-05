//! One pass of a carousel: model, frame, input, layers, controls and output.
use super::{
    api::{Carousel, CarouselOutput, CarouselPage},
    arrows,
    drive::{drive, Params},
    indicator::{self, Row},
    options::{CarouselIndicator, CarouselVariant, IndicatorPosition},
    scene::{Content, Painted, Scene},
    slides, stack,
    state::Model,
    style::Look,
};
use crate::{
    components::{Sense, Ui},
    context::invalid_value,
    Id, ImageSource, Rect, Shape, Vec2,
};

/// Fallback size when the layout offers none.
const FALLBACK: Vec2 = Vec2::new(480.0, 260.0);

impl Carousel {
    /// Show the pages built by `build(ui, page_index)`. For `Stack` each page is the content
    /// of a card; for `Images` it is the content of a slide.
    pub fn show<'a>(
        self,
        ui: &mut Ui<'_>,
        page: impl Into<CarouselPage<'a>>,
        mut build: impl FnMut(&mut Ui<'_>, usize),
    ) -> CarouselOutput {
        self.run(ui, page.into(), Content::Pages(&mut build))
    }

    /// Show photographs: `source(page_index)` names the image of a page, and the pages
    /// are laid out as the `Images` variant. Called only for visible pages.
    pub fn show_images<'a>(
        self,
        ui: &mut Ui<'_>,
        page: impl Into<CarouselPage<'a>>,
        mut source: impl FnMut(usize) -> ImageSource,
    ) -> CarouselOutput {
        let photos = Content::Photos {
            source: &mut source,
            overlay: None,
        };
        self.images().run(ui, page.into(), photos)
    }

    /// Like [`Self::show_images`] with content laid over the active photo, for example a
    /// title and subtitle. A scrim darkens the photo under it; `overlay` runs only for the
    /// visible pages, and only the active page takes input.
    pub fn show_images_with<'a>(
        self,
        ui: &mut Ui<'_>,
        page: impl Into<CarouselPage<'a>>,
        mut source: impl FnMut(usize) -> ImageSource,
        mut overlay: impl FnMut(&mut Ui<'_>, usize),
    ) -> CarouselOutput {
        let photos = Content::Photos {
            source: &mut source,
            overlay: Some(&mut overlay),
        };
        self.images().run(ui, page.into(), photos)
    }

    fn run(
        self,
        ui: &mut Ui<'_>,
        mut page: CarouselPage<'_>,
        mut content: Content<'_>,
    ) -> CarouselOutput {
        let source = self.id;
        ui.layout_item(|ui| ui.push_id(source, |ui| self.pass(ui, &mut page, &mut content)))
    }

    fn pass(
        mut self,
        ui: &mut Ui<'_>,
        page: &mut CarouselPage<'_>,
        content: &mut Content<'_>,
    ) -> CarouselOutput {
        let id = ui.scope;
        let mut style = ui.style().carousel.clone();
        style.merge(&self.style);
        let look = Look { style };
        let axis = self.orientation.axis();
        let images = self.variant == CarouselVariant::Images;
        let keys = self.keys.take().map(unique_keys);
        let count = keys.as_ref().map_or(self.count, Vec::len);

        // Frame: pages, an optional indicator strip, and the whole allocation.
        let kind = indicator::resolve(
            self.indicator,
            if images {
                CarouselIndicator::Dashes
            } else {
                CarouselIndicator::Pill
            },
            count,
        );
        let position = self.position.unwrap_or(if images {
            IndicatorPosition::Overlay
        } else {
            IndicatorPosition::After
        });
        let font = ui.style().font_size;
        let thickness = indicator::thickness(kind, &look, font);
        let strip = if kind == CarouselIndicator::None || position == IndicatorPosition::Overlay {
            0.0
        } else {
            thickness + look.gap()
        };
        let available = Vec2::new(ui.available_width(), ui.available_height());
        let width = self.width.unwrap_or(available.x).min(available.x.max(1.0));
        let width = if width.is_finite() { width } else { FALLBACK.x };
        let stage_height = if images {
            (width * if axis == 0 { 0.5 } else { 1.0 }).clamp(180.0, 420.0)
        } else {
            FALLBACK.y
        };
        let height = self
            .height
            .unwrap_or(stage_height + if axis == 0 { strip } else { 0.0 });
        let total = Vec2::new(width, height).max(Vec2::ONE);
        let rect = ui.allocate_space(total);
        let (stage, strip_rect) = split(rect, axis, strip, position, thickness, &look);

        let model = Model {
            count,
            keys: keys.as_deref(),
        };
        let root = ui.interact_id("pages");
        let enabled = self.enabled && ui.is_enabled();
        if count == 0 {
            invalid_value("Carousel", "no pages to show".into());
            ui.context.containers.carousels.remove(&id);
            let response = ui.interact(stage, "pages", Sense::HOVER);
            return CarouselOutput {
                response,
                page: 0,
                key: model.key(0),
                changed: false,
                autoplayed: false,
                progress: 1.0,
                dragging: false,
                activated: None,
            };
        }
        if let CarouselPage::Index(index) = page {
            if **index >= count {
                invalid_value(
                    "Carousel",
                    format!("page {} is out of range for {count} pages", **index),
                );
            }
        }
        let wrap = self.wrap && count > 1;
        let mut state = ui
            .context
            .containers
            .carousels
            .remove(&id)
            .unwrap_or_default();
        state.last_frame = ui.context.frame;
        let synced = state.sync(&model, page, wrap, self.initial);
        if synced.external {
            state.autoplay_at = None;
        }

        let area = stage.shrink(look.padding().min(stage.size().min_element() * 0.25));
        let extent = if images {
            slides::geometry(area, &look, count, axis).slide.size()[axis]
        } else {
            stack::geometry(area, &look, axis).card.size()[axis]
        };
        let response = ui.interact(stage, "pages", Sense::CLICK | Sense::DRAG | Sense::FOCUS);
        if self.wheel && enabled {
            ui.context.carousel_wheel_register(root, axis);
        }
        let params = Params {
            id,
            root,
            look: &look,
            wrap,
            axis,
            extent: extent * look.travel(),
            rect,
            autoplay: self.autoplay,
            enabled,
            wheel: self.wheel,
            spring: look.style.spring.unwrap_or(ui.style().motion.spring),
        };
        let mut driven = drive(ui, &params, &mut state, &model, response, synced.shift);
        let region = super::access::Region {
            label: &self.label,
            page: state.page,
            count,
            axis,
            enabled,
        };
        let access = super::access::begin(ui, root, region);

        // Layers, back to front.
        let last = count as f32 - 1.0;
        let (shown, over) = if wrap {
            (driven.position, 0.0)
        } else {
            let inside = driven.position.clamp(0.0, last);
            (inside, driven.position - inside)
        };
        let scene = Scene {
            id,
            stage,
            area,
            position: shown,
            over,
            target: state.target,
            from: state.from,
            model: &model,
            wrap,
            axis,
            look: &look,
            dragging: state.drag.is_some(),
            enabled,
        };
        let painted: Painted = match content {
            Content::Pages(build) if !images => stack::paint(ui, &scene, &mut **build),
            content => slides::paint(ui, &scene, content),
        };

        // Controls above the layers. Their clicks apply from the next pass on.
        let mut late = None;
        if self.arrows && count > 1 {
            let can = [wrap || state.page > 0, wrap || state.page + 1 < count];
            let step = arrows::show(ui, id, stage, axis, can, enabled, &look);
            late = step.map(|step| state.target + step);
        }
        if kind != CarouselIndicator::None {
            let row = Row {
                id: id.with("indicator"),
                kind,
                rect: strip_rect.unwrap_or(overlay_rect(area, axis, thickness)),
                axis,
                position: driven.position,
                count,
                wrap,
                look: &look,
                over_media: images && position == IndicatorPosition::Overlay,
                enabled,
            };
            if let Some(clicked) = indicator::show(ui, &row).filter(|_| enabled) {
                let step = super::motion::distance(state.page, clicked, count, wrap);
                late = Some(state.target + step);
            }
        }
        if let Some(target) = late.filter(|_| enabled) {
            if state.go(target, &model, wrap) {
                driven.changed = true;
                ui.context.request_repaint();
            }
        }
        if response.focus_visible && enabled {
            let ring = look.style.focus_ring.unwrap_or(ui.style().focus_border);
            let grow = Vec2::splat(ring.width.max(0.0) * 0.5 + 1.5);
            let outline = Rect::from_min_max(painted.front.min - grow, painted.front.max + grow);
            ui.paint(
                Shape::rect(outline, crate::Color::TRANSPARENT)
                    .corner_radius(look.rounding() + 2.0)
                    .border(ring),
            );
        }

        ui.a11y_end(access, Some(rect));

        // Write the page back, then report.
        let key = model.key(state.page);
        match page {
            CarouselPage::Index(index) => **index = state.page,
            CarouselPage::Key(slot) => **slot = key,
            CarouselPage::Internal => {}
        }
        state.layers = painted.layers;
        let progress = (1.0 - (driven.position - state.target as f32).abs()).clamp(0.0, 1.0);
        let dragging = state.drag.is_some();
        ui.context.containers.carousels.insert(id, state);
        let mut response = response;
        if driven.changed {
            response.mark_changed();
        }
        CarouselOutput {
            response,
            page: ui.context.containers.carousels[&id].page,
            key,
            changed: driven.changed,
            autoplayed: driven.autoplayed,
            progress,
            dragging,
            activated: driven.activated,
        }
    }
}

/// Make equal keys unique by their index so that two pages never share content state.
fn unique_keys(keys: Vec<Id>) -> Vec<Id> {
    let mut seen = std::collections::HashSet::new();
    let mut repeated = 0;
    let keys = keys
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            if seen.insert(key) {
                key
            } else {
                repeated += 1;
                Id::new((key, i))
            }
        })
        .collect();
    if repeated > 0 {
        invalid_value(
            "Carousel::keys",
            format!("{repeated} keys repeat an earlier one; made unique"),
        );
    }
    keys
}

/// Split the allocation into the pages area and the indicator strip.
fn split(
    rect: Rect,
    axis: usize,
    strip: f32,
    position: IndicatorPosition,
    thickness: f32,
    look: &Look,
) -> (Rect, Option<Rect>) {
    if strip <= 0.0 {
        return (rect, None);
    }
    let cross = 1 - axis;
    let (mut stage, mut bar) = (rect, rect);
    if position == IndicatorPosition::Before {
        stage.min[cross] += strip;
        bar.max[cross] = rect.min[cross] + thickness;
    } else {
        stage.max[cross] -= strip;
        bar.min[cross] = rect.max[cross] - thickness;
    }
    let _ = look;
    (stage, Some(bar))
}

/// Indicator rectangle over the pages, near the trailing edge.
fn overlay_rect(area: Rect, axis: usize, thickness: f32) -> Rect {
    let margin = 10.0;
    let mut bar = area;
    let cross = 1 - axis;
    bar.min[cross] = area.max[cross] - margin - thickness;
    bar.max[cross] = area.max[cross] - margin;
    bar
}
