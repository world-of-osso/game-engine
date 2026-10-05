# Portrait Party Frames

Static native `PartyMemberFrame` family under Modern and Forever. `PortraitPartyFrameState` supplies four non-self member slots; no roster adapter, runtime heads, click/menu wiring or Edit Mode setting. Compact party remains the default. [Contract](../../specs/group-frames.md).

## Content

`godot/ui-model/src/ui/screens/portrait_party_frame_component.rs` reuses unit-frame `PortraitSlot`, `portrait_slot`, `status_bar`, atlas lookup and labels. Bars reveal the supplied fractions; name tint uses supplied class colour, health retains Retail's locked art colour. Host receives slot metadata for later portrait bindings. Optional pets require `show_pets`, a supplied pet view and an online member. A guide replaces, rather than accompanies, the leader crown. No level or assistant region exists in this Retail template; role icons do.

### Retail source geometry

Paths below are relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/` (local source inspected 2026-10-05). There is no `Mainline/PartyMemberFrame.xml`; the template is in `PartyFrameTemplates.xml`.

- `Blizzard_UnitFrame/Mainline/PartyFrameTemplates.xml:79-103`: member 120×53; portrait 37×37 TOPLEFT (7,−6), CircleMask; portrait-on art TOPLEFT (1,−2). `:118-123`: name 57×12 TOPLEFT (46,−6).
- `PartyFrameTemplates.xml:138-156,242-246`: health 70×10 TOPLEFT (45,−19); mana 74×7 TOPLEFT (41,−30). `PartyMemberFrame.lua:44-62` reapplies these normal-player slots. Template health mask `:223-239` covers prediction/background textures, not the base health fill. Mana mask `:265-272` has no MaskedTexture binding. Do not invent base-fill masking.
- `PartyFrameTemplates.xml:302-329`: crown/guide BOTTOM relative TOP (−10,−6); role 12×12 TOPRIGHT (−5,−5). `PartyMemberFrame.lua:349-364,382-398`: leader-only crown or LFG guide; tank/healer/damage role. No assistant or level region.
- `PartyMemberFrame.lua:187`: health zero text `DEAD`. Offline view uses Retail `PLAYER_OFFLINE` wording (“Offline”), supplied explicitly by the host. This static component does not infer online state from world entities.
- `PartyFrameTemplates.xml:9-32,46-56,366-370`: pet 64×23, 18×18 CircleMask portrait (3,−3), half-scale party art, half-scale 71×10 health at (43,−18); parent anchor (23,−43). `PartyMemberFrame.lua:316-322,633-636`: connected + pet exists + showPartyPets; pet name hidden, no power bar.
- `Blizzard_UnitFrame/Shared/PartyFrame.lua:1,20,55-68,142-150`: four member templates; TOPLEFT layout slots; gaps 10 without pets, 26 with pets. `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:290-295`: Modern container TOPLEFT on manager TOPRIGHT (0,−7). Static screen reuses the existing HUD party anchor, not a new placement setting.

### Forever art mapping — corrected claim

Forever paths are relative to `~/.cache/wow-ui-sim/blizzard-ui/wowforever/AddOns/`.

`Blizzard_UnitFrame/Mainline/PartyMemberFrame.lua:13,71-118` explicitly adds the conditional `CharacterFrameOn` art branch. Portrait mask becomes the player portrait mask; frame/health use `CharacterFrameOnParty` atlases; mana moves to (46,−30), width 69. Component deliberately selects this approved branch for Forever; the cached standard template does **not** prove automatic activation. Power fills select the corresponding named CharacterFrameOnParty regions (not hard-coded crops); Modern selects Party regions.

The conditional CharacterFrameOnParty DB2 relation in `data/db2/1.60.1.69913/` is:

| Source | Relation |
|---|---|
| `UiTextureAtlasElement.csv:17785` | element 33561 = `ui-hud-unitframe-characterframeonparty-portraiton` |
| `UiTextureAtlasMember.csv:17334` | set-1 member 38477, `...portraiton-c60`, element 33561 → atlas 3960; crop (1,457)–(121,506), 120×49 |
| `UiTextureAtlas.csv:2562` | atlas 3960 → FDID 8036204, set 1, 256×512 |

This is the **same sheet as Forever's player portrait-on** (`scripts/forever-atlas-listfile.csv:7`, `interface/hud/uiunitframec60.blp`), verified by a relation test rather than pinned FDIDs/UVs. The local raw-BGRA BLP is 256×512; its 120×49 crop was inspected and saved as `data/diagnostics/party1-2026-10-05/camelot-party-art.png`.

**`uipartyframec60` also exists, but is a different branch.** `scripts/forever-atlas-listfile.csv:49` maps `interface/hud/uipartyframec60.blp` to 8116745. Versioned `UiTextureAtlas.csv:2616` maps it to set-1 atlas 4019 (128×128); `UiTextureAtlasMember.csv:17817` maps member 39017 to element 21081 (`UI-HUD-UnitFrame-Party-PortraitOn`), crop (1,53)–(121,102), 120×49. `:17818` maps member 39018 to the vehicle variant. Thus ordinary Party art (and half-scale pet art) resolves to uipartyframec60 under Forever; the requested conditional CharacterFrameOnParty frame resolves to uiunitframec60. Do not conflate them.

The stale root CSVs/community listfile miss these set-1 relations; the project supplement and versioned DB2 directories the atlas loader actually reads establish them. No new asset mapping or fallback added.

### Proof and capture

`godot/ui-model/tests/portrait_party_frame.rs` tests four names, supplied bar fractions, Offline/Dead and leader visibility under both skins, and Forever's relationship to Camelot/player art. `capture_modern_portrait_party_fixture` generates the Modern semantic registry golden; normal golden test compares the resulting external registry tree. It does not prove raster parity or live wiring.

Standalone offline preview: `GODOT_CAPTURE_SCREEN=portrait_party GODOT_CAPTURE_PATH=<png>` with `res://tests/capture_ui_screen.gd`. `RegistryUi.show_portrait_party` installs deterministic four-member data without connecting a server. Portrait slots are intentionally empty until runtime binding work.

## Sources

- [group-frames spec](../../specs/group-frames.md) — task boundary and retained default
- Local Retail/Forever XML/Lua and versioned DB2 citations above
- `godot/ui-model/src/ui/screens/portrait_party_frame_component.rs`, `godot/ui-model/tests/portrait_party_frame.rs` — component and behavioral capture

## See Also

- [[group-frames]] — live compact roster/raid implementation
- [[ui-system]] — authored registry screens and portrait machinery
