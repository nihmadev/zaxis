use zaxis::{Hyperlink, Separator, Ui};

pub fn show(ui: &mut Ui<'_>) {
    ui.add(Separator::new());
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.muted("zaxis · MIT licensed ·");
        Hyperlink::new("Source on GitHub")
            .url("https://github.com/nihmadev/zaxis")
            .open_in_browser()
            .show(ui);
    });
    ui.add_space(24.0);
}
