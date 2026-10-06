# Party Edit Mode settings

Party settings live in Options → HUD → Layout Settings → Party, in both skins. The existing Edit Mode layout file persists them per character; changing any setting saves the active layout immediately, rather than waiting for Options Apply. Compact remains the project default, at 98×44. No positioning controls or frame movers are added.

## Retail inventory

Sources below are relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`. `D` = `Blizzard_EditMode/Shared/EditModeSettingDisplayInfo.lua`; `P` = `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`. The Party `settings` table at P:274–288 is the exhaustive list of settings this system has, not the entire UnitFrame enum. Display values use D:8–21: width/height store offsets from minimum; percentage sliders store step indices.

| Setting | Source | Retail Modern default (stored → displayed) | Display range / step | Our control |
|---|---|---|---|---|
| Use Raid-Style Party Frames | D:222–227; P:276 | 0 → false; **ours true** | boolean | Checkbox |
| Show Party Frame Background | D:229–234; P:277 | false | boolean | Checkbox |
| Use Horizontal Layout (`UseHorizontalGroups`) | D:330–335; P:278 | false | boolean | Checkbox |
| Display Border | D:337–342; P:279 | false | boolean | Checkbox |
| Frame Height | D:288–300; P:280 | 8 → 44 | 36–72 / 2 | Slider |
| Frame Width | D:274–286; P:281 | 26 → 98 | 72–144 / 2 | Slider |
| Frame Size | D:355–366; P:282 | 0 → 100% | 100–200% / 5 | Slider |
| Sort By | D:317–328; P:283 | Group | Role, Group, Alphabetical | Dropdown |
| Aura Organization | D:368–380; P:284 | Legacy | Legacy, Buffs Top / Debuffs Bottom, Buffs Right / Debuffs Left | Dropdown |
| Opacity | D:383–394; P:285 | 100 → 100% (clamped) | 50–100% / 1 | Slider |
| Debuff Icon Size | D:424–436; P:286 | 5 → 100% | 50–200% / 10 | Slider |
| Big Defensive Icon Size | D:396–408; P:287 | 5 → 75% | 50–100% / 5 | Slider |
| Buff Icon Size | D:410–422; P:288 | 5 → 100% | 50–200% / 10 | Slider |
| Show Pets (requested addition) | `Blizzard_UnitFrame/Shared/PartyFrame.lua`:4,25,138–146; `Mainline/PartyMemberFrame.lua`:317 | CVar `showPartyPets`, **not in Edit Mode's Party settings table**; ours false preserves current look | boolean | Checkbox |

Retail hides compact-only controls in portrait mode and hides background in compact mode (`EditModeSystemTemplates.lua`:1297–1335). The explicit task instead requires dimensions/orientation/background/border/sort/pets to affect either active family. Our controls stay exposed and those shared settings apply to both families. Portrait dimensions scale its authored 120×53 shape relative to the unchanged compact 98×44 preset. Aura organization/icon sizing affect compact frames only, as Retail specifies.

## Wiring and data boundaries

`LayoutSettings.party` holds optional displayed-value settings; absent fields retain the old appearance. `use_raid_style_party_frames` retains its existing field. Width/height/icon/opacity values clamp at application; UI slider input snaps to the source step. Reset clears settings through the existing layout action. Built-in presets are not overwritten: existing layout persistence creates a saved layout on first change.

The compact and portrait components read generation-tracked LayoutSettings. Portrait bindings use the same sort as rendered labels; group hit handling receives the sorted view too. Sorting follows `Blizzard_CompactRaidFrames/Blizzard_CompactRaidFrameContainer.lua`:429–493: Group keeps player first then roster order; Role uses Tank, Healer, Damage, None with alphabetical ties; Alphabetical compares names. No raid settings change.

Replicated unit auras supply visible buff icons, and `UnitSummonedBy` plus `Health` supply pets for known party members. Compact pet lookup uses an independent member-name map, including self, not the non-self portrait roster. Compact pets are health-only. The Display Border control emits a parsed CSS-style stroke above member art; native pixel-difference proof checks that toggling it actually paints edges. Group-only data outside replication interest still contains only harmful auras, not buffs or pets: those extras are absent, not fabricated. Existing group health/power remains authoritative.

**Big defensive runtime classification is blocked:** Retail `AuraUtil.IsBigDefensive` reads classification not carried by `AuraView` or the existing spell catalog. The slider is persisted and the compact renderer honors a supplied defensive icon, but production does not designate arbitrary buffs as defensives. No protocol/server changes are authorized in this slot. Do not claim complete live coverage for that setting.

## Bounded verification — 2026-10-06

At implementation `6df0d0d3`, 1 native + 10 UI-model targeted tests pass; 11 prior portrait regressions remain valid for unchanged default art/selector behavior. One fixture generator is intentionally ignored. Native build and changed-file Rust formatting pass. Each skin passes six real-engine cases, including an actual border pixel-difference regression, sorted head rebinding and zero remaining owned portrait viewports. All eight final panel/portrait captures were inspected. [Full commands, test names, image verdicts and proof ledger](../../../data/diagnostics/party4-2026-10-05/proof.md).

The early capture fixture omitted wheel forwarding; subsequent panel inspection found truncated icon labels. Both were corrected. Native pixel inspection additionally exposed a border attribute that produced no stroke; the final valid CSS-style stroke paints visible edges. Compact self pets originally depended on the non-self portrait roster and reserved a power-bar region; separate RED tests reproduced both before correction.

No full step-4 completion claim: live big-defensive classification remains blocked above. No live/reference-image or clean global shutdown acceptance. Final Modern reports 51 texture RIDs / 43 ObjectDB instances; Forever 53 / 44, with additional renderer/font allocation warnings. All owned client/compositor PIDs exited; no existing WSLg/service was restarted. These warnings are not classified as harmless or pre-existing.

## Sources

- Local Retail files and exact line ranges in the inventory above.
- [Group frames spec](../../specs/group-frames.md).
- [Portrait family and runtime proof](portrait-party-frames.md).

## See Also

- [[portrait-party-frames]] — authored frames, portrait resource lifetime and prior acceptance boundaries.
