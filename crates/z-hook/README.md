# z-hook

Draws a [zaxis](https://github.com/nihmadev/zaxis) interface over the frames of the process it
runs in, by hooking how that process presents them, and gives it the host's input: debug and
profiling overlays, mod menus, tools inside 3D editors.

```rust
let overlay = z_hook::Overlay::install(z_hook::OverlayOptions::default(), |ctx| {
    zaxis::Window::new("Debug").show(ctx, |ui| {
        ui.label("inside the host");
    });
})?;
```

For processes you control or that allow it. `z-hook` gets control only after something loaded it
(a Vulkan loader reading an implicit layer, a mod loader, a plugin system); it does not inject
itself, hide, or work around protections.

| API | Status |
| --- | --- |
| Vulkan on Linux | implicit layer, run and checked (Mesa Anv, llvmpipe) |
| Vulkan, Direct3D 11 and 12 on Windows | compiles for `x86_64-pc-windows-msvc`, **not run** |
| OpenGL, macOS | not supported |

Input: Windows message translation (not run), X11 through XInput2 (checked, observation only),
none from Wayland hosts (use `z_hook::input_sink()`).

See the [documentation page](https://nihmadev.github.io/zaxis/hook/) for how it works, the
limits and what was checked. Examples: `cargo build -p z-hook --examples`, then
`VK_ADD_IMPLICIT_LAYER_PATH=crates/z-hook/layer ZAXIS_HOOK_ENABLE=1 vkcube`. Tests:
`cargo test -p z-hook` (the Vulkan ones need a device; `ZAXIS_SKIP_GPU_TESTS=1` skips them).

MIT.
