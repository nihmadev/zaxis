# CODEMAP

Navigation map for agents. Start here instead of reading directories. Rules and checks are in AGENTS.md; this file says where things live. Verify names with `rg` before relying on them; update the map when moving modules.

## Frame data flow

```
winit event ─► Context::on_window_event          src/context/events.rs
                 └─ input/gesture/pointer/keyboard accumulate into InputState (input.rs)
                    press ─► pointer.rs: popup branch targeting first (`Popups::level_at`: outside closes
                           the branch, inside an ancestor closes its descendants), then hit test
                    key ─► keyboard.rs `key_as`: owner decided here, consumed at once, from what the
                           last pass published: held-key owners, the leaf popup (Escape/Tab), then `claimed_key` (key_routing/:
                           Ui::keys claims → queue per Id), Actions, built-in controls, `group_key`
                           (focus_group/: arrows, Home, End), Tab traversal
                    wheel ─► scroll.rs: published camera reservation (pan_zoom.rs), Carousel,
                             then nested ScrollArea; consumed at dispatch, camera queue → next show
                    pan ─► shared primary capture/gesture.rs → pan_zoom.rs ordered camera input
host redraw ─► Context::run(|ctx| ...)           src/context/frame.rs  (run_at: stage order)
                 ├─ begin_pass: clock, resources/theme, each subsystem's per-pass data
                 ├─ user build (once): Window::show → Ui → Ui::add(Widget) → Widget::ui
                 │     widget = layout (Ui cursor) + hit regions (interaction.rs)
                 │              + Paint descriptions (paint.rs → paint/) + retained state
                 │                (Id-keyed, in the owner of its family; see below)
                 │              + deferred cell content (placement.rs → placement/)
                 │              + material draw: Ui::material → Paint::Material (cached mesh) and
                 │                MaterialUse (params, size, time: per frame, outside the cache key)
                 │              + accessibility node, only while collection is on
                 │                (src/accessibility/collect.rs)
                 │     Ui::keys / FocusGroup declare claims and groups; register_hit adds members
                 └─ finish_frame: overlays (popup branch retired level by level, modals, tooltips,
                      toasts, drag preview),
                      layer order of paint/hits, a11y geometry, scrolling/images, geometry,
                      publish routing (transforms, hits, key claims, focus groups, tabs), retire caches, settle
                      focus/capture, retire widget state, drop unclaimed routed input,
                      accessibility tree + diff (src/accessibility/tree.rs + tree/, convert.rs)
Context::draw_data() -> DrawData                 src/context.rs, src/context/geometry.rs
Renderer::render(&DrawData, clear)               src/renderer/frame.rs   (wgpu)
EmbeddedRenderer::{prepare+record, render_to}    src/renderer/embed.rs   (host-owned pass/texture, no window)
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
| Ui (layout scope) | `src/components/ui.rs`, `ui/{flow,scopes,region,theme,keys,focus}.rs` | `add`, `horizontal`, `vertical`, `at`, `allocate_space`, id scopes. |
| Window / Root / Popup | `src/components/{window,root,popup}.rs` | Window = draggable/resizable layer; Popup = shared overlay layer; a Popup built inside another is its child (`context/popup/branch.rs`). |
| Response / Sense | `src/components/{response,sense}.rs` | Per-widget interaction result. One-shot events: `src/context/gesture.rs`. |
| Keys for widgets | `src/context/key_events.rs` (`KeyInterest`, `KeyEvent`), `src/context/key_routing.rs` + `key_routing/{dispatch,queue}.rs`, `src/components/ui/keys.rs` (`Ui::keys` / `claim_keys` / `take_keys` / `focus_entry`) | Claims published per pass; owner chosen in `keyboard.rs` before Actions; bounded per-Id queue; held-key owners (`Owner`); `Context::input_stats`. Tests `tests/ui/keys/`, bench `benches/support/keys.rs`, docs `docs/content/docs/input.mdx`. |
| Focus groups | `src/context/focus_group.rs` + `focus_group/{publish,nav,dispatch}.rs`, `src/components/focus_group.rs` (`FocusGroup`, `FocusGroupOutput`, `FocusAxis`), `src/components/ui/focus.rs` (`Ui::focus_scope`) | Members join in `register_hit`; published with hits; `stop` / `navigate` serve Tab traversal (`keyboard.rs traverse_focus`) and `group_key`; used by RadioGroup and SegmentedControl. Tests `tests/ui/focus_group/`, `tests/accessibility/controls/focus_group.rs`. Table maps on `Id`: `src/context/id_map.rs`. |
| Context state | `src/context.rs` (struct of typed owners), `src/context/*.rs` | One owner/file per concern (see below). |
| Layout primitives | `src/layout.rs` | Logical pixels. |
| Shapes/tessellation | `src/shapes/`, `shapes/mesh/` | Rect, Color, Gradient, Shadow, Transform → meshes. |
| Text | `src/text/` | cosmic-text shaping (`shape.rs`), glyph atlas, fonts, rich runs, weights. Measurement API for components: `src/context/text_api.rs`. |
| Draw protocol | `src/protocol/` | `DrawData`, `DrawCommand`, `Vertex`, textures, `material.rs` (`MaterialId`, `MaterialDraw`, `MaterialSource`). Renderer-agnostic. |
| Materials | `src/material.rs`, `src/material/` | User WGSL fragment shaders: `description` (`Material`), `layout` (`ParamKind`, offsets, limits), `values` (`Params` packing), `assemble` + `compile` (prelude + naga validation), `registry` (content-keyed, shared through `SharedResources`), `error`. Prelude WGSL: `src/shaders/{common,material}.wgsl`. Context glue: `src/context/materials.rs` (`MaterialUse`, clock, repaint) and `src/context/geometry/material.rs` (batching, uniform blocks); drawing API: `src/components/material.rs` (`Ui::material`). |
| Clock | `src/time.rs` | `Instant`: std on native, `web_time` on wasm32. Re-exported as `zaxis::Instant`. |
| Renderer | `src/renderer/` | `Renderer` (`renderer.rs`: window facade) = `gpu.rs` (`Gpu`: the device-bound part shared with `EmbeddedRenderer`: layouts, samplers, texture store, geometry buffers, viewport uniform, material blocks, backdrop targets, counters; `Gpu::sibling`) + the window part (`init`, `surface`, `frame`: surface, presentation, recovery). `adapter` (+ `adapter/{native,web}.rs`: instance/adapter choice, WebGPU then WebGL2 in a page), `geometry` (buffers), `textures` (LRU uploads), `draw` (command recording shared by window and embedded), `pipeline` (`Target`, `PipelineSet`, `Kind`) + `pipeline_cache` (built-in and backdrop pipelines per target, shared by a family), `blur` (backdrop effects; `Compose` starts a frame from host pixels into a region), `backdrop` (feeds the engine), `materials` (`materials/{pipelines,uniforms}.rs`: pipeline cache per material and target, the frame's dynamic-offset uniform buffer), `viewport`. WGSL in `src/shaders/`. |
| Embedding | `src/renderer/embed.rs`, `embed/{options,viewport,error,pass,target}.rs` | `EmbeddedRenderer` over a host's `wgpu::Device`/`Queue`: `prepare` + `record` into the host's pass (`pass.rs`: items resolved at prepare, backdrop commands reported in `RecordReport`), `render_to` into the host's texture (`target.rs`: usage/format/region checks, direct pass or backdrop canvas via `blur::Compose`); `create_sibling`, `Renderer::create_embedded`. Example `examples/embed.rs` + `examples/embed/`, tests `tests/renderer/embed/`, bench rows `Embed` in `benches/support/embed.rs`, docs `docs/content/docs/embedding.mdx`. |
| Animation engine | `src/animation/` | tween/spring/decay/keyframes/timeline/path; retained tracks in `state/`; Context glue in `src/context/animation.rs`; component glue in `components/motion.rs`, `moving.rs`, `effects.rs`. |
| Images | `src/images/` | decode (raster+SVG), `worker.rs` (job execution: threads via `worker/threads.rs`, or inline per-frame slices on wasm32), `file.rs` (path loading; an error in a page), cache/lifecycle, resize. Context glue: `src/context/images.rs`; component: `components/image.rs`. |
| App runner | `src/app/` | `App` trait, `Windows` (multi-window), hub/registry (no winit/GPU, testable), runner (winit, split into platform-independent code and `native`/`web` platform modules), schedule (repaint deadlines), commands (window requests), `browser.rs` (`WebOptions`, browser key policy; plain data, all targets). |
| Glass | `src/components/glass.rs` (`Appearance` glass strength, fill/shadow), `components/theme/glass.rs` (`Theme::glass` marks control surfaces), `Blur::glass`, `BackdropEffect::glass` (`protocol/backdrop.rs`), lens and rim in `src/shaders/glass.wgsl` (shape from `renderer/blur/{plan,uniforms}.rs`) | Opt-in iOS glass for buttons and other controls; example `examples/glass.rs`, tests `tests/ui/glass.rs`, `tests/renderer/blur/lens.rs`. |
| Theme/Style | `src/components/{style,appearance}.rs`, `components/theme/` | `Style` is what widgets consume; typed `Theme` resolves into it (`resolve.rs`, `palette.rs`, `tokens.rs`, per-control `controls.rs`, `overrides.rs`). |
| Accessibility | `src/accessibility/` | `node` (`AccessNode`), `role` (roles, actions, state enums), `collect` (nodes while the UI is built, `Ui::accessible`), `widget` (`Accessible` wrapper), `text` (text runs), `tree` (tree + diff, coordinated in `tree.rs`; stages `tree/{select,bounds,identity,outline,diff}.rs`, motion hold `tree/policy.rs`), `convert` (to AccessKit nodes), `ids`, `actions` (updates out, requests in), `adapter` (accesskit_winit, native only), `audit` (missing names), `testing` (`AccessTree`, doc-hidden). Features `accesskit` (default), `accesskit_unix`. Complex components keep their description in a per-component `access.rs`. |
| Files, dialogs | `src/files.rs`, `src/files/` | `PickedFile` (`picked`), `FileFilter`, `FileTask` + `Notify` (`task`), background read/write (`io/{native,web}.rs`); feature `file-dialogs`: `dialog/` (`FileDialog` spec, `DialogBackend`, `MemoryDialogs`, `system/{native,web}.rs` over `rfd`). Context glue: `src/context/file_drop.rs` (+ `file_drop/{events,resolve}.rs`: hover/drop state, target choice), `file_io.rs`, `dialogs.rs`. Runner: `src/app/dialogs.rs` (`DialogHost`), `app/runner/dialogs.rs`, browser drag events `runner/web/drag.rs`. UI: `DropTarget::accepts_files`. |
| Overlay in another process | `crates/z-hook` (docs `docs/content/docs/hook.mdx`) | Separate crate over `EmbeddedRenderer`. `driver.rs` (frame driver, thread-local `Context`), `core.rs` + `input.rs` (shared state, queue, `route` by `OverlayInput`), `overlay.rs` (public handle), `backend.rs` (`PresentBackend`), `budget.rs`, `testing.rs` (mock backend). `vulkan/` implicit layer: `layer.rs` (negotiation, proc addrs), `instance.rs`, `device.rs` + `features/` (merge wgpu's needs into the host's device), `swapchain.rs`, `present.rs`, `gpu.rs` + `backend.rs` (wgpu over the host's device, drawing into the swapchain image), `surface.rs` + `input.rs` + `x11/` (XInput2 reader), `dump.rs`. `dxgi/` (Windows): `hooks.rs`, `vtable.rs`, `probe.rs`, `state.rs`, `wgpu_side.rs`, `shared.rs`, `host11.rs`, `host12.rs`. `win32/` message translation (`messages.rs`, `scan.rs`, `keys.rs`, `route.rs`, all platforms) and `window.rs` (Windows). Examples `vk_overlay` (layer), `vk_host`, `d3d11_host`, `d3d12_host`, `dxgi_overlay`; test `tests/vulkan_layer.rs`; bench `benches/overlay.rs`. |
| Actions, keymap | `src/actions.rs`, `src/actions/` (`action`, `registry`, `keymap` + `lookup`, `store`, `chord`, `stroke`, `mods`, `kbd`, `layout`, `names`, `id`, `conflict`); run time in `src/context/actions.rs` + `actions/{dispatch,handle}.rs`; widget glue `src/components/actions.rs`, `menu_bar/actions.rs`, `context_menu/actions.rs`, `button/action.rs`, `key_box/chord.rs` | Dispatch sits in `Context::key` before the focused control. Tests `tests/ui/actions/`, example `examples/actions.rs`, bench `benches/support/actions.rs`, docs `docs/content/docs/actions.mdx`. |
| Icons / emoji | `crates/z-icons` (`src/{lib,svg,catalog}.rs`, `tests/`, `examples/export.rs`, `generate.py`, own README/licenses); `crates/z-emoji` (feature `bundled-emoji`) | `z-icons` is a standalone, dependency-free `no_std` SVG crate; optional `catalog`. zaxis adapter: `src/images/source.rs`, re-export through `bundled-icons`; integration test `tests/icons.rs`, gallery `examples/icons.rs`. `generated.rs` is exempt from the size limit. |

## Context concerns (`src/context/`)

`Context` (`src/context.rs`) coordinates typed owners; each owns its state, per-pass reset and cleanup rule.

| File | Concern |
|---|---|
| `id.rs`, `shared.rs`, `init.rs` | Id hashing, resources shared across windows, construction. |
| `frame.rs` | Pass lifecycle: the order of begin/finish stages across all owners. |
| `events.rs`, `input.rs`, `viewport.rs`, `ime.rs` | winit bridge, `InputState`, DPI, IME window anchor. |
| `platform_input.rs`, `events.rs` | `InputEvent` (window-independent input) and its single entry point `Context::on_input`; `events.rs` also converts winit events (`on_window_event`). |
| `keyboard.rs`, `pointer.rs` | Explicit keyboard and pointer dispatchers (priority order, consumed/unhandled). Claims and group navigation hook in at `claimed_key`, `owned_key_tail` and `group_key`. |
| `key_events.rs`, `key_routing.rs`, `key_routing/`, `focus_group.rs`, `focus_group/`, `id_map.rs` | `KeyRouting` (claims, queue, held-key owners) and `FocusGroups` (groups being built, published groups, memory); `IdMap` / `IdSet` for per-pass tables. |
| `interaction.rs` | `Interaction`: hit regions (`HitAction`), capture, focus, activations; hover/hit-test queries; end-of-pass focus/capture settlement. Clipped hit regions also drive `cursor.rs`. |
| `gesture.rs` | One-shot `Response` events and `ClickCounter` (double/triple click sequences shared by gestures, text, trees, splits). |
| `text_input.rs` | `TextFields`: keys/text/IME/presses routed to fields, Tab-taking areas, field state. |
| `values.rs` | `ValueControls`: slider and drag-value input, numeric and color editor state. |
| `menus.rs` | `Menus`: combo box / context menu / menu bar state and their navigation keys. |
| `table.rs`, `tree.rs`, `split.rs`, `carousel.rs` | Tables (column resize, state kept while hidden), trees, split panes, carousel wheel. |
| `pan_zoom.rs` | Camera declarations and ordered addressed zoom/pan input, published with shared hit geometry; camera model lives in the application. |
| `containers.rs` | `Containers`: grids, cards, flows, tab pages, carousels, list boxes, collapsing headers. |
| `paint.rs`, `paint/` | `PaintState`; stages `route` (deferral, order, blur), `visual` (materialized transforms), `cache` (reuse/translation), `tessellate`, `data` (`Paint`). |
| `geometry.rs` | `FrameGeometry`: elements → incremental frame meshes + draw-data revisions. |
| `materials.rs` | `MaterialState` (clock of animated materials, repaint while one is visible), `MaterialUse`, `mark_material` (routed like `mark_blur`). |
| `visual.rs` | `Visuals`: composed/published transforms, effects being built, retained effect state. |
| `placement.rs`, `placement/` | Deferred/measured cell content: record (`placement.rs`), `visual` transform, `emit` (translate/clip/hand on), popup `portal` (every level of the branch and its extra panels). |
| `scroll.rs`, `scroll_input.rs` | Scroll routing, deferred scroll content, middle-button autoscroll. |
| `windows.rs`, `popup.rs`, `popup/branch.rs`, `modal.rs`, `tooltip.rs`, `toast.rs` | Layers and overlays: window order and drag, `Popups` (the open branch root → leaf, popup-class layers, dismissals), `branch.rs` (register/close/retire levels, press targeting, focus fallback), modal stack, tooltips, toasts. |
| `clipboard.rs`, `clipboard/{native,web,memory}.rs` | `ClipboardBackend`: system clipboard, browser document events + `navigator.clipboard`, in-memory (tests). |
| `drag/` | Drag-and-drop runtime: `input` (pointer), `keyboard`, `state`, `targets`, `preview`, `autoscroll`, `frame`. UI side: `components/drag_drop/`. |
| `selection.rs`, `selection/drag.rs` | Static text selection: one range per context (`SelectionState`: items by `Id`, scopes in build order), Ctrl/Cmd+A/C hook, drag and autoscroll. UI side: `components/text_block/`. Link middle-click is in `gesture.rs`. |
| `animation.rs`, `repaint.rs`, `images.rs` | Animation tick, repaint invalidation/deadlines, image glue (requests at displayed size, settle deadlines). |
| `diagnostics.rs`, `debug_overlay.rs`, `testing.rs` | Usage diagnostics, debug drawing, simulated input for tests (`zaxis::testing`). |
| `theme.rs`, `native_chrome.rs`, `text_api.rs` | Theme sampling, title-bar/native chrome, text measurement. |

## Components (`src/components/`)

Simple widgets are one file: `button`, `checkbox`, `switch`, `badge`, `card`, `loader`, `progress`, `separator`, `skeleton`, `hover`, `text`, `rich_text`, `toast`, `tooltip`, `title_bar`, `columns`, `field`, `icon_tabs`.

Complex ones are a parent file plus a directory split by responsibility (`show`/`options`/`style`/`state`/`paint`/`nav`, and `access` for what the component tells assistive technology):

| Component | Entry | Related |
|---|---|---|
| Static text: Hyperlink, SelectableLabel, `rich_label`, SelectionScope | `hyperlink.rs`, `hyperlink/{style,url}.rs`, `selectable_label.rs`, `selection_scope.rs`, `rich_text.rs` (flat `Span` model) | One engine, `text_block.rs` + `text_block/{options,state,layout,fit,links,select,paint,menu,copy_button,access}.rs`: reuses TextEdit's `Doc` paragraph table (`text_edit/doc.rs`; runs via `text/rich.rs` / `Paint::Rich`) and `text_geometry.rs`. Plain `Text` stays separate. |
| TextEdit (single + multiline) | `text_edit.rs`, `text_edit/{single,area,doc,doc_layout,events,scroll}.rs` | Shared: `edit_buffer.rs`, `edit_history.rs`, `text_geometry.rs`. Owners adapt it through internal hooks (event handler, affixes, character filter, whole-value selection): NumberInput and ColorPicker fields. |
| NumberInput / DragValue | `number_input.rs` (API), `number_input/{show,state,drag,editor,display,access}.rs`, `drag_value.rs`, `numeric.rs` | One engine: `state` transitions and steps, `drag` pointer/keys, `editor` TextEdit adapter, `display` drag surface. |
| ScrollArea | `scroll_area.rs`, `scroll_area/{chrome,rows}.rs` | Context: `context/scroll.rs`. |
| PanZoom | `pan_zoom.rs`, `pan_zoom/{state,show}.rs` | Local placement: `ui/region.rs`; input: `context/pan_zoom.rs`; uses shared visual placement. |
| ListBox | `list_box/` (`show`, `model`, `nav`, `select`, `paint`, `row`, `heights`) | Virtualized, keyed rows. |
| TreeView | `tree_view.rs`, `tree_view/` | Input: `context/tree.rs`. |
| Table / Grid | `table.rs`, `table/`, `grid.rs`, `grid/` | Table shares Grid columns + ScrollArea. |
| TabBar | `tab_bar.rs`, `tab_bar/{bar,item,options,layout,row,input,drag,zone,paint,style,state,access,view,menu,show}.rs` | Shared strip, reorder/release events and TabDropZone; used by Dock. |
| Dock | `dock.rs`, `dock/{model,normalize,operations,options,layout,anim,input,drag,drop_zones,float,focus_ring,paint,style,show,access,persist}.rs` | Application-owned tree; shared TabBar/SplitPane/Window; transient state in `context/containers.rs`. |
| SplitPane | `split_pane.rs`, `split_pane/{allocation,interaction,paint,show,style}.rs` | Input: `context/split.rs`. |
| ComboBox | `combo_box.rs`, `combo_box/{show,trigger,open,list,nav,choice,options,paint,style,access}.rs` | `show.rs` runs the stages in order: trigger (place, hit, AT node), open (dismissal/click/AT/keys → `Open`), list (Popup, filter TextEdit, virtualized rows), nav (highlight over enabled listed options), choice (one path for pointer/keyboard/AT). Uses Popup. |
| KeyBox | `key_box/` | Uses Popup. |
| ColorPicker | `color_picker.rs`, `color_picker/{show,row,panel,editor,fields,color,access}.rs` | Inline reveal or floating Window (`panel`); RGB/HEX `fields` are TextEdits with a draft adapter. |
| ContextMenu, MenuBar | `context_menu.rs`, `menu_bar.rs` (+ dirs) | Cascading menus, keyboard nav. |
| Modal / Dialog | `modal.rs`, `modal/{show,lifecycle,look,overlay,surface,access,dialog,geometry,parts}.rs` | `show::run` stages: lifecycle (mount, presence, keys, `close_reason` priority, finish), look (resolved appearance/spacing), overlay (dimming, blur, dismissal target), surface (chrome, header/body/footer, close button), access, measure. Stack: `context/modal.rs`. |
| Carousel | `carousel/` | Wheel routing: `context/carousel.rs`. |
| RadioGroup, SegmentedControl | `radio/`, `segmented/` | Options are one focus group (`Ui::focus_scope`, `focus_entry`, `navigated`); the grid's column rule stays in `radio/input.rs`. |
| Slider / DragValue | `slider.rs`, `slider/`, `drag_value.rs` | Keyboard: `context/keyboard.rs`. |
| Drag & drop, Reorder | `drag_drop/`, `reorder.rs` | Runtime in `context/drag/`. |
| CollapsingHeader | `collapsing_header.rs`, `disclosure.rs`, `disclosure/` | Shared row composition. |
| Effects/blur/motion | `effects.rs`, `blur.rs`, `motion.rs`, `moving.rs` | Subtree effects, moving elements. |

Adding a component usually touches: the component files, `components/mod.rs` exports, `lib.rs` re-exports, `Style`/theme tokens (`components/theme/controls.rs`, `resolve.rs`) if it is styled, a Context file only if it needs new input routing or retained state, `docs/content/docs/components/<name>.mdx` + `meta.json`, an example in `examples/`, tests in `tests/ui/`, and a scenario in `benches/` (see AGENTS.md).

## Tests, examples, docs, benches

- `tests/ui/` — behavior tests through the public API with `zaxis::testing` (main.rs lists modules; one `.rs` or dir per component); links and selection are in `tests/ui/text/` (its clipboard check is `#[ignore]`: `cargo test --test ui text::clipboard -- --ignored`). `tests/context/`, `tests/components/`, `tests/animation/`, `tests/text/`, `tests/shapes/`, `tests/renderer/` (needs a GPU; `ZAXIS_SKIP_GPU_TESTS=1`), `tests/images/`, `tests/app/`, `tests/ui/dock/` (model, layout, drag, floating, input and motion), `tests/accessibility/controls/dock.rs` (Dock semantics); `tests/accessibility/` (roles, names, requests and incremental updates of every component group through `zaxis::accessibility::testing::AccessTree`, no window; needs the `accesskit` feature). Top-level `layout_tests.rs`, `memory_tests.rs`, `motion_presets.rs`, `icons.rs`.
- Tests of private internals live in `tests/ui/*_internal.rs` (not in `src/`); `src/context/*_tests.rs` no longer exist.
- `tests/ui/nested_popup/{hierarchy,input,placement,lifecycle}.rs` — nested popups through the public API (chains, replacement, dismissal targeting, Escape/Tab, focus, edges, Grid/ScrollArea/visual/window move, cleanup); `tests/accessibility/overlays/popup.rs` — their layers in the tree.
- `tests/ui/keys/` (claims, order, lifecycle, priority through `Context::on_input`) and `tests/ui/focus_group/` (navigation, members, nested, compound members, lifecycle) are the public-path tests of key routing and focus groups.
- `tests/ui/pan_zoom/{camera,routing,integration,text,robustness}.rs` — camera math, published input, nested/deferred placement, text/IME/AT; `tests/renderer/embed/pan_zoom.rs` — actual GPU paint/hit/clip readback. `examples/pan_zoom.rs` + `pan_zoom/{scene,smoke}.rs` — public scenes and native input smoke.
- `examples/nested_popups.rs` + `nested_popups/editor.rs` — editor popup with TextEdit/ComboBox/KeyBox and a child panel, in a Modal, at a viewport edge (`--smoke-test` drives the pointer and keys).
- `examples/` — one per component plus `demo.rs`, `integration.rs` (custom host loop), `embed.rs` (zaxis inside a host's own wgpu device and 3D scene; `--pass`, `--smoke-test`), `custom_widget.rs` (knob, swatch and a `FocusGroup` panel; `--smoke-test` sends keys through `Context::on_input` and checks the model), `custom_animation.rs`, `accessibility.rs` (a form for screen readers; `--smoke-test` checks the tree without a window); multi-file examples use a same-named dir (`embed/`, `accessibility/`, `settings/`, `multi_window/`, `tree/`, `modals/`, `drag_and_drop/`, `split_pane/`, `animations/`, `dock/` for `examples/dock.rs`, `--smoke-test`).
- `benches/performance.rs` + `benches/support/scene/{build,cases,input,verify}.rs` — the single perf harness; add scenes here. `benches/support/access.rs` holds the accessibility cases (idle off/on, one change, full tree, text); component probes are one file each in `benches/support/` (`keys` for claimed keys and focus groups: idle off/on, claimed, unclaimed, group step, burst, many groups, reorder, lifecycle; `dock` for Dock idle, drag, split, focus, theme and lifecycle cases, `combo_box`, `modal`, `number` for NumberInput/DragValue, `text_area`, ...).
- `benches/support/popup.rs` — popup scenes: closed triggers, one popup, chains of depth 2/4/max, open/close, Escape, wheel, moving anchor, model change, many roots.
- `benches/support/pan_zoom.rs` — public camera/input/culling/lifecycle probes; `benches/support/scene/cases/catalog.rs` — shared scenario catalog.
- `docs/content/docs/` — mdx docs (`app`, `layout`, `input`, `repaint`, `renderer`, `protocol`, `animation`, `style`, `limitations`, `components/*`). Update with behavior changes.
- `prompts/` — historical task briefs per component; not runtime code, ignore unless asked.

## Search tips

- Public API of a type: `rg "pub struct Name|impl .* for Name|impl Name" src`.
- Who consumes a hit action / input path: `rg "HitAction::Name" src` (interaction.rs defines, pointer.rs/keyboard.rs/gesture.rs dispatch).
- Retained per-widget state is keyed by `Id` in the Context owner of its family (`text_input`, `values`, `menus`, `table`, `tree`, `split`, `containers`, `visual`); start from the component's `state.rs` or that owner. Cleanup rules live in each owner's `retire`; `frame.rs` only orders the stages.
- Feature gates: `bundled-emoji`, `bundled-weights`, `bundled-monospace`, `bundled-icons`, `image-gif`, `image-tiff`, `accesskit`, `accesskit_unix` (Cargo.toml).
