# Changelog

Releases use a numeric Cargo version and an adjective as their display name.
Git tags use `v<version>`; releases so far are **0.0.1 Blinking** (`v0.0.1`) and **0.0.2 Helious** (`v0.0.2`);
the next ones are **0.0.3 Genetic** (`v0.0.3`) and **0.0.4 Motion** (`v0.0.4`).
The entries below describe the prepared release; publication happens separately.

## 0.0.4 Motion

Adds accessibility through AccessKit, browser support (WebGPU with a WebGL2 fallback), a
virtualized `ListBox`, `Carousel`, `SegmentedControl`, hyperlinks, rich and selectable static
text, and custom WGSL materials. Public APIs may still change between development releases.

### Added

- Files from the system: `DropTarget::files` / `accepts_files` take files dragged from the file
  manager (hover highlight, filter, limit, one target per drop, modals block), with
  `Context::hovered_files` / `take_dropped_files`; `PickedFile`, `FileFilter`, `FileTask` and
  `Context::read_file` / `write_file` for one file type on the desktop and in a browser. The
  opt-in `file-dialogs` feature adds `FileDialog` (open, open several, save, folder) through
  `rfd`, polled with `Context::open_dialog` / `take_dialog_result`. `zaxis::testing::FileInput`
  simulates both without a window. Example: `file_drop`.
- Accessibility through AccessKit: every built-in component publishes a role, a name, a
  value, its state and the requests it accepts (UI Automation on Windows and
  NSAccessibility on macOS with the default `accesskit` feature; AT-SPI on Linux and the
  BSDs with the opt-in `accesskit_unix` feature; nothing in the browser). The tree is
  built only after assistive technology asks for it, and afterwards only changed nodes
  are sent. `Widget::accessible_label`, `accessible_description`, `accessible_role` and
  `accessibility_hidden` (`Accessible`); `Ui::accessible` and `Ui::accessible_group` for
  custom widgets (`AccessNode`, `AccessRole`, `AccessAction`, `AccessActionKind`,
  `AccessToggled`, `AccessLive`, `AccessOrientation`); `Context::announce` and
  `announce_assertive`; `Image::alt` and `decorative`, `Loader::label`, and
  `accessible_label` on `Modal`, `ListBox`, `TreeView`, `Table` and `Carousel`;
  `RunOptions::with_accessibility` and `WindowOptions::with_accessibility`;
  `DiagnosticKind::MissingAccessibleName`. `Toast` is a polite status region and `Field`
  validation messages are live. `TextEdit` publishes its lines, caret and selection and
  accepts value, replacement and selection requests through the edit buffer and undo
  history. Custom hosts use `Context::take_accessibility_update`,
  `on_accessibility_action`, `set_accessibility_active` and the re-exports
  `zaxis::accesskit` and `zaxis::accesskit_winit`. The `accessibility` example shows it.
  Checked with tests, a UI Automation client and Narrator on Windows 11; NVDA, JAWS,
  VoiceOver and Orca were not tried.
