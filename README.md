# zaxis

Typed themes resolve into the existing immediate mode `Style`:

```rust
context.set_theme(zaxis::Theme::light().density(zaxis::Density::Compact)
    .accent(zaxis::Color::rgb(35, 85, 155)));
```

`ui.with_theme(&theme, |ui| ...)` and `ui.with_style(&overrides, |ui| ...)` style
local subtrees and their popups. Explicit component overrides survive accent and
density changes; legacy `set_style` installs its exact value. Run
`cargo run --example themes`. See [Style and Theme](docs/content/docs/style.mdx)
for states, painters and migration.

An event-driven immediate mode GUI for desktop tools, written in Rust.
Uses winit and wgpu, with retained interaction state and cached CPU/GPU geometry.

**0.0.3 Genetic** is a development release. Public APIs may change between
development releases. CI checks Windows, Linux, and macOS; native GPU and
desktop behavior still needs platform-specific testing.

## Why zaxis when egui exists?

zaxis belongs in the same immediate-mode UI niche as egui. It is an independent
library, with its own API and implementation. Its goal is a convenient workflow
for desktop tools: a short path from application state to a running window,
ready-to-use motion, and automatic resource handling.

Implement `App::update`, then call `zaxis::run(app)`. The default runner owns the
event loop, repaint scheduling, and GPU recovery. Widgets come with hover and reveal
transitions; custom motion uses typed tweens, springs, keyframes, and sequences.
The animation engine retains channels and schedules their next repaint, so ordinary
animations do not need a separate clock or update loop in application code.

egui also has a compact runner through eframe and built-in animation helpers.
The difference is how the common tasks are packaged, rather than a claim that
zaxis is universally better or a replacement for egui.

