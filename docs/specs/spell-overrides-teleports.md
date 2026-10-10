# Spell overrides and teleport targeting

Native spell presentation consumes server aura overrides without changing learned spells or saved action slots. Source: `godot/ui-model/src/spell_overrides.rs`, `godot/rust/src/spells/`. [Implementation](../wiki/systems/spell-overrides-teleports.md).

## What it must do

- [ ] Active aura332 substitutions show and cast the replacement on action buttons, including its tooltip, icon and cooldown; removal restores the base spell.
- [ ] Learned spellbook entries present the active replacement name, icon, tooltip and cooldown without mutating known spells or base drag/binding IDs.
- [ ] Ground-destination spells enter targeting mode, show a ground reticle, send the clicked destination, and cancel on right-click or Escape. Forward Blink does not require a destination.
- [ ] Same-map teleports snap the player while preserving camera orbit without sweeping the camera through intervening geometry.
- [ ] Apply aura312 animation replacement sets when the native animation system has a replacement-set hook; otherwise record the gap.

## How it works

- [Native implementation and Retail references](../wiki/systems/spell-overrides-teleports.md)

## Implementation inventory

- `godot/ui-model/src/spell_overrides.rs`: derives effective actions from current auras.
- `godot/rust/src/spells/action_bar.rs`: replacement button icon/cooldown and activation.
- `godot/rust/src/spells/casting.rs`: resolves casts to the active replacement.
- `godot/rust/src/spells/spellbook.rs`: replacement names/icons for learned entries.
- `godot/rust/src/tooltip_sources.rs`: effective action tooltip.

- `godot/ui-model/src/spell_targeting.rs`: pending destination intent and cancellation.
- `godot/core/src/game/spell_catalog/build.rs`: authored ground cursor target metadata.
- `godot/rust/src/spells/ground_target.rs`: reticle and world-click placement.
- `godot/core/src/camera_follow_data.rs`, `godot/rust/src/camera.rs`: same-epoch follow vs teleport anchor translation.

## Tests asserting this spec

- `godot/ui-model/tests/spell_overrides.rs`: replacement/restoration and item/animation isolation.

- `godot/ui-model/src/spell_targeting.rs` tests: destination, cancellation and nonfinite input.
- `godot/core/tests/spell_catalog.rs`: Infernal Strike/Heroic Leap vs Blink/target leaps.
- `godot/core/src/camera_follow_data.rs`: orbit preserved at changed epoch.

## Known gaps (current cycle)

- [ ] Native animation selects M2 sequence IDs and action layers; no animation replacement-set loader/mapping hook exists. `AuraOverride::Animation(1013)` cannot be interpreted as an M2 animation ID.
- [ ] Private-server proof pending; the ground ring is native geometry, not exact Retail reticle art.

## Out of scope

- Server teleport validation, cross-map transfer lifecycle and new animation replacement-set infrastructure.
