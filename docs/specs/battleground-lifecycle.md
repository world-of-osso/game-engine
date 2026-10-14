# Native battleground lifecycle

Native Retail Mainline battleground PvP UI, backed by existing shared PvP messages and the [server lifecycle contract](../../../game-server/docs/specs/battleground-lifecycle.md). Source UI lives under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_PVPUI/Mainline`, `Blizzard_GroupFinder/Mainline/PVPHelper.*` and `Blizzard_PVPMatch`. [Implementation and proof](../wiki/systems/battlegrounds.md).

## What it must do

- [x] Expose the source-backed WSG specific queue, leave queue and server refusals. Requests remain pending until authority responds; errors preserve the retained snapshot and release pending.
- [x] Show an invitation independently of queue-window visibility. Enter Battle/Leave Queue echo match identity; accepted invitations stop offering another acceptance.
- [x] Render usable authored type36 flags and display server flag/score updates after entry. No locally invented objective outcome.
- [x] Display retained roster/objective columns, All/Alliance/Horde filters and winner/defeat results. Ended opens results even after closing the live scoreboard; unknown honor is absent and uninstrumented combat columns are explicitly unknown.
- [x] Submit leave for an accepted match, including Waiting; the server decides whether arrival is complete and performs the authoritative origin transfer. Modern and Forever share the same native behavior.

Retail source geometry: PVPUIFrame 563×428, PVPReadyDialog 306×210 with 115×22 accept/leave buttons, PVPMatchScoreboard 1024×420, PVPMatchResults 1024×620. The Mainline SpecificScrollBox selects implemented source-backed modes: WSG 1014, Arathi 1018 and Gilneas 120. Selection is retained in the queue request; unsupported modes are not fabricated. Node maps display authoritative POI states, 1500-resource limits and measured Bases Assaulted/Defended columns rather than WSG flags/captures.

## How it works

- [Native battleground system and bounded proof](../wiki/systems/battlegrounds.md).
- [Server source/content/transfer authority](../../../game-server/docs/specs/battleground-lifecycle.md).

## Implementation inventory

- `godot/ui-model/src/battleground.rs` — view, pending/actions, native panels/HUD, filters and results.
- `godot/rust/src/battleground.rs`, `account.rs`, `lib.rs` — mounted native canvas, transport event dispatch and lifecycle.
- `godot/rust/src/game_objects.rs` — normal source-display flag rendering and interaction.
- `godot/network/src/lib.rs`, `ipc/wire.rs`, `godot/cli/src/{main,requests}.rs` — PvP receipt and typed diagnostic access.
- `godot/network/examples/native_battleground_peers.rs`, `godot/tests/battleground_live.gd` — real-UDP support accounts and native-pointer driver. Private peers accept `peer:<index>:<action>` for exact actor staging: normal `attack:<NPC-entry>:<team>`, `equip:<bag>:<slot>`, `unequip-offhand:<bag>:<slot>` and `cast:<spell-ID>:<target-entity>`. `spec:<specialization-ID>` retains the server's class/level authority. `traits:<JSONfile>` submits a source-derived normal `CommitTraitConfig`; ordered talent lifecycle snapshots/results and learned/known spell receipts are retained in each peer's diagnostic JSON. This is fixture control, not a spell grant or a substitute for server acceptance. Casts require `BG_LOS_DIR` pointing to the same read-only eye table and geometry bake as the private server; witnesses use actual replicated bodies and clear baked rays, never fabricated LOS. Inventory refusals fail explicitly; passive player health and inventory/equipment receipts distinguish alive gameplay from dead-before-request runs. These commands do not alter NPC health, damage, scores or outcomes.

Private peer state files must expose complete JSON receipts throughout publication; concurrent readers may retain the preceding complete snapshot while the next snapshot is written. Publication errors remain explicit. Gameplay, connection deadlines, and cast timers remain unchanged.

The private native fixture's read-only `visuals` action reports actual camera position, local-player animation identity and mesh transforms/bounds/visibility. `portrait` reads the existing renderer receipt for a named portrait frame, excluding its image object from JSON. Neither changes rendering state nor substitutes those receipts for inspected pixels.

## Tests asserting this spec

- `godot/network/examples/native_battleground_peers.rs::tests::peer_snapshot_readers_always_observe_complete_json_during_publication` — real concurrent file reads retain complete receipts across successive publications.
- `godot/ui-model/tests/battleground.rs` — identity/pending/refusal behavior; both-skin invitation, HUD, roster filters, rewards and closed-scoreboard/end transition.
- `godot/rust/src/game_objects.rs::tests::battleground_flags_render_and_offer_object_use` — source flag type remains visible/usable; six related object tests pass.
- Server `crates/server/src/maps/tests.rs::ten_warsong_players_accept_over_the_channel_then_ack_one_reserved_map` — retained ten-player typed-channel entry, physical source objectives, outcome and origin return.
- Native private UDP5710 runs,2026-10-11: Modern and Forever, ten real accounts each; source-position travel acceleration, native object use, three captures, complete roster/outcome and all-player origin-map returns. Native invite decline in both skins; Modern automatic end-results regression. [Receipt boundaries](../wiki/systems/battlegrounds.md#bounded-live-proof).

## Coverage expansion — active 2026-10-12

- [ ] Entire source-backed random/epic coverage is in scope: Arathi Basin, Battle for Gilneas, Twin Peaks, Eye of the Storm, Temple of Kotmogu, Silvershard Mines, Seething Shore, Deephaul Ravine, Alterac Valley and Isle of Conquest. Deepwind Gorge lacks TDB authored placements; record this source gap, never fabricate a layout.
- [ ] Per-BG server behavior tests and private real-UDP Modern/Forever native queue/invite/entry/objectives/scoreboard/end/leave proof, with inspected PNGs under `/syncthing/AgentShared/2026-10-12/bg-coverage/`.

Arathi/Gilneas native selection, objective HUD, map-specific counters and type 42 render/interaction are implemented. Arathi (16 accounts) and Gilneas (6 accounts) each have private UDP 5712 Modern/Forever lifecycle proof with 14 inspected PNGs per map. Twin Peaks (10 accounts) reuses the flag HUD with its own PvpStat columns and has the same both-skin proof (three physical captures, Victory, return; 16 PNGs). Eye of the Storm shows four tower-control cells and a Netherstorm flag cell from sourced WorldStates plus a single Flag Captures column; the queue list scrolls natively (wheel/steppers) to reach it. Its both-skin UI/server channel tests pass. Landed M2 floor support unblocks both source starts: private Modern/Forever16-account runs prove four tower presence captures, three physical flag captures, natural1500/183=3, automaticVictory, exact native origin and all15peer return/clear, followed by16returnACKs and periodic save (`eye-live-proof.json`,20 inspected PNGs). Earlier falling failure remains recorded in `eye-start-floor-finding.json`; player-body art, preparation/resurrection and rewards remain uncertified. Scoreboard objective columns now render each map's sourced PvpStat titles in PvpScoreboardLayout order (WSG shows Flag Returns before Flag Captures). Both maps have real-channel server proof; three focused native/UI tests cover both skins. The older WSG receipts below are not evidence for another map. See [server source and promotion contract](../../../game-server/docs/specs/battleground-lifecycle.md#coverage-expansion--active-2026-10-12).

## Kotmogu continuation — active 2026-10-12

Queue 699/map 998 uses the same native Retail specific queue/invitation/result controls in both skins. Its four Orb of Power cells display authoritative source WorldStates as Alliance/Horde/At base; score limit is 1500. Scoreboard uses DB2 column order: Orb Possessions 418, Victory Points 419. Source type 24 flagstand orb M2s render and offer normal object use. Server channel/protocol rules are proved separately. Private UDP 5712 Modern/Forever proof uses four real accounts: source orb use, natural center ticks, 1500 victory, measured 418=1/419=1500, automatic results after closing the scoreboard and origin return. Eighteen inspected 1280×720 PNGs and `kotmogu-live-proof.json` record boundaries. Fixture baseline level/alive/source-origin setup precedes matching; in-match admin only accelerates authored travel. Canonical source publication, orb combat aura/kill credit and scenery/art parity are not certified; source NPC appearance/asset diagnostics and visible checker geometry remain explicit gaps.

## Deephaul Ravine continuation — active

Source1110/map2656 uses two real214690/217346 carts and neutral Crystal422413. Dynamic dropped Crystal422412/type37 must render through normal source-display model resolution and offer object use; no base-flag substitute. HUD shows East/West source cart-control cells and one Crystal cell (Inactive/Unclaimed/Alliance/Horde), not two team flags. Source minimum10-player roster, physical crystal/cart control, map-specific1020 Flag Captures/1021 Carts Controlled,1500victory, automatic results and source origin return require server and both-skin native proof. Real-channel source entry/cart motion/drop/type37 teammate pickup/expiry/capture/paired replacement/natural victory/all-player return passes. Private5712 Modern/Forever each now prove10accounts, visible authenticated5770750/603231, physical crystal capture and grounded actual-cart control/motion/replacement, natural1500–418, measured1020=1/1021=2, automaticVictory, exact native origin/nine-peer clear and10returnACKs then periodic save (`deephaul-live-proof.json`,24 inspected PNGs). GroundMap proves1,346route samples plus source starts/flag/triggers; canonical crystal receipts/player-bundle publication remains lead-owned. Native death/drop/type37 pickup and full scenery/portrait art are not certified by these runs. Source timing/control/path authority belongs in the server contract; vehicle/preparation/visual-aura/reward parity remains uncertified.

## Seething Shore continuation — active

Queue894/map1803 retains the source5–10perteam roster, levels40–90 and1500resource victory. Dynamic type42 Azerite objects281307/272471/281306 use normal source-display rendering and object use, not static invented placements. Source Lock2825 grants261857 Gathering (6s normal cancellable cast); completed captures credit100resources/stat744 Azerite Collected. Native HUD shows scores without unrelated team-flag cells; scoreboard uses744 and automatic authoritative end-results in both skins. Partial grounded channel proof covers real objects, wrong-copy refusal, movement cancellation, duplicate-use refusal, natural15capture victory and all-player exact origin return; it does not prove ship entry. Source starts6337/6380 are transport-relative35/36: server preserves TransportSpawnId and refuses unresolved transport entry; public894 selection stays hidden while active-match HUD/results remain supported. Authored transport spawns/transforms/attachment/collision remain inside this BG goal after Deephaul/epics. Native/private both-skin full lifecycle, parachute, preparation/broadcast/reward and visual-kit parity remain pending.

## Seething authored transport entry — active

Ships 35/36 render source WMO displays 9002/9001 (307812/307818) and their authored deck collision in each copy. Source route progress drives ship transforms between replication updates; late receipts must not rewind the local route clock. Attached native feet/facing retain the ship frame while idle or walking. Movement reports carry ship-local coordinates until actual deck departure, then report ordinary world coordinates even for an otherwise idle drop. Do not predict or send unattached movement while the required ship frame is still loading. Both-skin live entry/drop/objective/return acceptance remains pending; WMO parsing, route/carry math and server collision proofs alone do not certify that flow.

## Isle of Conquest continuation — active

Retail queue 30/map 628 displays authoritative reinforcements against source starting 400, without unrelated team-flag cells. Source banner entries use normal replicated object rendering and Opening interaction. Both skins retain the same queue identity, HUD, ordered Bases Assaulted 245/Bases Defended 246 scoreboard columns and automatic results. The shared AV/IoC both-skin fixture proves these UI contracts. Private Modern/Forever native runs (`modern-isle1`, `forever-isle2`) prove 20 real accounts, visible Quarry banner Opening, one-minute capture, natural 400→401 reinforcement tick, stat 245 and both scoreboard columns, exact native origin return and all 20 return ACKs/saves. Receipts are `*-isle-proof.json` in the coverage diagnostics. This does not prove victory, terrain/player appearance parity, destructible gates, siege vehicles or hangar transport.

## Alterac Valley continuation — active

Retail queue1/map30 admits10–40/team, levels20–90, world-space starts611/610 and700 reinforcements. Source button banners(type1) and Snowfall's neutral goober(type10) render as their replicated GameObjectDisplayInfo models and support normal object use. The native HUD displays remaining reinforcements against source starting700, without unrelated team flags. Scoreboard column order is63 Graveyards Assaulted,65 Graveyards Defended,61 Towers Assaulted,64 Towers Defended,82 Secondary Objectives; authoritative end updates open automatic results in both skins. Specific-list scrolling must reach the appended source AV row without hiding708/1110 or exposing894. Private `native_battleground_peers` may submit `attack:<source-NPC-entry>:<A|B> <nonce>`: only accepted in-progress peers of that authoritative team select an actually replicated living NPC and send normal SetTarget/AttackSwing. `attack-stop <nonce>` stops normal swings. Passive NPC HP DTOs observe real combat; they do not inject damage, health or outcomes. Parser tests do not certify attack delivery before real private live proof.

Current source pure/server proof does not certify native rendering/live lifecycle, preparation, resurrection, map icons, marshal/defender/quest/broadcast/aura/reward parity. See [epic source contract](../../../shared-protocol/docs/specs/epic-battlegrounds.md) and [server runtime boundary](../../../game-server/docs/specs/battleground-lifecycle.md#alterac-valley--active-implementation).

## Silvershard Mines continuation

Queue708/map727 uses the source3–10 per-team roster and source1500 victory. Cart objectives are real NPC60140, not static GO substitutes. Native Specific list admits this NPC-objective map; HUD shows South/North/East control from source WorldStates and scoreboard column428 Carts Controlled. Track Switch60283 follows source124491→normal124492(3s)→97372; the authoritative matched copy alone changes the source branch state. Both-skin private six-account rendered lifecycle proof is recorded in [Silvershard evidence](../wiki/systems/battlegrounds.md#silvershard-private-prerequisites--active2026-10-12). Queues708/1110 are visible in the new engine; their rows and required assets/cache ship together in the next client bundle. Old published clients lack these rows. No server admission gate is required; retain894's authored-transport gate. Lead-owned scoped world promotion and ordered orb/summon import commands are recorded in the server contract; no full content-policy rebuild.

## Known gaps (current cycle)

- [ ] Integration review remains lead-owned. Scoped UI/object tests, native builds, helper cargo check and changed-Rust formatting pass; full suites are periodic master gates, not per-agent branch runs. No master merge performed.
- [ ] Exact PvP-specific textures/icons/chrome parity is not certified; current native panels reuse existing skin-aware frame/button art.

## Out of scope

Numeric preparation/invitation/match/drop/reset/exit timers and honor rewards lack source policy; never fabricate countdowns or grants. Current server combat counters are not instrumented; native columns do not claim measured zero. Rated modes, durable reconnect/origin recovery and Retail dropped-flag visual parity are not implemented by this UI scope. See the server contract for those boundaries.
