# Character-select fixture readiness

## Decision

Fix only `world_reconnect_flow.gd` and `world_transfer_flow.gd`. Authentication state is not native UI readiness. Wait until the authenticated roster has its expected count and `CharCard_0` and the enabled `EnterWorld` button exist and are visible in the scene tree. Keep the real pointer actions and all reconnect/transfer, world reset, terrain and water assertions. Match the existing 1920×1080 native fixture viewport. Use a bounded preparation deadline, not a fixed frame delay or synthetic Enter World request.

Do not change product startup, selectors, character-select reveal, or server behavior. [Character-select loading](../specs/character-select-loading.md) requires the loading cover until the campsite and selected model are ready.

## Evidence

- VERIFIED on engine `618f5ab01`: original `native_reconnect_fixture` fails with “Authenticated selected character card and Enter World action missing”. A diagnostic copy observes raw `account_state().screen == CharacterSelect` while `CharacterSelectUI`, `CharCard_0` and `EnterWorld` are null.
- VERIFIED on the same engine: a rendered Forever client authenticated to an owned server built from game-server `88959866b4` and shared-protocol `7fa253f`, UDP 5964. The settled screenshot shows selected `Charfixture` and Enter World, both visible. Original: `/home/osso/.worktrees/charselect-fixtures-evidence/charselect-forever-master.png` (1920×1080).
- SOURCE-ONLY history pointer: first-parent source bisection locates early auth polling at `23c06168e` (September 30, 2026). `receive_account_during_startup` updates session state while screen events wait for asset initialization. This is not a historical runtime-fixture bisect. Its parent cannot build with the available helper/siblings (`ui-toolkit-godot-conversion` is absent). The lead explicitly waived historical binary bisection on November 9, 2026.

## Acceptance

The existing real-process fixtures are the acceptance tests. The acceptance commit tightens their control assertions to tree visibility and enabled Enter World without changing their premature wait, retaining RED. Implement the readiness wait only after that commit and the design handoff.

Run both `native_reconnect_fixture` and `native_transfer_fixture` through the owned headless-client helper with protocol `7fa253f`; all existing phase and water checks must pass. Run debug/release Cargo checks with zero warnings and formatting. Commit locally on `charselect-fixtures`; no fetch, push, canonical source edits or master changes.
