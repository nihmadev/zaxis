//! Font weight resolution through Text, styles, typography tokens and controls.
use crate::prelude::*;
use winit::dpi::PhysicalSize;
use zaxis::{
    Button, ButtonStyle, Checkbox, Column, FontWeight, Id, Root, Table, Text, TextEdit, TextStyle,
    TreeChildren, TreeModel, TreeNode, TreeView, TypographyRole, Window,
};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}

fn collect(paint: &[Paint], text: &str, out: &mut Vec<FontWeight>) {
    for primitive in paint {
        match primitive {
            Paint::Text {
                text: t, weight, ..
            } if t == text => out.push(*weight),
            Paint::Visual { paint, .. } => collect(paint, text, out),
            _ => {}
        }
    }
}

/// Weights of every painted string equal to `text`.
fn painted(c: &Context, text: &str) -> Vec<FontWeight> {
    let mut out = Vec::new();
    for cached in c.probe().cache.values() {
        collect(&cached.paint, text, &mut out);
    }
    out
}

fn with_typography(c: &mut Context, edit: impl FnOnce(&mut zaxis::Typography)) {
    let mut style = c.style().clone();
    edit(&mut style.typography);
    c.set_style(style);
}

fn show(c: &mut Context, build: impl FnOnce(&mut zaxis::Ui<'_>)) {
    c.run(|c| {
        Window::new("Weights").show(c, build);
    });
}

#[test]
fn text_without_a_weight_stays_regular_and_measures_as_before() {
    let mut c = setup();
    let mut rect = None;
    show(&mut c, |ui| {
        ui.label("Plain");
        ui.heading("Heading");
        ui.small("Small");
        rect = Some(ui.add(Text::new("Measured")).rect);
    });
    for text in ["Plain", "Heading", "Small", "Measured"] {
        assert_eq!(painted(&c, text), [FontWeight::REGULAR], "{text}");
    }
    let size = c.style().font_size;
    let measured = c.measure_text("Measured", size, FontWeight::REGULAR, f32::INFINITY);
    assert_eq!(rect.unwrap().size().x, measured.x);
    let weights = c.style().typography.weights;
    for role in [
        TypographyRole::Small,
        TypographyRole::Body,
        TypographyRole::Heading,
        TypographyRole::Title,
    ] {
        assert_eq!(weights.role(role), FontWeight::REGULAR);
    }
    assert_eq!(weights.control, FontWeight::REGULAR);
    assert_eq!(weights.selected(), FontWeight::REGULAR);
}

#[test]
fn builder_beats_style_and_style_beats_the_role() {
    let mut c = setup();
    with_typography(&mut c, |t| t.weights.heading = FontWeight::SEMIBOLD);
    show(&mut c, |ui| {
        ui.heading("Role");
        ui.add(
            Text::new("Styled")
                .typography(TypographyRole::Heading)
                .style(TextStyle {
                    weight: Some(FontWeight::MEDIUM),
                    ..Default::default()
                }),
        );
        ui.add(
            Text::new("Built")
                .typography(TypographyRole::Heading)
                .style(TextStyle {
                    weight: Some(FontWeight::MEDIUM),
                    ..Default::default()
                })
                .weight(FontWeight::BOLD),
        );
        ui.label("Body");
    });
    assert_eq!(painted(&c, "Role"), [FontWeight::SEMIBOLD]);
    assert_eq!(painted(&c, "Styled"), [FontWeight::MEDIUM]);
    assert_eq!(painted(&c, "Built"), [FontWeight::BOLD]);
    assert_eq!(painted(&c, "Body"), [FontWeight::REGULAR]);
}

#[test]
fn a_style_inherited_by_children_sets_their_weight() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.text.weight = Some(FontWeight::MEDIUM);
    c.set_style(style);
    show(&mut c, |ui| {
        ui.label("Inherited");
    });
    assert_eq!(painted(&c, "Inherited"), [FontWeight::MEDIUM]);
}

#[test]
#[cfg(feature = "bundled-weights")]
fn heavier_text_is_measured_and_allocated_with_its_own_metrics() {
    let mut c = setup();
    let (mut regular, mut bold) = (None, None);
    show(&mut c, |ui| {
        regular = Some(ui.add(Text::new("Hamburgefonstiv").wrap(false)).rect.size());
        bold = Some(
            ui.add(
                Text::new("Hamburgefonstiv")
                    .wrap(false)
                    .weight(FontWeight::BOLD),
            )
            .rect
            .size(),
        );
    });
    assert!(bold.unwrap().x > regular.unwrap().x);
    assert_eq!(bold.unwrap().y, regular.unwrap().y);
}

#[test]
fn controls_inherit_typography_and_component_styles() {
    let mut c = setup();
    with_typography(&mut c, |t| {
        t.weights.control = FontWeight::MEDIUM;
        t.weights.body = FontWeight::REGULAR;
    });
    let (mut a, mut b) = (false, false);
    show(&mut c, |ui| {
        ui.button("Inherits");
        ui.add(Button::new("Own").style(ButtonStyle {
            font_weight: Some(FontWeight::BOLD),
            ..Default::default()
        }));
        ui.add(Checkbox::new(&mut a, "Check"));
        ui.add(Checkbox::new(&mut b, "Check two"));
    });
    assert_eq!(painted(&c, "Inherits"), [FontWeight::MEDIUM]);
    assert_eq!(painted(&c, "Own"), [FontWeight::BOLD]);
    assert_eq!(painted(&c, "Check"), [FontWeight::MEDIUM]);
}

