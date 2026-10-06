# Player death flow

Native player death UI consumes the realm's owner-only death snapshots. Both Modern and Forever skins retain Retail mechanics. [Implementation and source audit](../wiki/systems/death-flow.md).

## What it must do

- [x] Default transport receives `DeathStateUpdate`; account dispatch preserves snapshot/error.
- [x] Dead snapshot shows `DEATH`; Release Spirit sends `ReleaseSpirit` on `DeathChannel`, once per answer.
- [ ] Ghost snapshot enables ghost visuals and a corpse minimap marker/edge arrow on the corpse's map.
- [x] Ghost proximity to the corpse shows `RECOVER_CORPSE`; Accept sends `ResurrectAtCorpse`. Leaving range/map hides it.
- [x] Spirit healer confirmation Accept sends `AcceptSpiritHealerResurrection`; cancellation, range exit and retries retain authoritative state.
- [ ] Live InteractUnit/right-click on a spirit healer opens the confirmation (implemented; native process proof pending).
- [ ] Alive clears death popups, ghost appearance and corpse marker. Disconnect resets owner state.
- [x] Popup text, labels and click actions work in both skins.
- [x] `RESURRECT` offers show the caster and server-supplied timeout; Accept/Decline/timeout send the corresponding response, once. Alive closes the offer. Both skins retain the same mechanics.
- [x] Stable replicated tapper identities exempt the local player and current group. Tap-denied nameplate health is Retail 0.9 grey; requested target health is 0.5 grey, through each skin's existing brightness treatment. Untapped/group-eligible units keep their ordinary health colours.

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

## Known gaps (current cycle)

- [ ] Corrected corpse-marker fixture rerun queued behind unrelated publishing operation's shared build lock; details and exact proof scopes in the [proof ledger](../wiki/systems/death-flow.md#behavioral-proof-2026-10-06).
- [ ] Ghost transparency/native GPU appearance not runtime-proven; preserve documented host compositor mitigation.

- [ ] Retail spirit-healer text describes 50% durability; pinned server applies 25% to equipped durability and does not attach computed sickness. Client formats the equipped-durability warning for that server; it cannot implement the missing debuff.

## Out of scope

New worktrees, merge/push, live development server mutation, soulstone/self-resurrection/recap and release/recovery timers absent from the supplied protocol. Resurrection offers and tap membership are now backed by explicit server/protocol data, never inferred from threat or health.
