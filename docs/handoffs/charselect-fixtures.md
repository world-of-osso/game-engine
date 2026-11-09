# Character-select fixture readiness

## Decision

Fix only `world_reconnect_flow.gd`, `world_transfer_flow.gd`, and the transfer fixture's outer deadline. Authentication state is not native UI readiness. Wait until the authenticated roster has its expected count and `CharCard_0` and the enabled `EnterWorld` button exist and are visible in the scene tree. Keep the real pointer actions and all reconnect/transfer, world reset, terrain and water assertions. Match the existing 1920×1080 native fixture viewport. Use a bounded preparation deadline, not a fixed frame delay or synthetic Enter World request.

Do not change product startup, selectors, character-select reveal, or server behavior. [Character-select loading](../specs/character-select-loading.md) requires the loading cover until the campsite and selected model are ready.

## Evidence

- VERIFIED on engine `618f5ab01`: original `native_reconnect_fixture` fails with “Authenticated selected character card and Enter World action missing”. A diagnostic copy observes raw `account_state().screen == CharacterSelect` while `CharacterSelectUI`, `CharCard_0` and `EnterWorld` are null.
- VERIFIED on the same engine: a rendered Forever client authenticated to an owned server built from game-server `88959866b4` and shared-protocol `7fa253f`, UDP 5964. The settled screenshot shows selected `Charfixture` and Enter World, both visible. Original: `/home/osso/.worktrees/charselect-fixtures-evidence/charselect-forever-master.png` (1920×1080).
- INFERRED, SOURCE-ONLY history pointer: `23c06168e` (September 30, 2026) added `receive_account_during_startup`, which updates session state while screen events wait for asset initialization. This does not identify a first bad runtime commit. Its parent cannot build with the available helper/siblings (`ui-toolkit-godot-conversion` is absent). The lead waived historical binary bisection on November 9, 2026.

## Acceptance

The existing real-process fixtures are the acceptance tests. Commit `878dd988d` tightens their control assertions to tree visibility and enabled Enter World while retaining RED. The implementation now waits up to 60 seconds for the expected authenticated roster and the revealed controls in the scene tree; both fixtures use a 1920×1080 viewport. The transfer server-side process deadline is 180 seconds so native startup, scene reveal and world readiness fit within its bound.

VERIFIED: Both `native_reconnect_fixture` and `native_transfer_fixture` pass through the owned headless-client helper; reconnect reaches `RECONNECTED`, and transfer completes every phase and receives one `WorldPortAck`. VERIFIED: debug and release workspace checks exit 0 with zero compiler warnings, and `cargo fmt --all --check` exits 0. Logs and the Forever screenshot are in `data/diagnostics/charselect-fixtures-2026-11-09/`. Commit `d631259b9` contains the fixture fix and wiki update; commit `878dd988d` contains the RED acceptance assertions. No product source changed; the branch is local-only.
