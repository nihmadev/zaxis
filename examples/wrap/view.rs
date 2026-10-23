//! The user interface: a wrapping filter bar and a wrapping group of cards.
use crate::model::{Edge, Flag, Model};
use zaxis::{
    vec2, Button, ButtonVariant, Card, Popup, Rect, Response, ScrollArea, SegmentedControl,
    TextEdit, Ui, Widget, Wrap, WrapOutput,
};

/// What the headless check looks for again: responses in build coordinates and the geometry
/// the wraps reported.
#[derive(Default)]
pub struct Seen {
    pub flags: Vec<(Flag, Response)>,
    pub tags: Vec<(String, Response)>,
    pub field: Option<Response>,
    pub add: Option<Response>,
    pub more: Option<Response>,
    pub clear: Option<Response>,
    pub bar: Option<WrapOutput<()>>,
    pub cards: Option<WrapOutput<()>>,
    pub card_rects: Vec<Rect>,
}

const CARD_WIDTH: f32 = 168.0;

pub fn show(ui: &mut Ui<'_>, model: &mut Model, seen: &mut Seen) {
    *seen = Seen::default();
    let height = ui.available_height();
    ScrollArea::vertical()
        .id_source("page")
        .max_height(height)
        .show(ui, |ui| {
            filter_bar(ui, model, seen);
            ui.add_space(6.0);
            alignment(ui, model);
            ui.add_space(6.0);
            cards(ui, model, seen);
        });
}

fn filter_bar(ui: &mut Ui<'_>, model: &mut Model, seen: &mut Seen) {
    let mut toggled = None;
    let mut removed = None;
    let mut added = false;
    let mut cleared = false;
    let bar = Wrap::new("filters").gap(6.0).show(ui, |ui| {
        for flag in Flag::ALL {
            ui.push_id(flag, |ui| {
                let button = ui.add(
                    Button::new(flag.label())
                        .selected(model.is_on(flag))
                        .tooltip(format!("Only {} tasks", flag.label().to_lowercase())),
                );
                if button.clicked() {
                    toggled = Some(flag);
                }
                seen.flags.push((flag, button));
            });
        }
        for (index, tag) in model.tags.iter().enumerate() {
            ui.item(("tag", tag), |ui| {
                let chip = ui.add(Button::new(format!("{tag} ×")).variant(ButtonVariant::Soft));
                if chip.clicked() {
                    removed = Some(index);
                }
                seen.tags.push((tag.clone(), chip));
            });
        }
        ui.item("add", |ui| {
            let field = ui.add(
                TextEdit::new(&mut model.draft)
                    .id_source("draft")
                    .placeholder("Add tag")
                    .width(110.0),
            );
            let add = ui.add(Button::new("Add"));
            added = add.clicked() || field.submitted();
            seen.field = Some(field);
            seen.add = Some(add);
        });
        ui.item("more", |ui| {
            let more = ui.add(Button::new("More").variant(ButtonVariant::Ghost));
            if more.clicked() {
                model.menu = !model.menu;
            }
            Popup::new("more-menu", more.rect)
                .size(vec2(150.0, 44.0))
                .show(ui, &mut model.menu, |ui| {
                    let clear = ui.add(Button::new("Clear filters").variant(ButtonVariant::Ghost));
                    cleared = clear.clicked();
                    seen.clear = Some(clear);
                });
            seen.more = Some(more);
        });
    });
    seen.bar = Some(bar);
    if let Some(flag) = toggled {
        model.toggle(flag);
    }
    if let Some(index) = removed {
        model.tags.remove(index);
    }
    if added {
        model.add_tag();
    }
    if cleared {
        model.clear();
        model.menu = false;
    }
}

fn alignment(ui: &mut Ui<'_>, model: &mut Model) {
    let items = [
        (Edge::Start, "Start"),
        (Edge::Middle, "Middle"),
        (Edge::End, "End"),
    ];
    let lines = [
        (Edge::Start, "Left"),
        (Edge::Middle, "Center"),
        (Edge::End, "Right"),
    ];
    Wrap::new("align").gap(10.0).show(ui, |ui| {
        ui.item("items", |ui| {
            ui.add(SegmentedControl::new(&mut model.item_align, items));
        });
        ui.item("lines", |ui| {
            ui.add(SegmentedControl::new(&mut model.line_align, lines));
        });
    });
}

fn cards(ui: &mut Ui<'_>, model: &mut Model, seen: &mut Seen) {
    let visible = model.visible();
    ui.muted(format!("{} of {}", visible.len(), model.tasks.len()));
    ScrollArea::vertical()
        .id_source("cards")
        .max_height(320.0)
        .show(ui, |ui| {
            let out = Wrap::new("cards")
                .spacing(vec2(8.0, 8.0))
                .align_items(model.item_align.align())
                .align_lines(model.line_align.align())
                .show(ui, |ui| {
                    for &index in &visible {
                        let task = &model.tasks[index];
                        ui.push_id(task.title, |ui| {
                            let card = Card::new("task").width(CARD_WIDTH).show(ui, |ui| {
                                ui.label(task.title);
                                if !task.note.is_empty() {
                                    ui.add(zaxis::Text::new(task.note).muted().wrap(true));
                                }
                            });
                            seen.card_rects.push(card.rect);
                        });
                    }
                });
            seen.cards = Some(out);
        });
}
