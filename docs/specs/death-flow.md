# Player death flow

Native player death UI consumes the realm's owner-only death snapshots. Both Modern and Forever skins retain Retail mechanics. [Implementation and source audit](../wiki/systems/death-flow.md).

## What it must do

- [x] Default transport receives `DeathStateUpdate`; account dispatch preserves snapshot/error.
- [x] Dead snapshot shows `DEATH`; Release Spirit sends `ReleaseSpirit` on `DeathChannel`, once per answer.
- [x] Ghost snapshot enables per-mesh transparency, world-only saturation 0.2, and corpse minimap/world-map markers on the corpse's map. Alive restores saturation 1.0 and removes markers.
- [x] Ghost proximity to the corpse shows `RECOVER_CORPSE`; Accept sends `ResurrectAtCorpse`. Leaving range/map hides it.
- [x] Spirit healer confirmation Accept sends `AcceptSpiritHealerResurrection`; cancellation, range exit and retries retain authoritative state.
- [x] Live right-click on a spirit healer opens the confirmation; accepting resurrects the player and displays the server-supplied Resurrection Sickness aura in BuffFrame's debuff row.
- [x] Alive clears death popups, ghost appearance and corpse marker. [Logout](logout.md) stops the character transport and releases the world/owner state; the account token remains and automatically reloads character select.
- [x] Popup text, labels and click actions work in both skins.
- [x] With DEATH visible, Escape leaves the popup open and opens the Game Menu. DEATH is not an Escape cancellation target; another escapable popup retains its normal cancellation route.
- [x] `RESURRECT` offers show the caster and server-supplied timeout; Accept/Decline/timeout send the corresponding response, once. Alive closes the offer. Both skins retain the same mechanics.
- [x] Stable replicated tapper identities exempt the local player and current group. Tap-denied nameplate health is Retail 0.9 grey; requested target health is 0.5 grey, through each skin's existing brightness treatment. Untapped/group-eligible units keep their ordinary health colours.

Retail grounding: `Blizzard_StaticPopup_Game/Mainline/GameDialogDefs.lua:320-327` sets DEATH `whileDead`, `notClosableByLogout` and `hideOnEscape=false`. `Blizzard_StaticPopup/StaticPopup.lua:814-831` skips non-Escape popups and returns whether anything closed; `Blizzard_GameMenuEsc/Blizzard_GameMenuEsc.lua:100-113` then opens the Game Menu when no handler consumes Escape.

