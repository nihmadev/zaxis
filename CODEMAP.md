# CODEMAP

Navigation map for agents. Start here instead of reading directories. Rules and checks are in AGENTS.md; this file says where things live. Verify names with `rg` before relying on them; update the map when moving modules.

## Frame data flow

```
winit event ─► Context::on_window_event          src/context/events.rs
                 └─ input/gesture/pointer/keyboard accumulate into InputState (input.rs)
host redraw ─► Context::run(|ctx| ...)           src/context/frame.rs  (run_at)
                 ├─ clears per-pass state, begins animations/text/images/drag/scroll
                 ├─ user build: Window::show → Ui → Ui::add(Widget) → Widget::ui
                 │     widget = layout (Ui cursor) + hit regions (interaction.rs)
                 │              + Paint descriptions (paint.rs) + retained state (Id-keyed)
                 │              + accessibility node, only while collection is on
                 │                (src/accessibility/collect.rs)
                 └─ finish_frame: sort layers/hits, popups, modals, tooltips, toasts, drag preview
                      └─ accessibility tree + diff against the previous pass
                         (src/accessibility/tree.rs, convert.rs)
Context::draw_data() -> DrawData                 src/context.rs, src/context/geometry.rs
Renderer::render(&DrawData, clear)               src/renderer/frame.rs   (wgpu)
repaint: Context::needs_repaint[_at]             src/context/repaint.rs, src/app/schedule.rs

accessibility out: Context::take_accessibility_update ─► runner Adapter::publish
                   (update_if_active)            src/accessibility/adapter.rs, src/app/runner/window.rs
accessibility in:  adapter ─► UserEvent::Access ─► Context::on_accessibility_action
                   ─► pending requests / clicked ─► next pass
                                                 src/app/runner/events.rs, src/accessibility/actions.rs
```

`app::run` (winit runner) wires all of this together per window: `src/app/runner.rs` (platform-independent handler) with `runner/{events,lifecycle,window}.rs`, and the platform: `runner/native.rs` (blocking GPU setup, several windows, native chrome) or `runner/web.rs` (+ `web/canvas.rs`: one canvas, asynchronous renderer, browser key/wheel policy). On `wasm32` the clock is `web_time` through `src/time.rs`.

## Where things live

| Area | Path | Notes |
|---|---|---|
| Public exports | `src/lib.rs` | Every public type is re-exported here. New public type → add here. |
| Widget trait | `src/components/widget.rs` | `Widget::ui(self, &mut Ui) -> Response`, plus `.tooltip()` / context-menu sugar. |
| Ui (layout scope) | `src/components/ui.rs`, `ui/{flow,scopes,theme}.rs` | `add`, `horizontal`, `vertical`, `allocate_space`, id scopes. |
| Window / Root / Popup | `src/components/{window,root,popup}.rs` | Window = draggable/resizable layer; Popup = shared overlay layer. |
| Response / Sense | `src/components/{response,sense}.rs` | Per-widget interaction result. One-shot events: `src/context/gesture.rs`. |
| Context state | `src/context.rs` (struct), `src/context/*.rs` | One file per concern (see below). |
| Layout primitives | `src/layout.rs` | Logical pixels. |
| Shapes/tessellation | `src/shapes/`, `shapes/mesh/` | Rect, Color, Gradient, Shadow, Transform → meshes. |
| Text | `src/text/` | cosmic-text shaping (`shape.rs`), glyph atlas, fonts, rich runs, weights. Measurement API for components: `src/context/text_api.rs`. |
| Draw protocol | `src/protocol/` | `DrawData`, `DrawCommand`, `Vertex`, textures. Renderer-agnostic. |
| Clock | `src/time.rs` | `Instant`: std on native, `web_time` on wasm32. Re-exported as `zaxis::Instant`. |
| Renderer | `src/renderer/` | `init` (renderer assembly), `adapter` (+ `adapter/{native,web}.rs`: instance/adapter choice, WebGPU then WebGL2 in a page), `frame` (submit/present), `geometry` (buffers), `textures` (LRU uploads), `blur` (backdrop effects), `viewport`. WGSL in `src/shaders/`. |
| Animation engine | `src/animation/` | tween/spring/decay/keyframes/timeline/path; retained tracks in `state/`; Context glue in `src/context/animation.rs`; component glue in `components/motion.rs`, `moving.rs`, `effects.rs`. |
| Images | `src/images/` | decode (raster+SVG), `worker.rs` (job execution: threads via `worker/threads.rs`, or inline per-frame slices on wasm32), `file.rs` (path loading; an error in a page), cache/lifecycle, resize. Context glue: `src/context/images.rs`; component: `components/image.rs`. |
| App runner | `src/app/` | `App` trait, `Windows` (multi-window), hub/registry (no winit/GPU, testable), runner (winit, split into platform-independent code and `native`/`web` platform modules), schedule (repaint deadlines), commands (window requests), `browser.rs` (`WebOptions`, browser key policy; plain data, all targets). |
| Theme/Style | `src/components/{style,appearance}.rs`, `components/theme/` | `Style` is what widgets consume; typed `Theme` resolves into it (`resolve.rs`, `palette.rs`, `tokens.rs`, per-control `controls.rs`, `overrides.rs`). |
| Accessibility | `src/accessibility/` | `node` (`AccessNode`), `role` (roles, actions, state enums), `collect` (nodes while the UI is built, `Ui::accessible`), `widget` (`Accessible` wrapper), `text` (text runs), `tree` (tree + diff), `convert` (to AccessKit nodes), `ids`, `actions` (updates out, requests in), `adapter` (accesskit_winit, native only), `audit` (missing names), `testing` (`AccessTree`, doc-hidden). Features `accesskit` (default), `accesskit_unix`. Complex components keep their description in a per-component `access.rs`. |
| Icons / emoji | `crates/z-icons` (feature `bundled-icons`), `crates/z-emoji` (crate `z-emoji`, feature `bundled-emoji`) | Separate workspace crates; `generated.rs` is exempt from the size limit. |

