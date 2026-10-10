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

## Scoped acceptance (2026-10-10)

| Capability | Evidence | Limit |
|---|---|---|
| Aura332 action apply/restore | UI-model RED/GREEN; effective ID drives native icon, tooltip, cooldown and cast | Real override aura render/cast pending |
| Spellbook base binding, substituted name/icon/cooldown | Pure projection RED/GREEN plus 12 native-model book layout tests | Runtime override proof pending |
| Ground intent destination/cancel | UI-model3/3; real pinned Infernal Strike/Heroic Leap vs Blink catalog1/1 | Live ground placement pending; reticle is native ring geometry |
| Same-map player/camera | Epoch/orbit unit test; both-skin private master3753ca2 Blink wall cast moved3.3311yd, no errors | Camera collision near Abbey confines the view; no pixel-exact Retail comparison |
| Aura312 | Recorded replacement-set loader/translation gap | Not applied |

Implementation660bd1606 native build passed with no warnings. Changed Rust formatting passed; workspace fmt finds only inherited `network/src/replica/codec.rs` and `ui-model/src/game_tooltip/merchant.rs` differences. Cargo check was cancelled after unexpected detachment: its ktx2-rw build script remained active for over nine minutes; no completed check proof is claimed. No broad suites.

Private override setup reached Fury spec72 but known spells omitted1719/85288. Online admin `learn-spell` reported success; that handler queues the profession learner, and no requested class spells appeared in the client snapshot. Retry ceiling reached. Complete an authentic talent allocation before continuing this real-server fixture; do not fabricate replicated auras or mutate production.

User-visible Blink PNGs and 60-frame MP4 sequences (1280x720, 4fps playback, not real-time recordings) are in `/syncthing/AgentShared/2026-10-10/overrides/`. ffprobe confirmed both60 frames/15s; ffmpeg decoded all frames and signalstats were inspected. Full logs stay under `data/diagnostics/overrides-2026-10-10/`. Private UDP5184 and all owned clients were stopped; UDP5000 untouched.
