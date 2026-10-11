# Native battleground lifecycle

Native Retail Mainline Warsong Gulch PvP UI, backed by existing shared PvP messages and the [server lifecycle contract](../../../game-server/docs/specs/battleground-lifecycle.md). Source UI lives under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_PVPUI/Mainline`, `Blizzard_GroupFinder/Mainline/PVPHelper.*` and `Blizzard_PVPMatch`. [Implementation and proof](../wiki/systems/battlegrounds.md).

## What it must do

- [x] Expose the source-backed WSG specific queue, leave queue and server refusals. Requests remain pending until authority responds; errors preserve the retained snapshot and release pending.
- [x] Show an invitation independently of queue-window visibility. Enter Battle/Leave Queue echo match identity; accepted invitations stop offering another acceptance.
- [x] Render usable authored type36 flags and display server flag/score updates after entry. No locally invented objective outcome.
- [x] Display retained roster/objective columns, All/Alliance/Horde filters and winner/defeat results. Ended opens results even after closing the live scoreboard; unknown honor is absent and uninstrumented combat columns are explicitly unknown.
- [x] Submit leave for an accepted match, including Waiting; the server decides whether arrival is complete and performs the authoritative origin transfer. Modern and Forever share the same native behavior.

Retail source geometry: PVPUIFrame563×428, PVPReadyDialog306×210 with115×22 accept/leave buttons, PVPMatchScoreboard1024×420, PVPMatchResults1024×620. WSG is the only exposed selectable mode; unsupported queue modes are not fabricated.

## How it works

- [Native battleground system and bounded proof](../wiki/systems/battlegrounds.md).
- [Server source/content/transfer authority](../../../game-server/docs/specs/battleground-lifecycle.md).

## Implementation inventory

- `godot/ui-model/src/battleground.rs` — view, pending/actions, native panels/HUD, filters and results.
- `godot/rust/src/battleground.rs`, `account.rs`, `lib.rs` — mounted native canvas, transport event dispatch and lifecycle.
- `godot/rust/src/game_objects.rs` — normal source-display flag rendering and interaction.
- `godot/network/src/lib.rs`, `ipc/wire.rs`, `godot/cli/src/{main,requests}.rs` — PvP receipt and typed diagnostic access.
- `godot/network/examples/native_battleground_peers.rs`, `godot/tests/battleground_live.gd` — real-UDP support accounts and native-pointer driver.

## Tests asserting this spec

- `godot/ui-model/tests/battleground.rs` — identity/pending/refusal behavior; both-skin invitation, HUD, roster filters, rewards and closed-scoreboard/end transition.
- `godot/rust/src/game_objects.rs::tests::battleground_flags_render_and_offer_object_use` — source flag type remains visible/usable; six related object tests pass.
- Server `crates/server/src/maps/tests.rs::ten_warsong_players_accept_over_the_channel_then_ack_one_reserved_map` — retained ten-player typed-channel entry, physical source objectives, outcome and origin return.
- Native private UDP5710 runs,2026-10-11: Modern and Forever, ten real accounts each; source-position travel acceleration, native object use, three captures, complete roster/outcome and all-player origin-map returns. Native invite decline in both skins; Modern automatic end-results regression. [Receipt boundaries](../wiki/systems/battlegrounds.md#bounded-live-proof).

## Known gaps (current cycle)

- [ ] Integration review remains lead-owned. Scoped UI/object tests, native builds, helper cargo check and changed-Rust formatting pass; full suites are periodic master gates, not per-agent branch runs. No master merge performed.
- [ ] Exact PvP-specific textures/icons/chrome parity is not certified; current native panels reuse existing skin-aware frame/button art.

## Out of scope

Numeric preparation/invitation/match/drop/reset/exit timers and honor rewards lack source policy; never fabricate countdowns or grants. Current server combat counters are not instrumented; native columns do not claim measured zero. Other battleground/rated modes, durable reconnect/origin recovery and Retail dropped-flag visual parity are not implemented by this UI scope. See the server contract for those boundaries.