#[test]
fn component_styles_set_button_window_text_edit_and_table_header_weights() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.button.font_weight = Some(FontWeight::SEMIBOLD);
    style.window.title_font_weight = Some(FontWeight::BOLD);
    style.text_edit.font_weight = Some(FontWeight::MEDIUM);
    style.table.header_font_weight = FontWeight::SEMIBOLD;
    c.set_style(style);
    let mut text = String::from("typed");
    c.run(|c| {
        Window::new("Styled title").show(c, |ui| {
            ui.button("Primary");
            ui.add(TextEdit::new(&mut text));
            Table::new("t")
                .column(Column::remainder("name").title("Name"))
                .max_height(80.0)
                .show(ui, |body| {
                    body.row(1, |row| {
                        row.cell(|ui| {
                            ui.label("cell");
                        });
                    });
                });
        });
    });
    assert_eq!(painted(&c, "Styled title"), [FontWeight::BOLD]);
    assert_eq!(painted(&c, "Primary"), [FontWeight::SEMIBOLD]);
    assert_eq!(painted(&c, "typed"), [FontWeight::MEDIUM]);
    assert_eq!(painted(&c, "Name"), [FontWeight::SEMIBOLD]);
    // Cells are ordinary text: a header weight never leaks into content.
    assert_eq!(painted(&c, "cell"), [FontWeight::REGULAR]);
}

#[test]
fn the_active_tab_uses_the_selected_weight_without_changing_others() {
    let mut c = setup();
    with_typography(&mut c, |t| {
        t.weights.control = FontWeight::REGULAR;
        t.weights.selected = Some(FontWeight::SEMIBOLD);
    });
    let mut tab = 1;
    show(&mut c, |ui| {
        ui.tab_bar(&mut tab, [(0, "First"), (1, "Second")]);
    });
    assert_eq!(painted(&c, "First"), [FontWeight::REGULAR]);
    assert_eq!(painted(&c, "Second"), [FontWeight::SEMIBOLD]);
}

struct Names;
impl TreeModel for Names {
    fn revision(&self) -> u64 {
        0
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        [Id::new(1)].into_iter()
    }
    fn children(&self, _: Id) -> impl Iterator<Item = Id> {
        std::iter::empty()
    }
    fn node(&self, _: Id) -> Option<TreeNode<'_>> {
        Some(TreeNode::leaf("Folder").children(TreeChildren::Leaf))
    }
}

#[test]
fn tree_names_follow_the_row_style() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.tree.row.font_weight = Some(FontWeight::MEDIUM);
    c.set_style(style);
    c.run(|c| {
        Root::new().show(c, |ui| {
            TreeView::new("tree").show(ui, &Names);
        });
    });
    assert_eq!(painted(&c, "Folder"), [FontWeight::MEDIUM]);
}

#[test]
#[cfg(feature = "bundled-weights")]
fn text_edit_carets_use_the_same_weighted_layout_as_painting() {
    let mut c = setup();
    let sample = "Hamburgefonstiv Привет";
    let size = c.style().text_edit_font_size;
    let regular = c.text_carets(sample, size, FontWeight::REGULAR);
    let bold = c.text_carets(sample, size, FontWeight::BOLD);
    assert_eq!(regular.len(), bold.len());
    assert!(bold.last().unwrap().1 > regular.last().unwrap().1);
    let measured = c.measure_text(sample, size, FontWeight::BOLD, f32::INFINITY);
    assert!((bold.last().unwrap().1 - measured.x).abs() < 0.01);
}

#[test]
fn changing_a_weight_repaints_once_and_the_idle_ui_does_not_redraw() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    let frame = |c: &mut Context, weight: FontWeight| {
        show(c, |ui| {
            ui.add(Text::new("Changing").weight(weight));
            ui.label("Neighbour");
        });
    };
    frame(&mut c, FontWeight::REGULAR);
    frame(&mut c, FontWeight::REGULAR);
    let revision = c.draw_data().revision;
    assert!(!c.needs_repaint());
    frame(&mut c, FontWeight::REGULAR);
    assert_eq!(c.draw_data().revision, revision);
    assert!(!c.needs_repaint());
    let before = c.probe().stats.tessellated_elements;
    frame(&mut c, FontWeight::SEMIBOLD);
    // Only the changed string is tessellated again.
    assert_eq!(c.probe().stats.tessellated_elements, before + 1);
    assert_eq!(painted(&c, "Changing"), [FontWeight::SEMIBOLD]);
    let revision = c.draw_data().revision;
    frame(&mut c, FontWeight::SEMIBOLD);
    assert_eq!(c.draw_data().revision, revision);
    assert!(!c.needs_repaint());
}
