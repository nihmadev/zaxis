use std::collections::HashSet;
use zaxis::{
    App, Card, Context, Frame, Hyperlink, LinkActivation, LinkTarget, RadioGroup, RichText, Root,
    ScrollArea, SelectableLabel, SelectionScope, TextEdit, Theme, Ui, Underline,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Scheme {
    Dark,
    Light,
}

const LONG: &str = "Selection follows grapheme clusters, so an emoji like 👩‍💻 or a letter with a \
combining accent, é, is never split. Drag across this paragraph, double click a word, triple click \
for the whole paragraph, or hold Shift and click to extend. The text wraps with the column.";

const CODE: &str = "fn main() {\n\tlet answer   = 42;\n\tprintln!(\"{answer}\");\n}";

struct Gallery {
    scheme: Scheme,
    visited: HashSet<String>,
    paste: String,
    smoke: bool,
    passes: usize,
}

impl Gallery {
    fn mark(&mut self, url: Option<String>, activation: Option<LinkActivation>) {
        if let (Some(url), Some(_)) = (url, activation) {
            self.visited.insert(url);
        }
    }

    fn article(&mut self, ui: &mut Ui<'_>) {
        let visited = |url: &str, set: &HashSet<String>| set.contains(url);
        let docs = "https://example.com/docs";
        let guide = "https://example.com/guide/getting-started";
        let text = RichText::new()
            .text("Read the ")
            .link_with(
                "documentation",
                LinkTarget::new()
                    .url(docs)
                    .visited(visited(docs, &self.visited)),
            )
            .text(" first, then follow the ")
            .link_with(
                "getting started guide for new contributors",
                LinkTarget::new()
                    .url(guide)
                    .visited(visited(guide, &self.visited)),
            )
            .text(" or open ")
            .code("src/lib.rs")
            .text(" and see the ")
            .bold("public API")
            .text(".");
        ui.with_width(380.0, |ui| {
            let out = ui.rich_label(text);
            if let Some(link) = out.activated() {
                let (url, activation) = (link.url.clone(), link.activation);
                self.mark(url, activation);
            }
        });
    }

    fn links(&mut self, ui: &mut Ui<'_>) {
        let rows: [(&str, Underline, bool, &str); 3] = [
            (
                "Underlined always",
                Underline::Always,
                true,
                "https://example.com/always",
            ),
            (
                "Underlined on hover",
                Underline::Hover,
                true,
                "https://example.com/hover",
            ),
            (
                "Never underlined",
                Underline::Never,
                true,
                "https://example.com/never",
            ),
        ];
        for (text, underline, enabled, url) in rows {
            let out = Hyperlink::new(text)
                .url(url)
                .underline(underline)
                .enabled(enabled)
                .visited(self.visited.contains(url))
                .show(ui);
            self.mark(out.url, out.activated);
        }
        let url = "https://example.com/visited";
        let out = Hyperlink::new("Visited link")
            .url(url)
            .visited(true)
            .underline(Underline::Always)
            .show(ui);
        self.mark(out.url, out.activated);
        ui.add(
            Hyperlink::new("Disabled link")
                .url("https://example.com/disabled")
                .enabled(false)
                .underline(Underline::Always),
        );
    }

    fn selectable(&mut self, ui: &mut Ui<'_>) {
        ui.with_width(380.0, |ui| {
            SelectionScope::new().show(ui, |ui| {
                ui.add(SelectableLabel::new(
                    "Select across several labels with one gesture",
                ));
                ui.add(SelectableLabel::new(CODE).monospace());
                ui.add(SelectableLabel::new(LONG));
                ui.add(
                    SelectableLabel::new("A very long file name that does not fit.txt")
                        .truncate(true),
                );
            });
        });
        ui.add_space(8.0);
        ui.copyable_label("cargo add zaxis");
    }
}

impl App for Gallery {
    fn update(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        c.set_theme(match self.scheme {
            Scheme::Dark => Theme::dark(),
            Scheme::Light => Theme::light(),
        });
        Root::new().show(c, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    RadioGroup::new(
                        &mut self.scheme,
                        [(Scheme::Dark, "Dark"), (Scheme::Light, "Light")],
                    )
                    .horizontal(),
                );
                ui.add_space(12.0);
                Card::new("article").show(ui, |ui| self.article(ui));
                ui.add_space(12.0);
                Card::new("links").show(ui, |ui| self.links(ui));
                ui.add_space(12.0);
                Card::new("selectable").show(ui, |ui| self.selectable(ui));
                ui.add_space(12.0);
                Card::new("paste").width(380.0).show(ui, |ui| {
                    ui.add(TextEdit::new(&mut self.paste).multiline().rows(3.0));
                });
            });
        });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 3 {
                frame.close();
            } else {
                c.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Gallery {
            scheme: Scheme::Dark,
            visited: HashSet::new(),
            paste: String::new(),
            smoke: std::env::args().any(|a| a == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Hyperlink")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(520.0, 760.0)),
            ..Default::default()
        },
    )
}