- `Hyperlink`, `RichText` and selectable static text (`ui.hyperlink`, `ui.hyperlink_to`, `Hyperlink`, `HyperlinkStyle`, `LinkActivation`, `UrlPolicy`; `ui.rich_label`, `RichText`, `Span`, `SpanStyle`, `LinkTarget`; `ui.selectable_label`, `ui.copyable_label`, `SelectableLabel`, `ui.selection_scope`, `SelectionScope`, `Context::selected_text`). A link is accent colored, underlines on hover, has one hit area per wrapped line, is a Tab stop and activates on release, Enter or Space; `Response::link_activation` tells a primary click from a middle or Ctrl+click, and the library opens an address only on request, after a scheme allowlist (`https`, `http`, `mailto`). A flat list of spans mixes weight, monospace, color, underline and independently focusable links inside one wrapped paragraph (one shaping pass per paragraph; `Paint::Rich` and `TextSystem::rich_layout` share a bounded cache that ignores colors). Static text can be selected by grapheme cluster with the pointer, double and triple click, Shift+click, Ctrl/Cmd+A and Ctrl/Cmd+C or Ctrl+Insert, with an I-beam cursor, autoscroll in a `ScrollArea`, a dimmed selection without focus, `Copy` and `Select all` menus, ellipsis (`truncate`, `max_lines`) that still copies the full text, a copy button, and one selection per window that spans the labels of a `SelectionScope` in document order. Hit testing, selection backing and painting share the paragraph layout of `TextEdit` (`text_geometry`); selection rectangles and underlines are snapped to the pixel grid. The `hyperlink` example shows it.
- `Carousel` (`CarouselVariant`, `CarouselPage`, `CarouselOutput`, `CarouselStyle`, `CarouselIndicator`, `IndicatorPosition`, `StackDirection`): a paged container with a stack of cards and a photo slider. Pages are built only for visible layers, the current page is an index, a key or internal, and pages have stable keys. Swipe follows the pointer and a release projects its momentum with `Decay` into a spring that keeps the gesture velocity, with a rubber band at the ends; keys, wheel (without stealing the vertical wheel from an enclosing `ScrollArea`), Shift+wheel, arrow buttons, a flowing pill / dots / strokes / count indicator, looping and autoplay that respects hover, focus, dragging and reduced motion. Photos use `ImageSource` with a Skeleton and a fade-in. `Style::carousel` holds the geometry. The `carousel` example shows both variants.
- `ListBox` (`ListModel`, `ListEntry`, `ListRow`, `ListMode`, `ListEvent`, `ListOutput`, `ListBoxStyle`, `ListDensity`): a virtualized list on stable keys with single, multiple and checked selection (Ctrl/Shift ranges, Ctrl+A, Esc), type-ahead, sticky section headers, separators, measured row heights with a Fenwick index and scroll anchoring, `scroll_to_key`, loading and empty states, drag reordering and nested controls. The shared row engine of `ScrollArea::show_rows` now takes a `RowMetrics` implementation, so `Table`, `TreeView`, `ComboBox` and `ListBox` use one virtualization core. The `list_box` example shows 120 000 rows.
- `SegmentedControl` (`Ui::segmented`, `SegmentOption`, `SegmentWidth`, `SegmentedSize`, `SegmentedVariant`, `SegmentedStyle`): a plate with a sliding raised thumb or an outlined Material 3 style row, icons and labels, one Tab stop with arrow/Home/End navigation, uniform shrinking with ellipsis and icon-only fallback, vertical mode and pixel-exact widths. The `segmented_control` example shows it.
- Browser support: the runner builds for `wasm32-unknown-unknown` and draws on a canvas with
  WebGPU, falling back to WebGL2 (`RunOptions::web`, `WebOptions`, `WebBackend`,
  `with_canvas_id`, `with_container_id`, `with_web_backend`, the `?zaxis_backend=` URL
  parameter, `Renderer::new_with_backends`). `run` and `run_with_options` keep one signature
  per platform; in a page they return immediately after registering the event loop and need a
  `'static` application. The runner is split into platform-independent code plus
  `runner/native.rs` and `runner/web.rs`; the renderer is created asynchronously, repaint
  deadlines use the browser's frame and timer scheduling, and a background tab draws nothing.
  One canvas means one window: `OpenOutcome::Unsupported` and `WindowError::Unsupported`
  report a second one. `examples/web_landing` runs natively and in a page, with a
  `Trunk.toml`; see the new `web` documentation page.
- `zaxis::Instant`: the crate's clock, `std::time::Instant` on native targets (public
  signatures unchanged) and `web_time::Instant` on `wasm32`, where std's panics.
- Clipboard backends: `ClipboardBackend`, `ClipboardError`, `Context::set_clipboard` and
  `testing::MemoryClipboard`. The system clipboard (arboard) is one backend; the browser
  backend takes paste text from the `paste` event and writes with `navigator.clipboard` and the
  `copy`/`cut` events, reporting a refused write on the next frame.
- `Context::decode_images_inline` and `SharedResources::decode_images_inline`: decode images on
  the drawing thread within a per-frame budget (the only mode on `wasm32`, which has no
  threads). Loading an image by file path reports an error in a browser.
- `Button::icon`: a leading icon tinted like the caption.
- Wheel scrolling glides: a `ScrollArea` draws its content easing toward the new offset
  (about 70 ms time constant; drags, explicit offsets and reduced motion snap), and one line
  of a notched wheel is now three lines of text instead of one font size.
