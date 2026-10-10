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
| Aura332 action apply/restore | UI-model RED/GREEN; effective ID drives native icon, tooltip, cooldown and cast | Both skins: real aura1719 apply/restore, effective335097 intent; no-target rejection expected |
| Spellbook base binding, substituted name/icon/cooldown | Pure projection RED/GREEN plus 12 native-model book layout tests | Both skins: base-bound book entry/tooltip swaps and restores |
| Ground intent destination/cancel | UI-model3/3; real pinned Infernal Strike/Heroic Leap vs Blink catalog1/1 | Both skins: pending reticle then clicked destination moves14.269yd; reticle is native ring geometry |
| Same-map player/camera | Epoch/orbit unit test; both-skin private master3753ca2 Blink wall cast moved3.3311yd, no errors | Camera collision near Abbey confines the view; no pixel-exact Retail comparison |
| Aura312 | Recorded replacement-set loader/translation gap | Not applied |

Implementation660bd1606 native build passed with no warnings. Changed Rust formatting passed; workspace fmt finds only inherited `network/src/replica/codec.rs` and `ui-model/src/game_tooltip/merchant.rs` differences. Initial cargo check was cancelled after a cold ktx2-rw build script attempted a new network download. Matching same-host cached KTX4.4.0 headers/static library were seeded into that check output (no download/source change); final helper cargo check passed at9455e03ba with no warnings. Cold-build automation itself was not changed. No broad suites.

Initial server3753ca2 override setup omitted1719/85288 despite admin learning success. Server masterca6b249 fixed live learned-spell publication. A fresh redb on private5184 then supplied these spells to Fury72 through admin learning. Native book drag exposed a separate release-only Button routing bug; approved fix9455e03ba routes spellbook sources through existing press-coordinate FrameClick, keeping hover. Both-skin final fixture7ac442591 passed plain click (one85288 intent), hover, book→main-bar drag, aura1719 replacement335097, expiry restoration and ground6544 placement. Replacement melee intents deliberately had no target and received the expected rejection; successful melee damage is not claimed. Ambient NPC spell-visual attachment errors10848/80676 remain unrelated.

User-visible override/ground PNGs and Blink PNGs/60-frame MP4 sequences (1280x720, 4fps playback, not real-time recordings) are in `/syncthing/AgentShared/2026-10-10/overrides/`. ffprobe confirmed both60 frames/15s; ffmpeg decoded all frames and signalstats were inspected. Full logs stay under `data/diagnostics/overrides-2026-10-10/`. Private UDP5184 and all owned clients were stopped; UDP5000 untouched.


### Approved draggable-button routing audit

Bag slots, equipment slots, merchant/trade item cells and non-backpack bag slots are clickable frame widgets and already receive press coordinates. Bank slot buttons were already specially routed. Spellbook buttons were the sole existing draggable Button source missing that route; only their prefix was added. Native mount/toy/pet journal drag entries were not found. `ActionRef::Macro(u32)` exists on the wire, but no native macro drag UI was found. Dragging from an action bar remains an existing explicitly unbuilt case, not fixed by this press-source change.