## Context concerns (`src/context/`)

| File | Concern |
|---|---|
| `id.rs`, `shared.rs`, `init.rs` | Id hashing, resources shared across windows, construction. |
| `events.rs`, `input.rs`, `pointer.rs`, `keyboard.rs`, `gesture.rs`, `ime.rs`, `viewport.rs` | Input pipeline, focus traversal, IME, DPI. |
| `clipboard.rs`, `clipboard/{native,web,memory}.rs` | `ClipboardBackend`: system clipboard, browser document events + `navigator.clipboard`, in-memory (tests). |
| `interaction.rs` | Hit regions (`HitAction`), capture, hover/active queries. Clipped hit regions also drive `cursor.rs`. |
| `paint.rs`, `paint/helpers.rs`, `geometry.rs` | Paint descriptions → per-element mesh cache → incremental frame meshes + draw-data revisions. |
| `placement.rs`, `scroll.rs`, `scroll_input.rs` | Deferred/measured cell content, scroll routing, middle-button autoscroll. |
| `windows.rs`, `popup.rs`, `modal.rs`, `tooltip.rs`, `toast.rs` | Layers and overlays: window order, single popup, modal stack, tooltips, toasts. |
| `drag/` | Drag-and-drop runtime: `input` (pointer), `keyboard`, `state`, `targets`, `preview`, `autoscroll`, `frame`. UI side: `components/drag_drop/`. |
| `selection.rs`, `selection/drag.rs` | Static text selection: one range per context (`SelectionState`: items by `Id`, scopes in build order), Ctrl/Cmd+A/C hook, drag and autoscroll. UI side: `components/text_block/`. Link middle-click is in `gesture.rs`. |
| `split.rs`, `tree.rs`, `carousel.rs`, `disclosure.rs` | Input routing specific to those components. |
| `animation.rs`, `repaint.rs`, `frame.rs` | Animation tick, repaint invalidation/deadlines, pass lifecycle and cleanup. |
| `diagnostics.rs`, `debug_overlay.rs`, `testing.rs` | Usage diagnostics, debug drawing, simulated input for tests (`zaxis::testing`). |
| `theme.rs`, `native_chrome.rs`, `text_api.rs` | Theme sampling, title-bar/native chrome, text measurement. |

## Components (`src/components/`)

Simple widgets are one file: `button`, `checkbox`, `switch`, `badge`, `card`, `loader`, `progress`, `separator`, `skeleton`, `hover`, `text`, `rich_text`, `toast`, `tooltip`, `title_bar`, `columns`, `field`, `tab_bar`, `icon_tabs`.

Complex ones are a parent file plus a directory split by responsibility (`show`/`options`/`style`/`state`/`paint`/`nav`, and `access` for what the component tells assistive technology):

