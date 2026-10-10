# Roadmap

What is planned for zaxis. Items move to [CHANGELOG.md](CHANGELOG.md) when they ship.
Order inside a section is rough priority, not a promise. Public APIs may change between
development releases.

Legend: **Next** is being designed or built now. Everything else is an idea.

## Next

- [x] **RadioGroup**: radio buttons with an accent dot and soft halo, roving focus, arrow
      navigation, horizontal and grid layouts, descriptions, `Field` validation
- [ ] **Command palette**: modal fuzzy search over commands with shortcuts, groups, recent
      items and keyboard-only operation

## Shipped recently

- [x] `SegmentedControl` with raised and outlined variants
- [x] `MenuBar` and hamburger menu with cascading submenus, check marks and shortcuts
- [x] `Badge`, `IconTabs`, `KeyBox`, `Toast`

## Components

- [x] `ListBox`: virtualized list with single and multi selection, shift/ctrl ranges,
      type-ahead (shared base for the palette, trees and tables)
- [x] `Hyperlink` and selectable, copyable static text
- [ ] `Breadcrumbs` with overflow menu
- [ ] `DatePicker`, `TimePicker`, `DateRangePicker`
- [ ] `TagInput` / chip input with suggestions
- [ ] `Stepper` / `NumberStepper` buttons and `Rating`
- [ ] `Pagination`
- [ ] `Accordion` (exclusive groups on top of `CollapsingHeader`)
- [ ] `Popover` with arrow and rich content, `HoverCard`
- [ ] `Notification center` (persistent toast history, actions, grouping)
- [ ] `Banner` / `Alert` (inline status messages)
- [ ] `Avatar` and avatar group
- [x] `Kbd` shortcut hints rendered per platform (formatter; no component yet)
- [ ] `Stepper` wizard / multi-step form
- [x] `Carousel` / pager with snapping (stack of cards and photo slider)
- [ ] `Toolbar` with overflow and grouped buttons, `ToggleButton` and toggle groups
- [ ] `StatusBar`
- [x] `Dock` / IDE split panels and draggable, closable, reorderable tabs; floating Window layers, spring focus ring and layout persistence
- [ ] `PropertyGrid` / inspector (typed rows, categories, reset to default)
- [ ] `Form` layout helpers (label columns, validation summary)
- [ ] `FileTree` with icons, rename in place and context actions
- [ ] `Minimap` for long documents
- [ ] `Gauge`, `Knob`, `RangeSlider` (two thumbs), `ColorSwatch` palettes
- [ ] `Spinner` variants and indeterminate progress presets
- [ ] `Empty state` and `Placeholder` blocks
- [ ] `Tour` / onboarding hints anchored to widgets
- [ ] `Rating`, `Stat` tiles, `Sparkline`
- [ ] `Chart` primitives: line, bar, area, histogram, with hover tooltips
- [ ] `Timeline` and `Gantt`-style track view
- [x] `PanZoom` container with application-owned camera, local controls and addressed gestures;
      scene grids, snapping and object selection remain application responsibilities
- [ ] `NodeGraph` editor primitives (ports, links, minimap)
- [ ] `Terminal` view for ANSI output
- [ ] `Diff view` with inline and side-by-side modes
- [ ] `Markdown` renderer (headings, lists, code, links, images)
- [ ] `Table` v2: variable row height, column reordering and hiding, pinned columns, cell
      editing, grouping and filtering

## Menus and commands

- [ ] Alt-key mnemonics and `F10` menu focus for `MenuBar`
- [ ] Hover delay and pointer-direction tolerance for submenus
- [ ] Icons, radio items and custom widgets inside menu rows
- [ ] Cascading submenus in `ContextMenu` on top of the shared panel engine
- [ ] Native OS menu bar (macOS) behind an optional feature
- [x] Application-level `Action` / `Keymap` registry: commands with ids, shortcuts,
      enabled state, shared by menus, palette and toolbars
- [x] Shortcut conflict detection and user rebinding UI (`KeyBox` based)
- [x] Nested popups: a popup opened from inside another keeps its parent (one branch per context, depth-limited)
- [ ] Several independent popup branches at once (tooltip-like pinned popups)

## Text and editing

- [ ] Mixed styles inside one string: colors, weights, links
- [ ] Rich text spans API (foundation for syntax highlighting, markdown and links)
- [ ] Code editor mode for `TextEdit`: line numbers, current-line highlight, gutter
      markers, bracket matching, indent guides, auto-indent
