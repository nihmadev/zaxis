//! Code and diff viewing with the monospace family and tabular figures.
use zaxis::{
    vec2, App, Column, Context, Frame, ScrollArea, Table, Text, TextFamily, TypographyRole, Ui,
    Window,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum View {
    Source,
    Diff,
    Files,
}

#[derive(Clone, Copy, PartialEq)]
enum Change {
    Context,
    Added,
    Removed,
}

struct DiffLine {
    old: Option<usize>,
    new: Option<usize>,
    change: Change,
    hash: String,
    text: String,
}

struct Demo {
    view: View,
    source: Vec<String>,
    diff: Vec<DiffLine>,
    files: Vec<(String, u32, u32, u64, String)>,
}

fn hash(seed: usize) -> String {
    format!(
        "{:07x}",
        (seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 36
    )
}

fn line(index: usize) -> String {
    match index % 6 {
        0 => format!("fn handler_{index}(input: &str) -> Result<u32, Error> {{"),
        1 => "\tlet value = input.trim().parse::<u32>()?;".to_owned(),
        2 => format!(
            "\tif value > {} {{ return Err(Error::TooLarge(value)); }}",
            index * 7
        ),
        3 => "\t// Tabs are tab stops, a fixed number of cells apart.".to_owned(),
        4 => "\tOk(value)".to_owned(),
        _ => "}".to_owned(),
    }
}

impl Demo {
    fn new() -> Self {
        let source: Vec<String> = (0..4000).map(line).collect();
        let mut diff = Vec::new();
        let (mut old, mut new) = (1, 1);
        for (i, text) in source.iter().take(600).enumerate() {
            let change = match i % 9 {
                3 => Change::Removed,
                4 | 5 => Change::Added,
                _ => Change::Context,
            };
            let (o, n) = match change {
                Change::Context => (Some(old), Some(new)),
                Change::Removed => (Some(old), None),
                Change::Added => (None, Some(new)),
            };
            old += usize::from(o.is_some());
            new += usize::from(n.is_some());
            diff.push(DiffLine {
                old: o,
                new: n,
                change,
                hash: hash(i / 7),
                text: text.clone(),
            });
        }
        let files = (0..40)
            .map(|i| {
                (
                    format!("src/module_{i}/handler.rs"),
                    (i * 37 % 900) as u32,
                    (i * 91 % 400) as u32,
                    1_000 + (i as u64 * 7_919) % 90_000,
                    hash(i * 13),
                )
            })
            .collect();
        Self {
            view: View::Source,
            source,
            diff,
            files,
        }
    }
}

fn code(text: impl Into<String>) -> Text {
    Text::new(text).typography(TypographyRole::Code).wrap(false)
}

fn source_view(ui: &mut Ui<'_>, lines: &[String]) {
    let cell = ui.monospace_metrics();
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let gutter = 6.0 * cell.cell_width + ui.style().spacing;
    ScrollArea::both()
        .id_source("source")
        .content_width(gutter + longest as f32 * cell.cell_width)
        .show_rows(ui, cell.line_height, lines.len(), |ui, i| {
            ui.horizontal(|ui| {
                ui.add(code(format!("{:>5}", i + 1)).muted());
                ui.add(code(lines[i].as_str()).tab_size(4));
            });
        });
}

fn diff_view(ui: &mut Ui<'_>, lines: &[DiffLine]) {
    let cell = ui.monospace_metrics();
    let spacing = ui.style().spacing;
    let longest = lines
        .iter()
        .map(|l| l.text.chars().count())
        .max()
        .unwrap_or(0);
    let width =
        (5 + 5 + 7 + 1) as f32 * cell.cell_width + 4.0 * spacing + longest as f32 * cell.cell_width;
    let (success, error) = (ui.style().success, ui.style().error);
    ScrollArea::both()
        .id_source("diff")
        .content_width(width)
        .show_rows(ui, cell.line_height, lines.len(), |ui, i| {
            let l = &lines[i];
            let number = |n: Option<usize>| n.map_or_else(|| " ".repeat(5), |n| format!("{n:>5}"));
            let (marker, color) = match l.change {
                Change::Context => (' ', None),
                Change::Added => ('+', Some(success)),
                Change::Removed => ('-', Some(error)),
            };
            ui.horizontal(|ui| {
                ui.add(code(number(l.old)).muted());
                ui.add(code(number(l.new)).muted());
                ui.add(code(l.hash.as_str()).muted());
                let mut text = code(format!("{marker}{}", l.text)).tab_size(4);
                if let Some(color) = color {
                    text = text.color(color);
                }
                ui.add(text);
            });
        });
}

fn files_view(ui: &mut Ui<'_>, files: &[(String, u32, u32, u64, String)]) {
    Table::new("files")
        .columns([
            Column::remainder("path").min_width(160.0).title("File"),
            Column::fixed("added", 80.0).title("Added").numeric(true),
            Column::fixed("removed", 90.0)
                .title("Removed")
                .numeric(true),
            Column::fixed("size", 110.0).title("Size, B").numeric(true),
            Column::fixed("hash", 100.0).title("Commit"),
        ])
        .max_height(420.0)
        .show_rows(ui, 30.0, files.len(), |body, i| {
            let (path, added, removed, size, hash) = &files[i];
            body.row(i, |row| {
                row.cell(|ui| {
                    ui.label(path.as_str());
                });
                row.cell(|ui| {
                    ui.label(format!("+{added}"));
                });
                row.cell(|ui| {
                    ui.label(format!("-{removed}"));
                });
                row.cell(|ui| {
                    ui.label(format!("{size}"));
                });
                row.cell(|ui| {
                    ui.add(
                        Text::new(hash.as_str())
                            .family(TextFamily::Monospace)
                            .muted(),
                    );
                });
            });
        });
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, _frame: &mut Frame<'_>) {
        Window::new("Code view")
            .default_position(vec2(30.0, 30.0))
            .default_size(vec2(760.0, 500.0))
            .show(context, |ui| {
                ui.tab_bar(
                    &mut self.view,
                    [
                        (View::Source, "Source"),
                        (View::Diff, "Diff"),
                        (View::Files, "Files"),
                    ],
                );
                match self.view {
                    View::Source => source_view(ui, &self.source),
                    View::Diff => diff_view(ui, &self.diff),
                    View::Files => files_view(ui, &self.files),
                }
            });
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run(Demo::new())
}