| Task | zaxis | egui / eframe |
| --- | --- | --- |
| Start a native app | `zaxis::run(app)` with defaults; `run_with_options` for customization | [`eframe::run_native`](https://docs.rs/eframe/0.36.2/eframe/fn.run_native.html) with app name, options, and an app-creation closure |
| Animate values | Typed transitions, tweens, springs, keyframes, and track composition with automatic repaint deadlines | [`Context` animation helpers](https://docs.rs/egui/0.36.2/egui/struct.Context.html#method.animate_value_with_time) for booleans and scalar values, including easing helpers |
| Load local images / SVG | Built-in async loading through `ui.image(source)`; no loader registration | [`egui_extras` loaders](https://docs.rs/egui_extras/0.36.2/egui_extras/loaders/fn.install_image_loaders.html) with format features and loader installation |
| Target applications | Desktop tools; the default runner drives one or several native windows, or one browser canvas on WebGPU / WebGL2 | [Native, web, and game-engine integrations](https://github.com/emilk/egui) |

This compares the everyday API paths with egui/eframe 0.36.2, not rendering speed
or overall feature coverage. zaxis is an early development release with a smaller
scope. Choose the workflow that fits your application.

## Quick start

Requires Rust **1.90** or newer. Once published on crates.io:

```toml
[dependencies]
zaxis = "=0.0.3"
```

Before publication, use `zaxis = { git = "https://github.com/nihmadev/zaxis" }`.

```rust
use zaxis::{App, Context, Frame, Window};

#[derive(Default)]
struct Counter(u64);

impl App for Counter {
    fn update(&mut self, context: &mut Context, _frame: &mut Frame<'_>) {
        Window::new("Counter").show(context, |ui| {
            if ui.button("Increment").clicked() {
                self.0 += 1;
            }
            ui.label(format!("Count: {}", self.0));
        });
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run(Counter::default())
}
```

**[Documentation](https://nihmadev.github.io/zaxis/)** — installation, components,
layout, input, repaint scheduling, custom hosts, renderer, and drawing protocol.

| Task | Documentation source |
| --- | --- |
| Add the library | [Installation](https://nihmadev.github.io/zaxis/installation/) |
| Create an application | [First application](https://nihmadev.github.io/zaxis/quickstart/) |
| Edit or drag typed numbers | [NumberInput / DragValue](https://nihmadev.github.io/zaxis/components/number-input/) |
| Open, save and drop files | [File dialogs and file drop](https://nihmadev.github.io/zaxis/file-dialogs-and-drop/) |
| Use controls | [Components](https://nihmadev.github.io/zaxis/components/) |
| Scroll long content and virtual lists | [ScrollArea](https://nihmadev.github.io/zaxis/components/scroll-area/) |
| Compose forms and data tables | [Grid](https://nihmadev.github.io/zaxis/components/grid/), [Table](https://nihmadev.github.io/zaxis/components/table/) |
| Fold arbitrary content and browse large hierarchies | [CollapsingHeader](docs/content/docs/components/collapsing-header.mdx), [TreeView](docs/content/docs/components/tree-view.mdx) |
| Add transitions or a custom spring | [Animation](https://nihmadev.github.io/zaxis/animation/) |
| Integrate an event loop | [Custom host](https://nihmadev.github.io/zaxis/integration/) |
| Find a public type or method | [API index](https://nihmadev.github.io/zaxis/api/) |
| Build or publish the documentation | [Site setup](https://nihmadev.github.io/zaxis/development/) |

## Run the examples

Clone the Git repository before running the examples and benchmarks.

Stable Rust 1.90 or newer, a desktop display, and a functioning GPU backend are required.

```sh
cargo run --example demo
cargo run --example settings
cargo run --example scroll_area
cargo run --example grid_table
cargo run --example text_edit
cargo run --example combo_box
cargo run --example modals
cargo run --example collapsing_headers
cargo run --example tree
cargo run --example integration
cargo run --example animations
cargo run --example custom_animation
```

The demo/settings/integration examples use immediate presentation; append
`-- --vsync` for synchronization. `grid_table` uses Immediate presentation.
Animation examples use the default Vsync runner.

`CollapsingHeader::new(id_source, caption).show(ui, body)` folds arbitrary measured
content and supports independent right actions. `TreeView::new(id_source).show(ui,
&model)` uses application-owned `TreeModel`, stable node Ids, keyboard navigation,
lazy request events and fixed-height virtualization. Both support controlled state
and local themes. Tree revision explicitly invalidates its expanded-row cache;
`reveal_node` opens a parent path and focuses/scrolls on an explicit request.

Built-in hover and reveal use 160 ms Quad Out; `Ui::tab_pages` uses directional
280 ms Quint Out slides. `Style::motion` controls duration, easing and reduced
motion. Stable animation channels run on monotonic time, sleep during delay and
stop scheduling when settled or hidden. Applications can implement `Interpolate`
and `Animation<T>` or provide closures without modifying zaxis.

## Browser

The same `zaxis::run` builds for `wasm32-unknown-unknown` and draws on a canvas with WebGPU,
falling back to WebGL2. The call returns immediately after registering the event loop, and
there is one window per page; system fonts, files and threads do not exist there. The
`web_landing` example runs natively and in a browser from one source:

```sh
cargo install trunk wasm-bindgen-cli --version 0.2.129
cd examples/web_landing && trunk serve
```

See [Web](docs/content/docs/web.mdx) for building without Trunk, backends, the clipboard, keyboard
handling and the limits.

## Images

Built-in images need no loader installation: `ui.image("assets/photo.jpg")`,
`ui.image(include_bytes!("assets/logo.svg"))`, or
`Image::new(ImageSource::rgba([width, height], pixels))`. PNG/JPEG/WebP/BMP/SVG work
out of the box; default features also include GIF/TIFF (first animation frame only).
Loading and SVG rasterization run on bounded background workers with automatic runner
wakeup, bounded CPU/GPU caches and explicit update/reload/release operations.
See [image API, memory limits and custom host integration](https://nihmadev.github.io/zaxis/components/image/).
Run `cargo run --release --example images -- --smoke-test` for the public API example.

## Optional features

Default features include `bundled-emoji`, `image-gif`, `image-tiff`, and `accesskit`.
The complete Noto Color Emoji font is shipped in the separate `z-emoji` crate.
Splitting the package keeps each crates.io upload small enough; it does not reduce
default download size or memory use. It also lets applications omit the font:

```toml
[dependencies]
zaxis = { version = "=0.0.3", default-features = false, features = ["image-gif", "image-tiff"] }
```

This keeps the optional image codecs and uses system fonts for emoji. Emoji
coverage and color rendering then depend on installed fonts. PNG/JPEG/WebP/BMP/SVG,
text shaping, and desktop backends remain available without default features.

## Accessibility

Widgets are exposed to screen readers through [AccessKit](https://accesskit.dev):
UI Automation on Windows and NSAccessibility on macOS with the default `accesskit`
feature, AT-SPI on Linux and the BSDs with the opt-in `accesskit_unix` feature. There
is no support in the browser. Nothing is built until a screen reader connects. Disabling
default features removes the tree; add `accesskit` back to keep it. Checked on
Windows 11 with a UI Automation client and Narrator; NVDA, JAWS, VoiceOver and Orca
were not tried. See [Accessibility](docs/content/docs/accessibility.mdx).

## Benchmark

```sh
cargo bench --bench performance
cargo bench --bench performance -- --hard
```

The benchmark opens a native GPU window, runs Vsync and Immediate, injects pointer
and keyboard interactions, and asserts the resulting state and cache behavior.
It covers widgets, window drag/resize, layouts, text, shapes, blur, DPI, repaint
deadlines, object lifecycle, and geometry/texture uploads. JSON results include
p50/p95/p99, throughput, frame budgets, mesh sizes, and CPU/GPU activity counters.

Use `--quick` for a smoke run, `--cpu-only` without a display, or `--gpu-wait` to
measure frames through GPU completion. Results default to `target/benchmark.json`.
See [benchmark details](https://nihmadev.github.io/zaxis/performance/#stress-benchmark) or
`cargo bench --bench performance -- --help` for filtering, load sizes and budgets.

## Contributing and support

See [CONTRIBUTING.md](CONTRIBUTING.md) for checks and contribution guidelines and
[CHANGELOG.md](CHANGELOG.md) for releases.
Report problems through [GitHub Issues](https://github.com/nihmadev/zaxis/issues)
or contact [@nihmadev on Telegram](https://t.me/nihmadev).
For sensitive vulnerabilities, contact Telegram privately; see [SECURITY.md](SECURITY.md).

## License

Source: [MIT](LICENSE). Bundled Inter (Regular, Medium, SemiBold, Bold), JetBrains Mono (Regular, Bold) and Noto Color Emoji fonts:
[Inter OFL](assets/OFL-Inter.txt), [JetBrains Mono OFL](assets/OFL-JetBrainsMono.txt),
[Noto Emoji OFL](https://github.com/nihmadev/zaxis/blob/main/crates/z-emoji/assets/OFL-NotoEmoji.txt).
Lucide-derived icons: [ISC](assets/LUCIDE-LICENSE). Include the corresponding
licenses when redistributing these assets.