- Custom materials: `Material` (WGSL fragment shader with a declared parameter schema and flags
  for texture, backdrop and clock use), `MaterialId` (content-keyed, idempotent registration
  through `Context::register_material` and `SharedResources`, validated with naga before any
  device sees it, errors with line and column as `MaterialError`), `Params` and `ParamKind`
  (WGSL uniform packing, 1 KiB per draw), `Ui::material` and `MaterialPaint` (rounded rectangle,
  image, backdrop, opacity, fallback), `DiagnosticKind::InvalidShader`. Draws with equal
  materials and parameters merge into one command; changing parameters rewrites a uniform block
  and never tessellates. Animated materials ask for frames only while visible. The renderer
  caches a pipeline per material (64 kept) and binds blocks with dynamic offsets; backdrop
  materials use the blur path. Examples `materials` and `pixel_cards`; a `materials` docs page.

### Changed

- The emoji font crate was renamed from `zaxis-emoji` (`crates/zaxis-emoji`) to `z-emoji`
  (`crates/z-emoji`, Rust path `z_emoji`). The `bundled-emoji` feature is unchanged.
- The `integration` example wires an AccessKit adapter and requires the `accesskit`
  feature.
- Public enums gained variants: `OpenOutcome::Unsupported`, `WindowError::Unsupported` and
  `RunError::Web`; `testing::Driver` clipboard methods return `ClipboardError` instead of
  `arboard::Error`. `arboard` and `pollster` are dependencies of native targets only.
- The UI shader no longer asks for `linear, sample` interpolation of the vertex color; the
  vertex `w` is 1, so the image is unchanged, and WebGL2 supports nothing else.
- `Hyperlink::open_in_browser` opens a new tab in a page.
- `DrawCommand` gained `material: Option<MaterialDraw>` and implements `Default`;
  `DrawData` gained `materials` and `material_uniforms`. A struct literal that lists every
  `DrawCommand` field needs `material` (or `..Default::default()`). `RendererStats` gained
  `material_*` counters. The vertex stage moved to `shaders/common.wgsl`, shared with materials,
  and the viewport uniform now carries the scale factor.

## 0.0.3 Genetic

Adds native multi-window apps with shared GPU and text resources, monospace and tabular
text, a menu bar, toasts, key binding boxes, icon tabs and badges, and window options for
overlays and heads-up panels. Public APIs may still change between development releases.

### Added

- `MenuBar` and `MenuItem`: a menu bar or (`compact`) hamburger menu with cascading submenus,
  check marks, shortcut captions, pointer title switching and full keyboard navigation
  (`Ui::menu_bar`, `ContextMenuItem::checked`). Popups can own extra panels, so a press
  inside a submenu is not an outside press. The `menu_bar` example shows it.
- Multiple native windows: `App` gained optional `windows`, `close_requested`,
  `window_failed` and `shortcut` hooks next to `update`; `WindowKey`, `WindowOptions`,
  `WindowPlan` (declarative windows), `Frame::open_window` and `Windows` (imperative open,
  close and `request_close`), `CloseRequested::reject` (e.g. "Save changes?" in a `Modal`),
  `GlobalShortcut`, `WindowInfo`/`WindowStatus`/`WindowError`, `AppStats`, and
  `RunOptions::with_exit_policy` (`ExitPolicy`) and `with_main_window`. A single-window
  `App` behaves as before. Each window has its own `Context`; closing one drops only its UI
  state.
- `SharedResources` and `Context::with_shared`: windows share one glyph atlas, image cache
  and appearance, so a glyph or image is rasterized, decoded and uploaded once;
  `SharedResources::set_theme` re-themes every window.
- Monospace text: bundled JetBrains Mono (Regular, Bold with `bundled-weights`) behind the
  default `bundled-monospace` feature (about 550 KB); `Text::monospace`, `TextFamily`,
  `TypographyRole::Code` / `Ui::code` with `Typography::code` and `TypographyWeights::code`,
  `Context::monospace_metrics` / `Ui::monospace_metrics` (`MonospaceMetrics`),
  `RunOptions::with_monospace_family` and `Context::with_font_families`. Ligatures are off
  so one character is one cell; tabs are tab stops (`Text::tab_size`, default 8 cells).
- `TextEdit::monospace`, `family`, `font_family` and `tabular_numbers`: caret, hit testing,
  selection and the IME rectangle read the same layout as `Text::monospace`.
- Tabular figures: `Text::tabular_numbers`, `TextStyle::tabular_numbers` (OpenType `tnum`),
  and `Column::numeric(true)` for right-aligned table columns with aligned digits.
- `Toast` and `Context::toast`: transient notifications stacked in the bottom-right corner.
  They paint above windows as passive overlays (no focus, no pointer input), fade and slide
  in, and fade out when their time is up; unset colors come from the current style.