| Component | Entry | Related |
|---|---|---|
| Static text: Hyperlink, SelectableLabel, `rich_label`, SelectionScope | `hyperlink.rs`, `hyperlink/{style,url}.rs`, `selectable_label.rs`, `selection_scope.rs`, `rich_text.rs` (flat `Span` model) | One engine, `text_block.rs` + `text_block/{options,state,layout,fit,links,select,paint,menu,copy_button,access}.rs`: reuses TextEdit's `Doc` paragraph table (`text_edit/doc.rs`; runs via `text/rich.rs` / `Paint::Rich`) and `text_geometry.rs`. Plain `Text` stays separate. |
| TextEdit (single + multiline) | `text_edit.rs`, `text_edit/{single,area,doc,doc_layout,events,scroll}.rs` | Shared: `edit_buffer.rs`, `edit_history.rs`, `text_geometry.rs`. NumberInput reuses these (`number_input.rs`, `numeric.rs`). |
| ScrollArea | `scroll_area.rs`, `scroll_area/{chrome,rows}.rs` | Context: `context/scroll.rs`. |
| ListBox | `list_box/` (`show`, `model`, `nav`, `select`, `paint`, `row`, `heights`) | Virtualized, keyed rows. |
| TreeView | `tree_view.rs`, `tree_view/` | Input: `context/tree.rs`. |
| Table / Grid | `table.rs`, `table/`, `grid.rs`, `grid/` | Table shares Grid columns + ScrollArea. |
| SplitPane | `split_pane.rs`, `split_pane/{allocation,interaction,paint,show,style}.rs` | Input: `context/split.rs`. |
| ComboBox, ColorPicker, KeyBox | `combo_box/`, `color_picker/`, `key_box/` | Use Popup. |
| ContextMenu, MenuBar | `context_menu.rs`, `menu_bar.rs` (+ dirs) | Cascading menus, keyboard nav. |
| Modal / Dialog | `modal.rs`, `modal/{access,dialog,geometry,parts}.rs` | Stack: `context/modal.rs`. |
| Carousel | `carousel/` | Wheel routing: `context/carousel.rs`. |
| RadioGroup, SegmentedControl | `radio/`, `segmented/` | |
| Slider / DragValue | `slider.rs`, `slider/`, `drag_value.rs` | Keyboard: `context/keyboard.rs`. |
| Drag & drop, Reorder | `drag_drop/`, `reorder.rs` | Runtime in `context/drag/`. |
| CollapsingHeader | `collapsing_header.rs`, `disclosure.rs`, `disclosure/` | Shared row composition. |
| Effects/blur/motion | `effects.rs`, `blur.rs`, `motion.rs`, `moving.rs` | Subtree effects, moving elements. |

Adding a component usually touches: the component files, `components/mod.rs` exports, `lib.rs` re-exports, `Style`/theme tokens (`components/theme/controls.rs`, `resolve.rs`) if it is styled, a Context file only if it needs new input routing or retained state, `docs/content/docs/components/<name>.mdx` + `meta.json`, an example in `examples/`, tests in `tests/ui/`, and a scenario in `benches/` (see AGENTS.md).

## Tests, examples, docs, benches

- `tests/ui/` — behavior tests through the public API with `zaxis::testing` (main.rs lists modules; one `.rs` or dir per component); links and selection are in `tests/ui/text/` (its clipboard check is `#[ignore]`: `cargo test --test ui text::clipboard -- --ignored`). `tests/context/`, `tests/components/`, `tests/animation/`, `tests/text/`, `tests/shapes/`, `tests/renderer/` (needs a GPU; `ZAXIS_SKIP_GPU_TESTS=1`), `tests/images/`, `tests/app/`, `tests/accessibility/` (roles, names, requests and incremental updates of every component group through `zaxis::accessibility::testing::AccessTree`, no window; needs the `accesskit` feature). Top-level `layout_tests.rs`, `memory_tests.rs`, `motion_presets.rs`, `icons.rs`.
- Tests of private internals live in `tests/ui/*_internal.rs` (not in `src/`); `src/context/*_tests.rs` no longer exist.
- `examples/` — one per component plus `demo.rs`, `integration.rs` (custom host loop), `custom_widget.rs`, `custom_animation.rs`, `accessibility.rs` (a form for screen readers; `--smoke-test` checks the tree without a window); multi-file examples use a same-named dir (`accessibility/`, `settings/`, `multi_window/`, `tree/`, `modals/`, `drag_and_drop/`, `split_pane/`, `animations/`).
- `benches/performance.rs` + `benches/support/scene/{build,cases,input,verify}.rs` — the single perf harness; add scenes here. `benches/support/access.rs` holds the accessibility cases (idle off/on, one change, full tree, text).
- `docs/content/docs/` — mdx docs (`app`, `layout`, `input`, `repaint`, `renderer`, `protocol`, `animation`, `style`, `limitations`, `components/*`). Update with behavior changes.
- `prompts/` — historical task briefs per component; not runtime code, ignore unless asked.

## Search tips

- Public API of a type: `rg "pub struct Name|impl .* for Name|impl Name" src`.
- Who consumes a hit action / input path: `rg "HitAction::Name" src` (interaction.rs defines, pointer.rs/keyboard.rs/gesture.rs dispatch).
- Retained per-widget state is keyed by `Id` in Context maps; start from the component's `state.rs` or the Context file with the matching name.
- Feature gates: `bundled-emoji`, `bundled-weights`, `bundled-monospace`, `bundled-icons`, `image-gif`, `image-tiff`, `accesskit`, `accesskit_unix` (Cargo.toml).
