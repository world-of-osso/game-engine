# Player death flow

Native player death UI consumes the realm's owner-only death snapshots. Both Modern and Forever skins retain Retail mechanics. [Implementation and source audit](../wiki/systems/death-flow.md).

## What it must do

- [ ] Default transport receives `DeathStateUpdate`; account dispatch preserves snapshot/error.
- [ ] Dead snapshot shows `DEATH`; Release Spirit sends `ReleaseSpirit` on `DeathChannel`, once per answer.
- [ ] Ghost snapshot enables ghost visuals and a corpse minimap marker/edge arrow on the corpse's map.
- [ ] Ghost proximity to the corpse shows `RECOVER_CORPSE`; Accept sends `ResurrectAtCorpse`. Leaving range/map hides it.
- [ ] Interacting with a spirit healer near the graveyard opens confirmation; Accept sends `AcceptSpiritHealerResurrection`. Server refusal is visible and permits retry.
- [ ] Alive clears death popups, ghost appearance and corpse marker. Disconnect resets owner state.
- [ ] Popup text, labels and click actions work in both skins.

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
- `godot/network/src/deathstate_fixture.rs`: feature-gated loopback server for that host integration test.

## Known gaps (current cycle)

- [ ] `RESURRECT` from another player is blocked: pinned protocol has no offer, caster name, offer identifier/expiry, or accept/decline request. Shared pure `cast_resurrect` is not a wire message or a server offer handler.
- [ ] Tap-denied greying is blocked: server `CreatureTap.tappers` is server-local; replicated `UnitFlags` has no viewer-relative tap-denied flag; shared schema has no tap list/owner component.
- [ ] Retail spirit-healer text describes 50% durability; pinned server applies 25% to equipped durability and does not attach computed sickness. Client formats the equipped-durability warning for that server; it cannot implement the missing debuff.

## Out of scope

Server/protocol edits, new worktrees, merge/push, live development server mutation, soulstone/self-resurrection/recap and release/recovery timers absent from the supplied protocol. No fabricated resurrection offer or inferred tap ownership.
