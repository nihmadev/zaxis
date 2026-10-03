# Changelog

Releases use a numeric Cargo version and an adjective as their display name.
Git tags use `v<version>`; the first release is **0.0.1 Blinking** (`v0.0.1`).
The entries below describe the prepared release; publication happens separately.

## 0.0.1 Blinking

First development release. Public APIs may change between development releases.

- Event-driven desktop runner using winit and wgpu, retained interaction state,
  repaint scheduling, and cached CPU/GPU geometry.
- Windows, buttons, checkboxes, sliders, text editing with IME, ComboBox/Popup,
  number controls, color pickers, scrolling, Grid, and Table.
- Animations, transitions, drawing primitives, gradients, shadows, and backdrop blur.
- Text shaping and bidirectional layout, system font fallback, and bundled color emoji.
- Optional `bundled-emoji` feature, enabled by default, with complete font data in
  the separate `zaxis-emoji` crate; Lato remains in the main crate.
- Asynchronous local image decoding and SVG rasterization with bounded resource
  caches; optional GIF/TIFF codecs expose the first frame.
- Public draw protocol and custom-host integration, examples, benchmarks, API
  documentation, and a GitHub Pages guide.
- Rust 1.90 minimum; release checks cover MSRV, desktop platforms, and crate packaging.
