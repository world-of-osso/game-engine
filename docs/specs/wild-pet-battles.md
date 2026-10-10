# Wild pet battles

Wild PvE encounters use server-authoritative snapshots and actions, rendered through `godot/ui-model/src/wild_pet_battle.rs` and the native host. Retail Mainline `Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.xml`, `.lua` and `Blizzard_PetBattleUIPatchwerks.xml` define the controls and layout; Forever changes art only.

## What it must do
- [ ] Right-click a wild battle-pet creature to request a battle with the journal's first three ordered combat slots.
- [ ] Journal slots equip distinct owned instances and send an authoritative loadout request.
- [ ] Both pets display health, level, family and auras; ability1–3 display cooldown/availability.
- [ ] Swap, pass, confirmed forfeit and eligible trap actions carry server battle/round identity.
- [ ] Wild PvE uses the untimed Retail timer mode, not an invented timeout.
- [ ] The camera frames both active 3D pet models; swapping updates models.
- [ ] End displays outcome/XP/capture and disables further actions. Journal updates remain authoritative.
- [ ] Modern and Forever retain identical geometry and controls.

## How it works
- [Battle-pet journal and assets](../wiki/systems/battle-pets.md).
- [Server turn catalog](../../../game-server/docs/wiki/systems/pet-battle-engine.md).

## Implementation inventory
- `godot/ui-model/src/wild_pet_battle.rs`: state, action admission and Retail-derived screen.
- `godot/ui-model/src/pet_journal.rs`: three ordered combat-slot controls.
- `godot/rust/src/wild_pet_battle.rs`: inputs, networking, native HUD and two-model camera scene.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs`: actual network receipt and requests.
- `godot/rust/src/merchant.rs`, `gameplay.rs`: wild right-click routing and modal movement.

## Tests asserting this spec
- `godot/ui-model/tests/wild_pet_battle.rs`: both-skin frames/timer/actions, round identity, cooldowns, swaps, forfeit/end.
- `godot/ui-model/tests/pet_journal.rs`: owned loadout action and duplicate-slot clearing.

## Known gaps (current cycle)
- [ ] Native build and both-skin private live full win/capture proof with inspected PNGs.

## Out of scope
PvP matchmaking and turn effects not admitted by the inherited server engine. No unsupported-effect direct-damage substitute is allowed.
