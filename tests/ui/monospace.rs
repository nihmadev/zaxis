//! Monospace and tabular text through Text, typography, styles, TextEdit and Table.
use crate::prelude::*;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Ime, WindowEvent},
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::{
    Align, Column, FontWeight, Id, Rect, Table, Text, TextEdit, TextFamily, TextStyle,
    TypographyRole, Window,
};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(900, 600), 1.0);
    c
}

fn show(c: &mut Context, build: impl FnOnce(&mut zaxis::Ui<'_>)) {
    c.run(|c| {
        Window::new("Mono").show(c, build);
    });
}

/// Fonts, tabs and positions of every painted string equal to `text`.
fn painted(c: &Context, text: &str) -> Vec<(Option<TextFont>, u16, Vec2)> {
    fn walk(paint: &[Paint], text: &str, out: &mut Vec<(Option<TextFont>, u16, Vec2)>) {
        for p in paint {
            match p {
                Paint::Text {
                    text: t, position, ..
                } if t == text => out.push((None, DEFAULT_TAB, *position)),
                Paint::Paragraph {
                    text: t,
                    font,
                    tab,
                    position,
                    ..
                } if t == text => out.push((Some(*font), *tab, *position)),
                Paint::Visual { paint, .. } => walk(paint, text, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for cached in c.probe().cache.values() {
        walk(&cached.paint, text, &mut out);
    }
    out
}

fn mono(weight: FontWeight) -> TextFont {
    TextFont::new(weight, TextFamily::Monospace, false)
}

#[test]
fn plain_text_still_paints_as_plain_text() {
    let mut c = setup();
    show(&mut c, |ui| {
        ui.label("Plain");
        ui.add(Text::new("Wide").tab_size(8));
    });
    for text in ["Plain", "Wide"] {
        assert_eq!(painted(&c, text).len(), 1, "{text}");
        assert_eq!(
            painted(&c, text)[0].0,
            None,
            "{text}: no family, no figures"
        );
    }
}

#[test]
fn text_family_is_chosen_by_builder_style_and_role_in_that_order() {
    let mut c = setup();
    let patch = zaxis::StyleOverrides {
        text: TextStyle {
            family: Some(TextFamily::Monospace),
            ..Default::default()
        },
        ..Default::default()
    };
    show(&mut c, |ui| {
        ui.add(Text::new("builder").monospace());
        ui.code("role");
        ui.add(
            Text::new("role off")
                .typography(TypographyRole::Code)
                .family(TextFamily::Proportional),
        );
        ui.with_style(&patch, |ui| {
            ui.label("style");
            ui.add(Text::new("style off").family(TextFamily::Proportional));
        });
    });
    let family = |t: &str| painted(&c, t)[0].0.map(|f| f.family);
    assert_eq!(family("builder"), Some(TextFamily::Monospace));
    assert_eq!(family("role"), Some(TextFamily::Monospace));
    assert_eq!(family("style"), Some(TextFamily::Monospace));
    assert_eq!(family("role off"), None, "proportional plain text");
    assert_eq!(family("style off"), None);
}

#[test]
fn code_role_uses_the_code_size_and_reports_matching_metrics() {
    let mut c = setup();
    let typography = c.style().typography;
    let mut rect = None;
    let mut metrics = None;
    show(&mut c, |ui| {
        metrics = Some(ui.monospace_metrics());
        rect = Some(
            ui.add(
                Text::new("0123456789")
                    .typography(TypographyRole::Code)
                    .wrap(false),
            )
            .rect,
        );
    });
    let metrics = metrics.unwrap();
    let expected = c.monospace_metrics(typography.code, typography.weights.code);
    assert_eq!(metrics, expected);
    assert!((rect.unwrap().size().x - 10.0 * metrics.cell_width).abs() < 1e-3);
    assert!((rect.unwrap().size().y - metrics.line_height).abs() < 1e-3);
}

#[test]
fn unwrapped_monospace_text_keeps_columns_and_the_parent_only_clips() {
    let mut c = setup();
    let cell = c.monospace_metrics(14.0, FontWeight::REGULAR).cell_width;
    let long = "abcdefghij".repeat(40);
    let mut rects = Vec::new();
    show(&mut c, |ui| {
        ui.with_max_width(200.0, |ui| {
            for s in [long.as_str(), "short"] {
                rects.push(ui.add(Text::new(s).monospace().wrap(false)).rect);
            }
            rects.push(
                ui.add(Text::new("wrap me please ".repeat(10)).monospace())
                    .rect,
            );
        });
    });
    assert!(
        (rects[0].size().x / cell - 400.0).abs() < 0.01,
        "never cut by layout"
    );
    assert!((rects[1].size().x - 5.0 * cell).abs() < 1e-2);
    assert!(
        rects[2].size().y > 2.0 * rects[1].size().y,
        "wrapping is still opt-out"
    );
}

#[test]
fn tab_size_of_text_is_in_cells() {
    let mut c = setup();
    let cell = c.monospace_metrics(14.0, FontWeight::REGULAR).cell_width;
    let mut widths = Vec::new();
    show(&mut c, |ui| {
        for tab in [2, 4, 8] {
            widths.push(
                ui.add(Text::new("\tx").monospace().tab_size(tab).wrap(false))
                    .rect
                    .size()
                    .x,
            );
        }
    });
    for (w, tab) in widths.iter().zip([2.0, 4.0, 8.0]) {
        assert!((w - (tab + 1.0) * cell).abs() < 1e-2, "{w} for tab {tab}");
    }
}

#[test]
fn unchanged_monospace_text_builds_no_layouts_and_families_do_not_invalidate_each_other() {
    let mut c = setup();
    let pass = |c: &mut Context, code: &str| {
        show(c, |ui| {
            ui.label("proportional label");
            ui.add(Text::new(code.to_owned()).monospace().wrap(false));
        });
    };
    pass(&mut c, "let x = 1;");
    let built = c.text_layouts_built();
    pass(&mut c, "let x = 1;");
    assert_eq!(
        c.text_layouts_built(),
        built,
        "an unchanged UI shapes nothing"
    );
    pass(&mut c, "let x = 2;");
    assert_eq!(
        c.text_layouts_built(),
        built + 1,
        "only the changed monospace string"
    );
}

fn edit_pass(c: &mut Context, text: &mut String, monospace: bool, rows: bool) -> Rect {
    let mut rect = None;
    show(c, |ui| {
        let mut edit = TextEdit::new(text).id_source("code").width(300.0);
        if monospace {
            edit = edit.monospace();
        }
        if rows {
            edit = edit
                .multiline()
                .wrap(false)
                .tab_indent(true)
                .tab_size(4)
                .rows(3.0);
        }
        rect = Some(ui.add(edit).rect);
    });
    rect.unwrap()
}

fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}

fn cursor(c: &Context) -> usize {
    let id = c.probe().focused_widget.expect("focused field");
    c.probe().text_edits[&id].buffer.cursor
}

#[test]
fn single_line_monospace_edit_hits_and_selects_on_the_cell_grid() {
    let mut c = setup();
    let cell = c.monospace_metrics(14.0, FontWeight::REGULAR).cell_width;
    let mut text = "iiiiWWWW0123".to_owned();
    // Equal cells: the same pointer offset lands on the same character regardless of glyph.
    // A fresh context per click keeps them from being read as a double click.
    for (cells, expected) in [
        (0.2, 0),
        (3.4, 3),
        (3.6, 4),
        (4.4, 4),
        (7.7, 8),
        (11.9, 12),
        (25.0, 12),
    ] {
        let mut c = setup();
        let rect = edit_pass(&mut c, &mut text, true, false);
        let left = rect.min.x + c.style().text_edit_padding.left;
        click(&mut c, Vec2::new(left + cells * cell, rect.center().y));
        edit_pass(&mut c, &mut text, true, false);
        assert_eq!(cursor(&c), expected, "pointer at {cells} cells");
    }
    let rect = edit_pass(&mut c, &mut text, true, false);
    let left = rect.min.x + c.style().text_edit_padding.left;
    let y = rect.center().y;
    // Shift+Right extends the selection one cell per character.
    click(&mut c, Vec2::new(left + 2.0 * cell, y));
    c.set_modifiers(ModifiersState::SHIFT);
    for _ in 0..4 {
        c.on_key_event(KeyCode::ArrowRight, ElementState::Pressed, false);
        c.on_key_event(KeyCode::ArrowRight, ElementState::Released, false);
    }
    c.set_modifiers(ModifiersState::empty());
    edit_pass(&mut c, &mut text, true, false);
    let id = c.probe().focused_widget.unwrap();
    assert_eq!(c.probe().text_edits[&id].buffer.selection(), 2..6);
    // Caret and selection come from the carets of the same layout.
    let carets = c.text_carets(&text, 14.0, mono(FontWeight::REGULAR));
    for (byte, x) in carets {
        assert!((x - byte as f32 * cell).abs() < 1e-2);
    }
}

#[test]
fn ime_composition_in_a_monospace_field_uses_the_cell_layout() {
    let mut c = setup();
    let cell = c.monospace_metrics(14.0, FontWeight::REGULAR).cell_width;
    let mut text = "abcd".to_owned();
    let rect = edit_pass(&mut c, &mut text, true, false);
    click(&mut c, rect.center());
    edit_pass(&mut c, &mut text, true, false);
    c.on_window_event(&WindowEvent::Ime(Ime::Preedit("xyz".into(), Some((3, 3)))));
    edit_pass(&mut c, &mut text, true, false);
    let area = c.probe().ime_area.expect("IME rectangle while composing");
    let left = rect.min.x + c.style().text_edit_padding.left;
    // The composed run sits after "abcd" in whole cells.
    let n = (area.min.x - left) / cell;
    assert!((n - n.round()).abs() < 1e-2, "IME caret at {n} cells");
    assert!(n.round() >= 4.0);
    c.on_window_event(&WindowEvent::Ime(Ime::Commit("xyz".into())));
    edit_pass(&mut c, &mut text, true, false);
    assert_eq!(text, "abcdxyz");
}

#[test]
fn multiline_monospace_edit_uses_the_tab_size_in_cells() {
    let mut c = setup();
    let cell = c.monospace_metrics(14.0, FontWeight::REGULAR).cell_width;
    let mut text = "\tx\nabc\ty".to_owned();
    let rect = edit_pass(&mut c, &mut text, true, true);
    let paint = painted(&c, "\tx");
    assert_eq!(paint.len(), 1);
    assert_eq!(paint[0].0, Some(mono(FontWeight::REGULAR)));
    assert_eq!(paint[0].1, 4, "the field's tab size reaches the layout");
    let layout = c.paragraph_layout("\tx", 14.0, mono(FontWeight::REGULAR), f32::INFINITY, 4);
    assert!((layout.lines[0].clusters[1].x0 - 4.0 * cell).abs() < 1e-2);
    let layout = c.paragraph_layout("abc\ty", 14.0, mono(FontWeight::REGULAR), f32::INFINITY, 4);
    assert!(
        (layout.lines[0].clusters[4].x0 - 4.0 * cell).abs() < 1e-2,
        "tab stops, not fixed width"
    );
    // A click after the tab lands on the character past it.
    let left = rect.min.x + c.style().text_edit_padding.left;
    click(&mut c, Vec2::new(left + 4.4 * cell, rect.min.y + 20.0));
    edit_pass(&mut c, &mut text, true, true);
    assert_eq!(cursor(&c), 1, "between the tab and `x`");
}

#[test]
fn numeric_columns_right_align_tabular_digits() {
    let mut c = setup();
    let values = ["1,111.11", "9,876.50", "0.00", "88,888.88"];
    show(&mut c, |ui| {
        Table::new("numbers")
            .columns([
                Column::fixed("name", 80.0).title("Name"),
                Column::fixed("value", 140.0).title("Value").numeric(true),
            ])
            .show_rows(ui, 30.0, values.len(), |body, i| {
                body.row(i, |row| {
                    row.cell(|ui| {
                        ui.label(format!("row {i}"));
                    });
                    row.cell(|ui| {
                        ui.label(values[i]);
                    });
                });
            });
    });
    let mut right = Vec::new();
    for value in values {
        let found = painted(&c, value);
        assert_eq!(found.len(), 1, "{value}");
        let (font, _, position) = found[0];
        let font = font.expect("numeric cells are not plain text");
        assert!(font.tabular && font.family == TextFamily::Proportional);
        let width = c
            .measure_text(value, c.style().font_size, font, f32::INFINITY)
            .x;
        right.push(position.x + width);
    }
    for edge in &right {
        assert!((edge - right[0]).abs() < 1e-2, "right edges {right:?}");
    }
    assert_eq!(Column::fixed("a", 1.0).numeric(true).horizontal, Align::End);
    assert_eq!(
        Column::fixed("a", 1.0)
            .numeric(true)
            .numeric(false)
            .horizontal,
        Align::Start
    );
    let _ = Id::new("unused");
}
