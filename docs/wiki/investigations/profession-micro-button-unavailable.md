# Profession micro-button unavailable

**VERIFIED:** The unavailable message came from a client-side disabled configuration, not from a missing ProfessionSnapshot. **VERIFIED in source and tests:** The native ProfessionsBookFrame and empty/learned row projection are implemented. **UNVERIFIED:** This lane has no live screenshot proof.

## Root cause

- **VERIFIED:** In shipped engine source `3d8feea93`, `godot/ui-model/src/micro_menu.rs` assigned `ProfessionMicroButton` the `SYSTEM_DISABLED` reason, and `micro_button_click` sent that action to `unavailable_message`. That path returns “This system is currently disabled.”
- **VERIFIED:** The same source includes `db2` in `scripts/player_bundle.py`'s `DATA_DIRS`; no bundle omission was found for the profession metadata used by the new book projection.
- **VERIFIED:** Server source at `aebaea6942` loads the selected character's persisted `PlayerProfessions` and sends an owner `ProfessionSnapshot` on entry and changes. No server change was needed for this client-side disabled action.

## Fix and proof

- **VERIFIED:** Engine commits `3a6dfaba8`, `dd71252f0`, `d2ebd1d2f`, and `aab84e3a4` add the separate ProfessionsBookFrame, snapshot/DB2 projection for learned rows, ProfessionMicroButton availability and open-state art, and shared close lifecycle integration.
- **VERIFIED by targeted tests:** `game-engine-ui-professions --test integration professions_book_` passed two tests: empty rows in Modern and Forever, plus learned Tailoring/Cooking rows in Forever.
- **VERIFIED by targeted test:** `game-engine-ui-model --test integration professions_micro_button_` passed one test for availability and pushed art.
- **VERIFIED by targeted test:** `game-engine-godot professions_account_dispatches_loading_snapshot_and_skill_refresh` passed one test for the existing owner snapshot dispatch path.
- **VERIFIED:** `cargo fmt --all -- --check` passed on the engine slot. The targeted `game-engine-godot` account-dispatch test passed.
- **VERIFIED:** Native `engine-build.py --root <slot> --cli` exited 0 and produced the extension and CLI. The exact `/home/osso/Projects/world-of-osso/game-engine/data/cache/professions.sqlite` path was absent before and after one exact-path unlink attempt and the build.
- **GATE BLOCKED:** The first native build recorded a C warning from transitive `libquickjs-sys`, generated `quickjs.c:27172`: assignment discards `const` qualifier. No warning suppression or dependency patch was made. The zero-warning gate is not met.
- **UNVERIFIED:** This lane did not establish the live book toggle or row rendering in a running client and did not capture empty/learned screenshots. The NPC/source was not verified. Do not describe database spawn records as trainer proof.
- **BLOCKED:** The user directed that private-server provisioning stop. Do not start a server/client or claim live screenshots under this directive.

## Sources

- [Professions contract](../../specs/professions-frame.md)
- [Professions UI system](../systems/professions-ui.md)
- [`micro_menu.rs`](../../../godot/ui-model/src/micro_menu.rs) — current action routing; shipped disabled assignment was at `3d8feea93`.
- [`player_bundle.py`](../../../scripts/player_bundle.py) — bundle data-directory selection.