- `KeyBox`, `Ui::key_box`, `KeyBinding` and `MouseBinding`: a game-style box that captures
  the next key or mouse button (`A`, `RShift`, `LMB`, `MMB`, ...), with `KeyBinding::label`
  and `is_down`, optional left-click capture and `Context::key_capture_active` so hotkeys can
  ignore input meanwhile.
- `IconTabs` and `Ui::icon_tabs`: a vertical strip of icon buttons bound to a selection, with
  eased hover and selection fills and a tooltip per tab.
- `Badge`: a small passive chip for key caps, status pills and counts.
- `Window` options: `title_bar(false)` (with `Ui::drag_window` to move it), `fit_content`,
  `on_top` for heads-up panels, and `visual(scale, opacity)` for entrance and exit animations.
  Windows without a title strip may be smaller than 64 pixels.
- `Button::align` places the caption at the start, center or end of a button.
- `Slider::caption_above` puts the caption above the track with the value on the right.
- `Ui::tab_pages_live`: tab pages that keep their normal look and input while they slide.
- `Color::with_alpha` and `Color::with_opacity`.
- Examples `multi_window` (with `--smoke-test`) and `code_view` (source, diff and file
  table in the code font).
- Docs: a `menu-bar` page; the limitations page no longer lists a missing menu bar.

### Changed

- The renderer shares device, queue, pipelines and the GPU texture store between windows
  (`Renderer::create_sibling`); each window keeps its own surface, buffers and blur targets.
  The desktop runner was split into `app` submodules.
- The context menu popup style is shared with the menu bar; ArrowLeft and ArrowRight
  navigate cascading submenus.
- Combo box highlights fade through the same hue instead of black and pick a readable
  foreground for the active fill.

### Fixed

- Backdrop blur no longer skips source pixels when downsampling (box prefilter).
- Windows created with a transparent attribute composite with the desktop through
  premultiplied alpha where the surface supports it; otherwise they stay opaque.
- Right alignment (`Column::align`) uses the real measured text width.


## 0.0.2 Helious

Adds a typed theme system and the first batch of editor-style components (context menus,
tooltips, collapsing headers, virtualized trees, split panes), plus modal dialogs, drag and
drop, validation and diagnostics. Public APIs may still change between development releases.

### Added

- Themes: typed themes, palettes and tokens with scoped style overrides; local themes are
  inherited by containers, popups, menus and modals. `Context::new()` now starts from
  `Theme::dark()`.
- Default look reworked: Inter, neutral gray scale with one blue accent, hairline
  borders, outer keyboard-focus ring, white slider/switch thumbs, accent-highlighted
  menu rows, underlined `tab_bar`, and `ButtonVariant` (`Surface`, `Solid`, `Soft`,
  `Outline`, `Ghost`). Controls resolve hover, pressed, focus and disabled states from
  the theme.
- Context menus anchored to the pointer: `Widget::context_menu` with
  `Response::menu_selected`. The menu opens when the secondary button is released.
- Tooltips with a delay and passive overlay routing (they never take input):
  `Widget::tooltip`.
- `CollapsingHeader` and shared disclosure styles.
- `TreeView`: virtualized rows, keyboard navigation and lazy loading of children.
- `SplitPane`: resizable panels with size constraints.
- Modal dialogs: `Modal`, `Dialog` and `Confirm` (with `ModalStyle`, `ModalAnchor`,
  `CloseReason`) share the popup layers and the one hit-test path. An open modal blocks
  pointer, wheel, keys, text, IME and shortcuts below it (`Context::input()` is idle
  outside it), cancels captures and drags on open, traps Tab, restores focus on close
  and reports each close reason once. Stacked modals, popups over a modal, edge sheets,
  reduced motion and local themes are supported.
- In-window drag and drop: `DragSource`, `DropTarget`, typed payloads, previews,
  autoscroll, keyboard pick-up, `DragStyle`, `TreeView::drag_nodes` with
  `TreeEvent::Moved` (a new variant), and `Table`/`Grid::drag_rows`.
- Custom widgets: `Ui::interact(rect, id_source, Sense)` gives an external widget a
  `Response` through the same hit path as built-in controls (`Sense::HOVER`, `CLICK`,
  `DRAG`, `FOCUS`); `Response::mark_changed`.
- `Response` events: `double_clicked`, `secondary_clicked`, `drag_started`, `dragged`,
  `drag_delta`, `drag_stopped`, `gained_focus` (and `lost_focus` on every control),
  each reported once per user input. Context menus and drag-and-drop sources use the same
  signals.
