//! Real input through `Context::on_input`, no window. Every step checks the model the
//! input should have changed and the geometry the wraps reported.
use crate::{model::Edge, Demo};
use zaxis::{
    winit::{
        dpi::PhysicalSize,
        event::{ElementState, MouseButton},
    },
    Context, Rect, Response, Vec2,
};
use zaxis::{InputEvent, Theme};

type Check = Result<(), String>;

fn check(ok: bool, what: &str, got: impl std::fmt::Debug) -> Check {
    ok.then_some(())
        .ok_or_else(|| format!("{what}: got {got:?}"))
}

/// Passes until the layout, which depends on the previous pass, stops changing.
fn settle(app: &mut Demo, context: &mut Context) {
    for _ in 0..4 {
        context.set_theme(Theme::dark());
        context.run(|c| app.view(c));
    }
}

fn shown(context: &Context, response: &Response) -> Rect {
    context.visual_rect(response.id, response.rect)
}

fn click(app: &mut Demo, context: &mut Context, at: Vec2) {
    let physical = at * context.scale_factor();
    context.on_input(InputEvent::PointerMoved {
        x: physical.x as f64,
        y: physical.y as f64,
    });
    for state in [ElementState::Pressed, ElementState::Released] {
        context.on_input(InputEvent::Button {
            button: MouseButton::Left,
            state,
        });
        settle(app, context);
    }
}

/// A task that contains at least one of the tags, written out independently of the model.
fn mentions(tags: &[String], task: &crate::model::Task) -> bool {
    let text = format!("{} {}", task.title, task.note).to_lowercase();
    tags.iter().any(|tag| text.contains(tag))
}

fn lines(items: &[Rect]) -> usize {
    let mut rows: Vec<i32> = items.iter().map(|r| r.min.y.round() as i32).collect();
    rows.dedup();
    rows.len()
}

pub fn run() -> Check {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(900, 700), 1.0);
    let mut app = Demo::default();
    settle(&mut app, &mut context);
    let all = app.model.tasks.len();
    let tagged = app.model.visible().len();
    check(tagged < all, "the starting tags already filter", tagged)?;
    check(
        app.seen.card_rects.len() == tagged,
        "one card per visible task",
        app.seen.card_rects.len(),
    )?;
    let wide = lines(&app.seen.bar.as_ref().ok_or("no filter bar")?.items);

    // A toggle changes the model and the cards that match.
    let (_, bug) = app.seen.flags[1];
    let at = shown(&context, &bug).center();
    click(&mut app, &mut context, at);
    check(app.model.active.len() == 1, "Bug is on", &app.model.active)?;
    let expected = app
        .model
        .tasks
        .iter()
        .filter(|t| t.bug && mentions(&app.model.tags, t))
        .count();
    check(
        app.seen.card_rects.len() == expected,
        "cards match the filters",
        (app.seen.card_rects.len(), expected),
    )?;

    // A chip on the second line removes its own tag, once.
    let tags = app.model.tags.len();
    let (name, chip) = app.seen.tags[tags - 1].clone();
    let at = shown(&context, &chip).center();
    click(&mut app, &mut context, at);
    check(
        app.model.tags.len() == tags - 1 && !app.model.tags.contains(&name),
        "the tag is removed once",
        &app.model.tags,
    )?;

    // Narrow window: lines break, nothing leaves the width, the field and its action stay
    // together and still take input where they are shown.
    context.on_input(InputEvent::Resized {
        width: 300,
        height: 700,
    });
    settle(&mut app, &mut context);
    let bar = app.seen.bar.as_ref().ok_or("no filter bar")?;
    check(
        lines(&bar.items) > wide,
        "the bar wraps in 300 px",
        lines(&bar.items),
    )?;
    check(
        bar.items.iter().all(|r| r.max.x <= 300.0),
        "items stay in the window",
        bar.rect,
    )?;
    let field = app.seen.field.ok_or("no field")?;
    let add = app.seen.add.ok_or("no add button")?;
    let (field_rect, add_rect) = (shown(&context, &field), shown(&context, &add));
    check(
        (field_rect.center().y - add_rect.center().y).abs() < 20.0
            && add_rect.min.x >= field_rect.max.x,
        "field and action share a line",
        (field_rect, add_rect),
    )?;
    click(&mut app, &mut context, field_rect.center());
    context.on_text_event("ui");
    settle(&mut app, &mut context);
    check(
        app.model.draft == "ui",
        "typing reaches the wrapped field",
        &app.model.draft,
    )?;
    let add_rect = shown(&context, app.seen.add.as_ref().ok_or("no add button")?);
    click(&mut app, &mut context, add_rect.center());
    check(
        app.model.tags.last().is_some_and(|tag| tag == "ui") && app.model.draft.is_empty(),
        "Add adds one tag",
        (&app.model.tags, &app.model.draft),
    )?;

    // The menu opens at the displayed trigger and its action works.
    let more = shown(&context, app.seen.more.as_ref().ok_or("no menu")?);
    click(&mut app, &mut context, more.center());
    check(app.model.menu, "the menu opens", app.model.menu)?;
    let clear = app.seen.clear.ok_or("the menu is not built")?;
    let clear_rect = shown(&context, &clear);
    check(
        clear_rect.min.y >= more.max.y - 1.0,
        "the menu opens under the trigger",
        (clear_rect, more),
    )?;
    click(&mut app, &mut context, clear_rect.center());
    check(
        app.model.active.is_empty() && app.model.tags.is_empty() && !app.model.menu,
        "Clear resets the filters",
        &app.model.active,
    )?;

    // Cards of different heights line up on the chosen edge of their line.
    context.on_input(InputEvent::Resized {
        width: 700,
        height: 700,
    });
    for align in [Edge::Start, Edge::Middle, Edge::End] {
        app.model.item_align = align;
        settle(&mut app, &mut context);
        let cards = app.seen.cards.as_ref().ok_or("no cards")?;
        let mut heights: Vec<f32> = cards.items.iter().map(|r| r.size().y).collect();
        heights.dedup();
        check(heights.len() > 1, "the cards differ in height", &heights)?;
        for line in &cards.lines {
            for item in cards
                .items
                .iter()
                .filter(|r| r.center().y > line.min.y && r.center().y < line.max.y)
            {
                let spare = line.size().y - item.size().y;
                let want = line.min.y + align_offset(align) * spare;
                check(
                    (item.min.y - want).abs() < 0.5,
                    "cards sit on the aligned edge",
                    (item, line),
                )?;
            }
        }
    }
    Ok(())
}

fn align_offset(edge: Edge) -> f32 {
    match edge {
        Edge::Start => 0.0,
        Edge::Middle => 0.5,
        Edge::End => 1.0,
    }
}