DEATH Escape native proof (2026-10-09): `4ac383a7d` RED renders DEATH but times out opening GameMenuUI. `8a44570de` GREEN retains rendered DEATH and opens the authored menu through physical Escape after a real UDP Dead snapshot; Resume and all existing logout phases pass. Logs: `/home/osso/.worktrees/logs/logoutfix2-death-{red-runtime2,green-runtime}.log`. Final [logout/death/popup regressions](logout.md#proof) pass.

## Remote player life state

- [x] Visible remote players receive explicit Alive, Dead (unreleased corpse), or Ghost; health alone never identifies player life state. Server visibility follows shared `ghost_visible`: living observers do not receive ghosts; ghost observers see ghosts. Map/distance restrictions and self-visibility remain.
- [x] Dead remote player animation holds its authored corpse pose instead of locomotion; Ghost and Alive resume locomotion. Ghost transparency mapping matches the owner's 0.45 per-mesh value; Alive maps to opaque.
- [x] Unit-frame mappings and nameplate status text use `Dead` for Dead or Ghost, including positive-health ghosts. Both-skin target labels are tested. Focus/target-of-target inherit the same state. Retail `CompactUnitFrame_UpdateStatusText` uses `DEAD` for both; no separate TargetFrame Ghost label is invented.
- [x] Remote player status plates remain eligible through Dead/Ghost under existing CVars, range and selectability rules; dead NPC plates remain hidden.
- [ ] Native per-mesh projection and asynchronous replacement visuals retain current corpse/ghost presentation: implemented, but two-player/GPU integration proof belongs to the lead gate.

Owner popups, corpse markers and world grading remain owner-only.

Targeted proof: network `replica::tests::ghoststate_replica_decodes_remote_life_transitions_over_udp`; native `ghoststate` mapping, animation-policy and rendered UI-registry tests. Live remote visual parity remains untested in this work.

## How it works

- [Death flow and protocol gaps](../wiki/systems/death-flow.md)

## Implementation inventory

- `godot/network/src/lib.rs`: default death subscription.
- `godot/rust/src/account.rs`: death event and matching requests.
- `godot/rust/src/death_flow.rs`: owner state, proximity, native visual projection.
- `godot/rust/src/party_frames.rs`: existing StaticPopup input/render loop.
- `godot/ui-model/src/death_flow.rs`: popup lifecycle and action decisions.
- `godot/rust/src/minimap.rs`, `godot/ui-model/src/minimap.rs`: corpse marker and edge arrow.
- `godot/network/src/replica/codec.rs`: remote life-state decoder.
- `godot/rust/src/{replicated,world,targeting,nameplates}.rs`: explicit life-state mapping, corpse animation and status text/eligibility.

## Tests asserting this spec

- `godot/network/examples/native_input_fixture/logout.rs` + `godot/tests/world_logout_flow.gd`: authoritative Dead snapshot → rendered DEATH → physical Escape → DEATH remains and authored Game Menu opens; Resume then Alive snapshot retains existing logout phases.
- `godot/network/src/wire_tests.rs`: real loopback UDP death update and requests.
- `godot/network/src/replica/tests.rs`: explicit remote state transitions over real UDP.
- `godot/rust/src/{animation/remote_player_tests,unit_frame_dead_tests,nameplates}.rs`: authored corpse hold, ghost appearance mapping, both-skin target labels and plate eligibility.
- `godot/ui-model/src/death_flow.rs`: concrete snapshot/proximity/click decisions in both skins.
- `godot/rust/src/account.rs`: native event dispatch.
- `godot/rust/src/account_deathstate_tests.rs`: UDP snapshot → AccountEvent → rendered popup click → production Account request → server receipt, in both skins.
- `godot/network/src/deathstate_fixture.rs`: feature-gated loopback server for that host integration test, including resurrection responses.
- `godot/rust/src/account_deathstate_tests.rs`: resurrection offer → rendered popup → Account accept → real UDP server receipt in both skins.
- `godot/ui-model/tests/rezrtap.rs`, `godot/rust/src/{nameplates,replicated}.rs`: concrete target fills and shared nameplate/tap rules.

## Live acceptance — 2026-10-07

Engine `50af22766`, Godot `4.7.2-pr123946-pr123546`, private server `f25258b`, UDP 5318, level-20 disposable character. Rendered under private Weston `dl38` with Dozen; WSLg untouched. Evidence paths below are relative to canonical `data/diagnostics/deathloop-20261007/`; `proof-ledger.txt` records setup, revisions, readiness boundaries and cleanup. Steps 01, 03, 04 and 06 retain prior accepted evidence and were not rerun.

| Step | Result | Inspected captures and observable proof |
|---|---|---|
| 01 Death/release popup | PASS (retained) | `01-dead-ready.png`: death popup and Release Spirit. |
| 02 Ghost grading/map pin | PASS | `reproof02-ghost-map.png`, `reproof02-ghost-map-checks.json`, `reproof02-death-status.txt`: authoritative Ghost, saturation 0.2, transparent player meshes and rendered Corpse world-map pin. |
| 03 Corpse range | PASS (retained) | `03-outside-32yd.png`, `03-inside-28yd.png`: recovery popup absent outside 30 yd, present inside. |
| 04 Corpse resurrection | PASS (retained) | `04-corpse-alive.png`, matching state: alive after recovery; prior ledger records approximately 50% health/mana with regeneration. |
| 05 Spirit healer/sickness | PASS | `reproof05-confirm.png`: physical healer right-click and plain-text warning. `reproof05-alive-ready.png`, matching state and death-status: Alive, ghost effects cleared, spell 15007 / texture 136147 visible in `DebuffButton0`. Initial capture preceded asynchronous icon readiness. |
| 06 Player resurrection offer | PASS (retained) | `06-offer-accept.png`, `06-declined.png`, `06-accepted-alive.png`: prior accepted offer/decline/accept proof; timeout not re-proven here. |
| 07 Dead logout/relog | FAIL (2026-10-07); persistence PASS (2026-10-09), requested logout route FAIL | Historical `f25258b`: Login teardown passed, relog returned Alive/HP 0 without corpse. Current server `70ae62a67` including `c2903d3` preserves Ghost/position/corpse and unreleased Dead; bounded re-proof below records remaining client logout failures. |
| Local corpse animation | PASS | `reproof-corpse-held.png`, checks and death-status: authoritative Dead/HP 0; actual local `M2Animation` clip 1 remains held across successive captures, then returns to clip 0 on ghost release. |

No new client defect or code fix in this re-proof. The client projects the relog snapshot it receives; synthesizing ghost state from HP would conceal lost server corpse data. No merge or push.

## Step 07 live re-proof — 2026-10-09

Engine `f55c65a20b613023be971d74525468ab7367375d`; server `70ae62a671385f5c091745cf7fd954607bb9599a` (contains persistence fix `c2903d3`). Extension, IPC CLI, server and admin built through the shared build lock, each exit 0. Private UDP 5460, fresh diagnostic `game.redb`, account `fb_deathlive`, level-20 Deathlive. One rendered client at a time under headless cage; no product code changes, push, merge or UDP 5000 mutation.

All eight PNGs were individually inspected at 1920×1080. Captures are in `/syncthing/AgentShared/2026-10-09/death-relog/`; state snapshots, logs, manifest and build results are under canonical `data/diagnostics/deathlive-2026-10-09/`.

| Step | Result | Inspected captures and observable proof |
|---|---|---|
| Die → release spirit | PASS | Admin kill, actual Release Spirit button click. `01-ghost-before-relog.png`: graded ghost world, ghost HUD, minimap corpse icon. `02-ghost-status.txt`: authoritative Ghost, corpse at `(-8949, 82.62083, 132)`. |
| Logout to character list | FAIL | Actual Game Menu → Log Out detaches world/disconnects, but opens Login (`02-logout-login.png`), not CharacterSelect. Cause: `godot/rust/src/logout.rs:71–72` explicitly selects Login. Character list reached on authenticated client startup (`03-character-list.png`), then actual Enter World click. |
| Ghost relog persistence | PASS | `04-ghost-after-relog.png`: ghost appearance/grading, HUD and minimap corpse icon persist. `04-ghost-after-status.txt`: Ghost with identical corpse/graveyard. Before/after account snapshots have identical authoritative `(-8946.23, 79.9953, 183.477)` and local `(-8946.23, 79.90372, 183.477)` positions, HP 0. |
| Run to corpse → resurrect | PASS | Actual scripted forward movement, 1 + 6.3 seconds at heading 183.08°, no teleport. `05-recover-corpse.png`: recovery dialog; actual Accept click. `06-resurrected-alive.png`: colour/HUD restored. Authoritative Alive, null corpse/graveyard; subsequent snapshot HP 2755 at the death position. |
| Unreleased death: normal logout | FAIL | `07-unreleased-before-logout.png`: Dead/Release Spirit. Escape does not open Game Menu; logout button unavailable. Cause: `godot/rust/src/party_frames.rs:289–304` routes Escape to the open death popup and consumes the key before Game Menu. No release sent. |
| Unreleased death: disconnect/relog control | PASS (disconnect only) | Kicked only `fb_deathlive` via private admin socket, then replaced the stopped client and selected Deathlive. `08-unreleased-after-relog.png`: still Dead, Release Spirit popup, HP 0, same position and corpse; no automatic release/resurrection. Not a normal-logout acceptance substitute. |

Server reference: `crates/server/src/auth_character_tests.rs:685–729` cites `Player::LoadFromDB`/`PLAYER_FLAGS_GHOST` and `Player::LoadCorpse` for preserving released Ghost and corpse; `:732–770` expects unreleased Dead and HP 0 without another durability loss. Live state matches both state expectations; durability was not measured live. No external reference audit or server tests were rerun.

Diagnostic automation required two corrections: remove an unavailable snapshot method, then atomic command-file writes after a partial JSON read stopped polling. Ghost relog therefore includes a stopped/replaced client rather than uninterrupted same-process login. All owned server/client/compositor PIDs were stopped, own slice inactive, UDP 5460 free. No changes to other acceptance steps.

## Known gaps (current cycle)

- [ ] Step 07 strict logout workflow remains FAIL: Login instead of character list; unreleased death popup prevents Escape opening Game Menu. Server persistence is live-proven, but normal unreleased logout and uninterrupted same-client ghost relog remain unproven.
- [ ] Full multi-skin/reference visual parity is not established by this bounded rendered run.

## Out of scope

New worktrees, merge/push, live development server mutation, soulstone/self-resurrection/recap and release/recovery timers absent from the supplied protocol. Resurrection offers and tap membership are now backed by explicit server/protocol data, never inferred from threat or health.