- Validation: `Field` and `Validation`, `.status(SemanticStatus)` on text edits, number
  controls, combo boxes, checkboxes and switches (destructive border and soft ring).
- Diagnostics: `Context::diagnostics`, `set_diagnostic_handler`, `DebugOverlay` and
  `set_debug_overlay`; id collisions, open scopes, widgets without layout space, popups
  without an anchor, tree model issues, clipboard, native window and image failures are
  reported instead of panicking or printing. `Frame::device_resets` and
  `Frame::last_device_loss` report GPU recovery.
- One-expression forms: `Ui::heading`/`title`/`small`/`typography`,
  `Ui::combo_box_values` and `ComboBox::from_pairs`; `Slider` is generic over `Numeric`
  (integers step by one with no decimals); `Button::width`.
- `Card`: framed, padded container (`CardVariant::Surface`/`Outline`, `Style::card`).
- Animation playback control: per-channel rate, reverse and seek, `MotionStyle::time_scale`,
  and `animation::testing` helpers for deterministic track checks.
- Composition and values: `Parallel::at` offsets, `Sequence::mark`/`time_of`, `Path` with
  constant-speed `PathFollow` and `trimmed` draw-on, `Oklab` color blending,
  `Interpolate` for `Option`, `Vec`, triples and `Transform`, `impl_interpolate!` and
  `impl_spring_value!` field macros, and a `Skeleton` loading placeholder.
- More easing: `Expo`, `Circ`, `Back`, `Elastic` and `Bounce` families, CSS
  `Easing::cubic_bezier` and `Easing::steps` (new `Easing` variants). `Decay` fling
  momentum, `SpringOptions::duration_bounce`, and velocity-preserving
  `transition_smooth`.
- Icons: opt-in `bundled-icons` feature re-exporting the separate `z-icons` crate as
  `zaxis::icons`. It bundles all 1866 Lucide icons as `static` SVG data (`icons::USER`,
  `icons::ARROW_LEFT`; unused icons are dropped by the linker). An `&'static Icon`
  converts into `ImageSource`; strokes are white, so `Image::tint` sets the color. The
  `icons` example shows a filterable gallery. Lucide is ISC licensed (some icons are MIT,
  from Feather); `LICENSE-LUCIDE` ships in the crate.
- Benchmarks cover menus, disclosure and split panes; docs and examples updated for the
  new components.
- Font weights and families: `FontWeight` (100–900) on `Text::weight`, `TextStyle::weight`,
  `Typography::weights` per role, and the component styles of Button, TextEdit, Window,
  TitleBar, Table headers, tree rows and the active tab. Bundled Inter now ships Regular,
  Medium, SemiBold and Bold as real font files (`bundled-weights`, on by default, about
  1 MB); missing weights use the nearest file and bold is never synthesized. Custom
  families: `FontFamily`, `Context::with_fonts`, `RunOptions::with_font_family`. The default
  look is unchanged until a weight is set. `RunOptions` gained the `font_family` field, so
  struct literals need `..Default::default()`.

### Changed

- Memory: the renderer prefers DX12 on Windows (falling back to the default backends),
  honors `WGPU_BACKEND`, requests `MemoryHints::MemoryUsage`, drops the 4x MSAA target
  (all shapes are already antialiased in geometry) and no longer copies the bundled emoji
  font to the heap. Idle process memory of the examples fell from about 410 MB to
  140–190 MB on an Intel iGPU.
- Builders no longer panic on NaN, infinity, zero or negative values: they are
  normalized and reported once as `InvalidValue`.
- `SliderStatus` is now an alias of `SemanticStatus`.
- Inter is now the only bundled font in the main crate: SVG text also uses it and Lato
  was removed.
- Internal: color picker, text edit, slider, split pane, text layout and animation state
  were split into smaller modules; context and widget regressions were reorganized into
  test modules.

### Deprecated

- `disabled(bool)` (use `enabled(!x)`), `Ui::add_enabled`, `button_enabled`,
  `checkbox_enabled`, `switch_enabled`, `slider_enabled`, and the `rounding` builders
  (use `corner_radius`).

### Fixed

- `tab_bar` no longer overflows a narrow window: tabs keep their natural width and the
  strip scrolls horizontally. A plain wheel now scrolls horizontal-only `ScrollArea`s.
- Fractional clips stay inside their panels.
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
