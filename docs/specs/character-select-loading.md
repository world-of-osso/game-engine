# Character-select loading

Native character selection prepares its campsite and selected character before revealing the roster screen. Host: `godot/rust/src/character_select.rs` and `lib.rs`. [Preparation measurements](../wiki/investigations/world-entry-stalls.md).

## What it must do

- [x] Cover selected-character preparation with the loading screen; never expose a sky-only character-select frame.
- [x] Report Loading to automation while the selected scene is preparing; CharSelect means the scene can be revealed.
- [x] Reveal only after the selected model, requested terrain, sky and selected campsite-object drawables are attached. Preparation errors remain visible on the loading screen and are logged; failure never reveals an incomplete scene.
- [x] Keep empty-roster creation available; it must not await a nonexistent selected model.
- [x] Reuse loaded player outfit catalogs across selection and world entry, with isolated data-root/product namespaces. Prepare the default Retail namespace from client start instead of importing CSV rows on the selection critical path.

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

- [x] Published-base Windows process-frame regression and inspected 1920×1080 login/loading/camp/world captures accepted on 2026-10-14. Covered selection: Windows 1.706 s, Linux 2.398 s; differing release/debug builds do not establish cross-platform speed parity.
- [x] Current-master Linux process-frame reveal regression after preserving its SFID/SKID loader changes: PASS at `92e8f2d8e`, inspected 1920×1080 camp/model. Existing sky skin525143 bytes authenticate against current CASC and require an FDID alias in packaging; private proof supplied that alias without a loader fallback.

## Out of scope

- Publication and production access remain prohibited by the security hold.
