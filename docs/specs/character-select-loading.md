# Character-select loading

Native character selection prepares its campsite and selected character before revealing the roster screen. Host: `godot/rust/src/character_select.rs` and `lib.rs`. [Preparation measurements](../wiki/investigations/world-entry-stalls.md).

## What it must do

- [ ] Cover selected-character preparation with the loading screen; never expose a sky-only character-select frame.
- [ ] Reveal only after the selected model, requested terrain, sky and selected campsite-object drawables are attached. Preparation errors remain visible on the loading screen and are logged; failure never reveals an incomplete scene.
- [ ] Keep empty-roster creation available; it must not await a nonexistent selected model.
- [ ] Reuse loaded player outfit catalogs across selection and world entry, with isolated data-root/product namespaces. Prepare the default Retail namespace from client start instead of importing CSV rows on the selection critical path.

## How it works

- [Character preparation and profiling](../wiki/investigations/world-entry-stalls.md#character-select-and-enter-world-loads--2026-10-01).

## Implementation inventory

- `godot/rust/src/assets/player.rs`: shared player outfit catalogs, default Retail prewarm and asynchronous player preparation.
- `godot/rust/src/character_select.rs`: preparation lifetime and selected-model ownership.
- `godot/rust/src/character_select/background.rs` and `objects.rs`: campsite drawable readiness.
- `godot/rust/src/lib.rs`: loading/roster visibility projection.

## Tests asserting this spec

- `assets::player::catalog_lifetime_tests`: real prepared equipment remains usable after its CSV source leaves.
- `godot/tests/charselect_scene_reveal.gd`: process-frame regression rejecting roster reveal before actual terrain and character meshes.

## Known gaps (current cycle)

- [ ] Windows and Linux cold/warm preparation timing, gated 1920×1080 Windows login-to-ready sequence and in-world appearance acceptance.

## Out of scope

- Publication and production access remain prohibited by the security hold.
