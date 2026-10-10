# Spell overrides and teleport targeting

verified: 2026-10-10

Native aura332 presentation derives an effective spell from the local player's current `UnitAuras`. Stored action slots and known spells remain base IDs; action activation, icon, cooldown and tooltip read the effective ID. Spellbook learned entries substitute presentation each rebuild while retaining base binding IDs; their cooldown shade queries the effective spell timer. Removal therefore restores base state without rebinding.

## Retail references

Local Retail snapshot: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_ActionBar/Mainline/ActionButton.lua`: native action APIs own effective action texture, cooldown and GameTooltip:SetAction. Lua does not expose the C++ aura substitution implementation.
- `Blizzard_APIDocumentationGenerated/SpellDocumentation.lua`: `C_Spell.GetOverrideSpell`.
- `Blizzard_APIDocumentationGenerated/SpellBookDocumentation.lua`: `FindSpellOverrideByID` and spellbook item info.
- `Blizzard_PlayerSpells/SpellBook/Blizzard_SpellBookItem.lua`: `UpdateSpellData` reads native item info; `UpdateVisuals` displays its name/icon; `UpdateCooldown` (:376) reads `GetSpellBookItemCooldown`. The API's override information is distinct from stored known spells.
- SpellActivationOverlay handles proc glow events, not action binding ownership; no new aura-based glow is inferred.

## Ground targeting and same-map teleports

The catalog reads `SpellEffect.ImplicitTarget_0/1 = 87` (DEST_DEST) into `ground_targeted`; Blink target55 and target-unit leaps do not enter cursor mode. The pending intent remains local until an unhandled world click hits drawn terrain/WMO/doodad geometry. The hit is transformed back under WorldUnits and sent in `destination`. Right-click/Escape cancel; UI clicks do not place. A green ground ring marks the cursor point (native ring geometry, not pixel-exact Retail reticle art).

MovementControl epoch changes already snap the player. The camera now translates its previous pose by the player displacement on that epoch change, preserving orbit and normal endpoint collision checks instead of interpolating across the entire teleport distance. Cross-map reset still clears the camera anchor.

## Animation gap

`godot/rust/src/animation/mod.rs` selects M2 sequence animation IDs; action layers in `animation/action.rs` select explicit spell/combat clips. Neither loads animation replacement-set records or translates set IDs to animation IDs. Aura312 set1013 is retained on the replica but not applied. Adding that data subsystem is outside this client pass.

## Sources

- [Contract](../../specs/spell-overrides-teleports.md)
- `godot/ui-model/src/spell_overrides.rs`, `godot/rust/src/spells/action_bar.rs`, `casting.rs`, `spellbook.rs`, `tooltip_sources.rs`.

## See Also

- [[player-cast-feedback]] — native spell animation selection.
- [[godot-conversion]] — native client lifecycle and proof boundaries.
