# Hotreload Frame Stability

## Problem

When Dioxus hotreload changes a static attribute value (e.g. `width: 320.0` → `width: 350.0`), the dx CLI converts the static attribute to a dynamic one and sends a new `HotReloadedTemplate` with different `roots`. This produces a new `Template` with different pointers and structure. Dioxus's `diff_node` compares old vs new Template — they don't match — so it emits `replace_node_with`, tearing down the entire frame tree and rebuilding it with new frame IDs. `LoginUi`'s cached frame IDs become stale.

## How rsx! Templates Work

Each `rsx!` call site generates (in debug builds):

1. **`__TEMPLATE_ROOTS`** — `&'static [TemplateNode]`, the compiled-in template structure
2. **`__ORIGINAL_TEMPLATE`** — `OnceLock<HotReloadedTemplate>`, built via `HotReloadedTemplate::new(... __TEMPLATE_ROOTS)`. All dynamic mappings are `Dynamic(id)` — they delegate to the runtime `DynamicValuePool`.
3. **`__TEMPLATE`** — `GlobalSignal<Option<HotReloadedTemplate>>`, keyed by `GlobalKey::File { file, line, column, index }`. Starts as `None`.

On render:
- Signal is `None` → uses `__ORIGINAL_TEMPLATE` (compiled-in Template)
- Signal is `Some(hotreloaded)` → uses the hotreloaded Template

## What Hotreload Does

`dioxus_devtools::apply_changes` iterates `msg.templates`, looks up the `GlobalSignal` by `TemplateGlobalKey { file, line, column, index }`, and calls `signal.set(Some(new_hot_reloaded_template))`.

The new `HotReloadedTemplate` has:
- **New roots** where changed static attrs become `TemplateAttribute::Dynamic { id }`
- **New `template: Template`** with fresh `Box::leak`'d `node_paths`/`attr_paths`
- `HotReloadDynamicAttribute::Named(...)` entries for the converted attrs

This new Template doesn't match the original → `diff_node` triggers full replacement.

## Template Comparison

`Template::eq` (dioxus-core `nodes.rs:329`):
- `static_items_merged()` true → **pointer comparison** of roots/node_paths/attr_paths
- `static_items_merged()` false → **structural comparison** (value equality)

We force `opt-level = 0` for dioxus-core (Cargo.toml) to get structural comparison. But structural comparison still fails because the roots genuinely differ (Static → Dynamic conversion).

## Root Cause: Our Renderer

`GameUiRenderer` stores templates in a `Vec<Template>` (line 50 of `dioxus_renderer.rs`). In `load_template`:

```rust
if !self.renderer.templates.contains(&template) {
    self.renderer.templates.push(template);
}
```

This is a linear scan with no key-based lookup. When hotreload sends a new Template for the same call site, `Vec::contains` returns false (different structure/pointers) → it gets pushed as a new entry. The old and new templates coexist with no connection. There's no way to recognize that the new template replaces the old one for the same `rsx!` call site.

**This is our bug.** We should be using `HashMap<TemplateGlobalKey, Template>` so we can:
1. Look up the existing Template by its source location key
2. Recognize that a hotreloaded template is the same logical template
3. Preserve frame ID associations across the replacement

## Fix

Replace `templates: Vec<Template>` with `templates: HashMap<TemplateGlobalKey, Template>`.

**Problem**: `load_template` receives `Template`, not `TemplateGlobalKey`. The key is only available in `apply_changes` (from `HotReloadTemplateWithLocation`). We need to thread it through.

**Approach**:
1. In `DioxusScreen::sync()`, before calling `apply_changes`, capture the `TemplateGlobalKey` for each template in the hotreload message
2. Store these keys on `GameUiRenderer` so they're available when `load_template` fires during `render_immediate`
3. In `load_template`, match the incoming Template to a key and look up the existing entry in the HashMap
4. If found: reuse the existing frame IDs instead of creating new ones. Update attributes in-place.
5. If not found: create new frames as before (first render path)

## Key Files

- `src/ui/dioxus_screen.rs` — `DioxusScreen::sync()`, calls `apply_changes`
- `src/ui/dioxus_renderer.rs` — `MutationApplier` implements `WriteMutations` (load_template, replace_node_with, set_attribute)
- `src/ui/registry.rs` — `FrameRegistry`, frame storage with name-based lookup
- `src/ui/mod.rs` — `ui_resource!` macro, generates `LoginUi::resolve()`
- `src/ui/screens/login_component.rs` — login screen rsx components

## Dioxus Internals (0.7.3)

- `dioxus-core/src/diff/node.rs:32` — `if self.template != new.template { return self.replace(...) }`
- `dioxus-core/src/nodes.rs:329` — `Template::eq` pointer vs structural comparison
- `dioxus-core/src/hotreload_utils.rs:264` — `DynamicValuePool::render_with()` uses `hot_reload.template`
- `dioxus-core/src/hotreload_utils.rs:366` — `HotReloadedTemplate::new()` creates Template with Box::leak'd paths
- `dioxus-devtools/src/lib.rs:12` — `apply_changes()` sets GlobalSignal per TemplateGlobalKey
- `dioxus-rsx/src/template_body.rs:139-226` — rsx! macro expansion (debug builds)
