# Hotreload Frame Staleness

When Dioxus hotreloads a template that contains a changed static attribute, the entire frame tree is torn down and rebuilt with new frame IDs. UI code caching those IDs (e.g. `LoginUi`) becomes stale. The root cause is in our renderer's template storage, not in Dioxus itself.

## Finding

Dioxus hotreload converts changed static attributes to dynamic ones and sends a new `HotReloadedTemplate` with structurally different roots. `diff_node` compares old vs new `Template` — they don't match — and emits `replace_node_with`, which rebuilds the entire tree with new IDs.

Our `GameUiRenderer` stores templates in `Vec<Template>` with a linear `contains()` scan. When a hotreloaded template arrives for the same `rsx!` call site, `contains()` returns false (different structure/pointers) and it is pushed as a second entry. There is no way to identify it as a replacement for the original template at that call site.

## Root Cause

`templates: Vec<Template>` in `src/ui/dioxus_renderer.rs` has no key-based identity. Hotreloaded templates and their originals coexist with no connection, so frame ID associations are lost on every hotreload.

## Resolution

Replace `Vec<Template>` with `HashMap<TemplateGlobalKey, Template>`:
1. In `DioxusScreen::sync()`, capture `TemplateGlobalKey` for each template in the hotreload message before calling `apply_changes`.
2. Thread keys through to `load_template` so they are available at insertion time.
3. On match: reuse existing frame IDs and update attributes in-place.
4. On miss: create new frames as before (first render path).

`TemplateGlobalKey` encodes `{ file, line, column, index }` — the same key `dioxus-devtools` uses when setting the `GlobalSignal`.

## Sources

- [hotreload-frame-stability.md](../../hotreload-frame-stability.md) — full root cause analysis and proposed fix

## See Also

- [[dioxus-renderer]] — GameUiRenderer architecture (if page exists)
