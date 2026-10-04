# Changelog

Releases use a numeric Cargo version and an adjective as their display name.
Git tags use `v<version>`; releases so far are **0.0.1 Blinking** (`v0.0.1`) and **0.0.2 Helious** (`v0.0.2`);
the next one is **0.0.3 Genetic** (`v0.0.3`).
The entries below describe the prepared release; publication happens separately.

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
