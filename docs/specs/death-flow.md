# Player death flow

Native player death UI consumes the realm's owner-only death snapshots. Both Modern and Forever skins retain Retail mechanics. [Implementation and source audit](../wiki/systems/death-flow.md).

## What it must do

- [x] Default transport receives `DeathStateUpdate`; account dispatch preserves snapshot/error.
- [x] Dead snapshot shows `DEATH`; Release Spirit sends `ReleaseSpirit` on `DeathChannel`, once per answer.
- [x] Ghost snapshot enables per-mesh transparency, world-only saturation 0.2, and corpse minimap/world-map markers on the corpse's map. Alive restores saturation 1.0 and removes markers.
- [x] Ghost proximity to the corpse shows `RECOVER_CORPSE`; Accept sends `ResurrectAtCorpse`. Leaving range/map hides it.
- [x] Spirit healer confirmation Accept sends `AcceptSpiritHealerResurrection`; cancellation, range exit and retries retain authoritative state.
- [x] Live right-click on a spirit healer opens the confirmation; accepting resurrects the player and displays the server-supplied Resurrection Sickness aura in BuffFrame's debuff row.
- [x] Alive clears death popups, ghost appearance and corpse marker. Logout stops the account transport and releases the world/owner state; the saved token remains.
- [x] Popup text, labels and click actions work in both skins.
- [x] `RESURRECT` offers show the caster and server-supplied timeout; Accept/Decline/timeout send the corresponding response, once. Alive closes the offer. Both skins retain the same mechanics.
- [x] Stable replicated tapper identities exempt the local player and current group. Tap-denied nameplate health is Retail 0.9 grey; requested target health is 0.5 grey, through each skin's existing brightness treatment. Untapped/group-eligible units keep their ordinary health colours.

## Remote player life state

- Visible remote players receive explicit Alive, Dead (unreleased corpse), or Ghost; health alone never identifies player life state.
- Dead remote players hold their authored death animation instead of locomotion. Ghosts resume locomotion with the same per-mesh transparency as the local ghost; Alive restores opaque rendering. Replacement visuals retain the current presentation.
- Server visibility follows existing shared `ghost_visible`: living observers do not receive ghosts; ghost observers see ghosts. Map/distance restrictions and self-visibility remain.
- Target/focus/target-of-target and nameplate status text replace numeric health with `Dead` for Dead or Ghost, including ghosts with positive health. This follows Retail's `DEAD` label for both (`CompactUnitFrame_UpdateStatusText`); it does not invent a separate TargetFrame Ghost label. Both skins retain their existing label layouts. Remote player status plates remain eligible through Dead/Ghost under the existing CVars, range and selectability rules; dead NPC plates remain hidden.
- Owner popups, corpse markers and world grading remain owner-only.

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

## Tests asserting this spec

- `godot/network/src/wire_tests.rs`: real loopback UDP death update and requests.
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
| 07 Dead logout/relog | FAIL | `reproof07-logout-ready.png` and checks: Login, world detached, transport disconnected, death snapshot cleared. `reproof07-relogin-ready.png`, checks and death-status: InWorld with HP 0, but authoritative server state Alive and null corpse/graveyard; ghost state does not persist. |
| Local corpse animation | PASS | `reproof-corpse-held.png`, checks and death-status: authoritative Dead/HP 0; actual local `M2Animation` clip 1 remains held across successive captures, then returns to clip 0 on ghost release. |

No new client defect or code fix in this re-proof. The client projects the relog snapshot it receives; synthesizing ghost state from HP would conceal lost server corpse data. No merge or push.

## Known gaps (current cycle)

- [ ] Step 07 end-to-end persistence remains blocked by pinned server `f25258b` returning Alive with HP 0 and no corpse after ghost logout/relog. Logout teardown itself passes. Server changes are outside this client acceptance run's authority.
- [ ] Full multi-skin/reference visual parity is not established by this bounded rendered run.

## Out of scope

New worktrees, merge/push, live development server mutation, soulstone/self-resurrection/recap and release/recovery timers absent from the supplied protocol. Resurrection offers and tap membership are now backed by explicit server/protocol data, never inferred from threat or health.