- [ ] Syntax highlighting with pluggable highlighters
- [ ] Search and replace (regex, case, whole word), multiple carets, column selection
- [ ] Code folding, completion popup, inline diagnostics
- [ ] Italic and optical sizes, extra OpenType features, runtime font setter
- [ ] Per-script font fallback controls and font fallback weights
- [ ] Spell-check hooks
- [ ] Undo history across several editors with transaction grouping
- [ ] Text `Selection` for static labels and cross-widget selection

## Layout and virtualization

- [ ] Variable-height virtualization (chat, logs, diffs with wrapping)
- [ ] Flex-style layout helpers: wrap, grow/shrink, baseline alignment
- [ ] Responsive containers (breakpoints) and container queries
- [ ] Sticky headers and footers in scroll areas
- [ ] Masonry / flow grid
- [ ] Constraint-based `Layout` for forms and inspectors
- [ ] Scroll snapping and smooth programmatic scrolling API

## Windows and platform

- [x] OS file drag-in (`DroppedFile` / `HoveredFile`) wired to `DropTarget`
- [x] Native file dialogs (open, save, pick folder) as an optional feature
- [ ] Clipboard images and rich clipboard formats
- [ ] Cross-window drag and drop
- [ ] Window state API: reset, forced reposition and resize, save and restore geometry
- [ ] Confirm-before-exit hook for OS close requests across all platforms
- [ ] System tray, notifications and dock/taskbar badges
- [ ] Global hotkeys outside the window
- [ ] Always-on-top, borderless and tool windows presets
- [ ] High-contrast and system theme following (light/dark/accent change at runtime)
- [ ] Touch and pen input, gestures (pinch, pan)
- [ ] Gamepad navigation for focus
- [ ] Wayland and X11 polish, per-monitor DPI changes while dragging between screens

## Accessibility and internationalization

- [x] OS accessibility tree via AccessKit (roles, names, values, focus)
- [x] Screen-reader announcements for toasts and validation
- [ ] Full keyboard operation audit and visible focus everywhere
- [ ] Right-to-left layout mirroring for components
- [ ] Locale-aware number, date and plural formatting helpers
- [ ] Message catalogs integration for labels
- [ ] Reduced-transparency and increased-contrast modes

## Theming and style

- [ ] Theme editor example with live preview and export
- [ ] More built-in themes (high contrast, warm, solarized, catppuccin-style)
- [ ] Theme loading from files (TOML/JSON) and hot reload during development
- [ ] Design-token documentation page generated from the code
- [ ] Per-component state layers (focus, selected, drag) unified across controls
- [ ] Icon packs beyond Lucide, icon font support, animated icons

## Animation and effects

- [ ] Shared-element transitions between pages and lists
- [ ] Layout animations for reorder, insert and remove across all lists
- [ ] Scroll-linked animations and parallax
- [ ] Gesture-driven springs (flick, rubber banding) for scroll areas
- [ ] Particle and confetti effects module
- [ ] Skeleton shimmer presets for more shapes

## Rendering and performance

- [x] Render into an existing wgpu pass (embedding in game engines and other apps)
- [ ] Public dirty-region texture upload API
- [ ] Glyph atlas eviction and GPU buffer shrinking
- [ ] Shader-based rounded clipping for arbitrary content
- [x] Custom shaders and materials for widgets
- [ ] 3D viewport embedding helper
- [x] Overlay inside another process by hooking present (`z-hook`: Vulkan layer on Linux checked; DXGI on Windows written, not run)
- [ ] z-hook: run and verify the Windows DXGI paths; OpenGL hooks; input for Wayland hosts
- [ ] Frame pacing diagnostics and a built-in profiler overlay
- [ ] Software renderer fallback when no GPU adapter is available
- [x] WebAssembly target for the library (browser runner)
- [ ] Android and iOS exploration

## Images and media

- [ ] Animated GIF / WebP / APNG playback
- [ ] External SVG resources and network image loading (optional)
- [ ] Image viewer with zoom, pan, rotate and crop
- [ ] Video frame widget with a pluggable decoder
- [ ] Audio waveform widget

## Developer experience

- [ ] Widget inspector overlay (hit regions, ids, layout bounds) in `DebugOverlay`
- [ ] Record and replay of input for tests and bug reports
- [ ] Screenshot tests for components (golden images)
- [ ] Snapshot/headless renderer for CI
- [ ] Cargo template and `cargo generate` starter
- [ ] Live documentation playground in the docs site
- [ ] Migration guides between development releases
- [ ] Persistence helpers: optional serialization of window geometry and UI state
- [ ] Localized documentation
