//! A Vulkan implicit layer that draws a zaxis window over any Vulkan application.
//!
//! Build it with `cargo build -p z-hook --example vk_overlay`, then run a Vulkan program with
//! the layer enabled (see `layer/README.md` in the documentation):
//!
//! ```sh
//! VK_ADD_IMPLICIT_LAYER_PATH=crates/z-hook/layer ZAXIS_HOOK_ENABLE=1 vkcube
//! ```

use z_hook::{OverlayOptions, ToggleKey};
use zaxis::{winit::keyboard::KeyCode, Context, Slider, TextEdit, Window};

z_hook::vulkan_layer!(|| {
    let mut level = 0.5_f32;
    let mut enabled = true;
    let mut clicks = 0_u32;
    let mut name = String::new();
    let options = OverlayOptions::default().toggle(Some(ToggleKey::new(KeyCode::F10)));
    let ui = move |ctx: &mut Context| {
        Window::new("Overlay").show(ctx, |ui| {
            ui.add(Slider::new(&mut level, 0.0..=1.0));
            ui.checkbox(&mut enabled, "Enabled");
            if ui.button("Click").clicked() {
                clicks += 1;
            }
            ui.label(format!("Clicks: {clicks}"));
            ui.add(TextEdit::new(&mut name).id_source("name"));
        });
    };
    (options, ui)
});
