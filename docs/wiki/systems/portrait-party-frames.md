# Portrait Party Frames

Native `PartyMemberFrame` family under Modern and Forever. `PortraitPartyFrameState` supplies four non-self member slots from the live group roster, with runtime heads for known appearances. [Party settings](party-edit-mode-settings.md) are exposed in both skins. Portrait click/menu wiring and full live acceptance remain excluded. Compact party remains the default. [Contract](../../specs/group-frames.md).

## Step 2 — roster and source corrections

`godot/rust/src/party_frames.rs::group_frames_state` supplies both compact and portrait states. The portrait adapter excludes the local player, preserves server roster order, and reads only online members' `GroupState.live` for health/power/type and Dead/Ghost. Names/class/leader/role/online come from the roster; unavailable live bars stay empty. Offline rendering overrides those fractions as Retail does. Raid returns no portrait members.

`LayoutSettings.use_raid_style_party_frames` is persisted with existing per-character layouts: absent/true keeps compact; false selects portraits. `group_frames_screen` reads the canvas's settings and toggles both roots. The existing per-frame `set_state`, generation-tracked Screen sync and layout `sync_skin` propagate roster/settings changes. No settings UI added (step 4).

Source paths below are relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/`:

- `PartyMemberFrame.lua:570-585`: offline health takes maximum value and `SetStatusBarDesaturated(true)`; portrait desaturates, pet hides; reconnect removes desaturation. Member component supplies full offline health and postsetup marks the sampled health texture, not its white vertex tint. Native `ui/parts.rs` forwards this existing texture flag to the existing canvas desaturation shader; `ui/projection.rs` preserves it while adding disabled-scrollbar desaturation. The first native recapture at `27b41020` caught projection overwriting it with false on non-scrollbars despite green registry/parts tests. `capture_ui_screen.gd::offline_party_health_is_desaturated` reproduced both skins RED at actual rendered pixels before correcting that overwrite. Portrait slots remain empty in step 2; postsetup applies the flag when a portrait texture is installed, but does not create runtime heads.
- `UnitFrame.lua:930-950`: offline power takes maximum value and `SetStatusBarColor(0.5,0.5,0.5)` unless locked. `:535-541` resets colour and desaturates only dead **player** power. Party offline power therefore uses full width and half-grey vertex tint, not invented power desaturation; atlas hue may remain dark blue.
- `PartyFrameTemplates.xml:293-311`: overlay fills its 120×53 parent; leader/guide BOTTOM relative TOP at (-10,-6), using atlas size. In down-positive coordinates, leader rect is `(120/2-10-w/2, 6-h, w, h)`, or `(42,-10,16,16)` for both current skins. Step 1 already had that rect. Above-frame crown extension is source-authored, not an anchor defect; keep it, make derivation explicit, and test against resolved atlas dimensions. `PartyMemberFrame.lua:349-364` controls crown/guide visibility.

Behavioral coverage: concrete 2→4→2→0 member roster, promotion, offline, death and departure through roster adapter and rendered registry; both-skin default/portrait/raid switching; offline bar flags/tint and crown geometry; native image projection flag. At `27b41020`, two targeted Godot tests and all 12 portrait UI tests pass (one fixture-regeneration test ignored). At `a220f937`, native build and both-skin raster regression pass; changed-file format checks pass. Earlier unit proof remains valid for unchanged adapter/component code; raster proof covers the projection follow-up.

Inspected captures: `data/diagnostics/party2-2026-10-05/{modern,forever}-party-recapture.png` and `*-detail.png`. Both show full grey offline health, full dim-blue power and the source-authored crown rect. Samples at (130,294): Modern (178,178,178), Forever (175,175,175); power (130,306) is (6,62,122) under both skins. `raster-red/` retains the reproduced green-health failure. Both clients/Weston exit 0 and all owned PIDs are gone. Pre-existing shutdown leaks persist (Modern 7 texture RIDs/10 ObjectDB instances; Forever 9/11), so this is bounded static raster proof, not clean shutdown or step-5 live acceptance. Full proof/commands are in the same directory's `proof.md`; no runtime heads, settings UI, pet acceptance or merge/push.

## Step 4 — presentation settings

[Party Edit Mode settings](party-edit-mode-settings.md) records the exhaustive local Retail inventory, defaults/ranges, shared-family task overrides, persistence and data boundaries. Compact remains default. Settings implementation and bounded proof do not imply step-5 live acceptance.

## Step 3 — runtime member portraits

`party_frames.rs::update_group_frames` synchronizes the roster canvas before `GameClient::sync_party_portraits`. `unit_portraits/party.rs` selects four non-self names in roster order only for the portrait style and non-raid groups. Each name owns the existing `Portrait` renderer, detached model request and last replicated `UnitAppearance`. Reorder moves surviving views between hosts; rebuilt hosts recreate their views. Departures, compact/raid selection and scene exit cancel pending loads and free views with their child viewports/models. No party renders exist under the compact default.

Retail source paths relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/`:

- `PartyMemberFrame.lua:570-585`: `Portrait:SetDesaturated(true)` when disconnected, false on reconnect. Native mask shader desaturates the rendered head, not only its host's registry texture.
- `PartyMemberFrame.lua:592-620`: `PartyMemberHealthCheck` tints dead `(0.35,0.35,0.35,1)`, ghost `(0.2,0.2,0.75,1)`, living health `(0,20%]` red, otherwise white. Offline ignores stale live health/death, consistent with the group adapter dropping offline live states.
- `UnitFrame.lua:175-189,199-200`: portraits use `SetPortraitTexture`, refreshed on `UNIT_PORTRAIT_UPDATE`. Neither that handler nor the party portrait health/connection handlers introduce range-driven tint/desaturation/fade. Retain a member's last known head outside replication interest; do not apply CompactUnitFrame's 0.5 range alpha to this portrait family.

Appearance availability is a real protocol boundary: `GroupMemberSnapshot` supplies name/class/level, not race/customization/equipment; live group states supply health/power/death/position. An unseen member cannot have a truthful 3D head until replicated. No invented model or other member's head fills that gap. Known offline/out-of-interest heads remain available while portrait style stays selected. Settings UI, pet portrait wiring and live acceptance remain excluded.

Proof runner: `res://tests/party_portraits.gd` instantiates the offline `PartyPortraitFixture`, using the production selector/resource owner and real detached player-model loader. It exercises default/switch-to-compact zero hosts, member join/leave/reorder, actual grey offline head pixels, dead tint and four joins/four leaves. Separate Godot processes isolate each skin; CPU selector tests do not switch global skins.

At native `5f33198a` plus fixture `978772c5`, both skin processes pass all five tests and exit 0. Offline head sample: 0 colored / 127 visible grey pixels. Both `data/diagnostics/party3-2026-10-05/{modern,forever}-party-recapture.png` and enlarged `*-party-detail.png` pass inspected bounded runtime-head raster; not live or full reference parity. Five targeted native Rust tests and six UI-model tests pass; native build and Cargo formatting pass. Owned portrait SubViewports are zero after four joins/four leaves, compact switch and fixture shutdown. All owned PIDs exit. Global shutdown warnings remain (Modern 16 texture RIDs/16 ObjectDB instances, Forever 18/17), higher than the static step-2 baseline; clean global shutdown is not proved. Diagnostics retain rejected test assumptions: typed array conversion, automatic node renaming during reparent, and colored skin-ring pixels outside the grey head. Full proof and revision ledger: `data/diagnostics/party3-2026-10-05/proof.md`.

## Authored camera and frontal key — 2026-10-06

`unit_portraits.rs::frame_portrait` already uses the loaded M2 root's source metadata and camera **type 0**, not camera array index zero. Human female HD's portrait camera is record 1. Eye/target use the visual root transform; camera clipping and existing FoV sampling remain unchanged. Player body selection follows local build 12.1.0.69933's ChrRaceXChrModel → ChrModel.DisplayID → CreatureDisplayInfo.ModelID → CreatureModelData.FileDataID chain. All 118 linked body-display rows have zero portrait-display/portrait-texture overrides. Inventory of 76 unique body models finds type-0 cameras for every race listed in character_model_data; unrelated rows 14/15 lack female cameras and row 99 lacks cached models. No guessed race/sex camera or forced frontal yaw was added.

Portraits previously reused the sheet's fixed `(1,0.6,0.3)` key rays, although `m2.gdshader` lights with **negative** sun_direction. The face lacked its direct key, exaggerating the dark profile/hair silhouette. `2cb76764` binds common preview fill plus rays **toward** the target (`-camera.global_basis.z`) after framing, including transformed creature roots. Character-sheet lighting stays unchanged. The offline fixture also incorrectly made all four named members human; Theron/Valeera now use Blood Elf HD, Jaina/Uther Human HD. These remain default player appearances, not unique named NPC models.

Eight composed authored Stand head/camera tests pass: Human male/female 37.01°/35.48°, Blood Elf 27.65°/36.99°, Dwarf 33.24°/35.47°, Orc 25.65°/41.87°. Limit 55° allows authored three-quarter framing with 13° margin beyond the largest sample, while excluding a 90° profile. No camera-selection/side-on-angle error reproduced; lighting correction preserves authored composition. Native raster compares each of four frozen heads under frontal and reversed keys: frontal luminance is 1.98–2.07× reversed. Both skins' five existing roster/render/lifetime tests pass. Inspected heads are lit and recognizable; Human female retains the smaller head framing from the existing empty-first-track FoV default (1 radian). This is not exact Retail screenshot parity, live acceptance or proof for every race's pixels.

Proof and captures: `data/diagnostics/portraitcam-2026-10-06/proof.md`, `{modern,forever}-party-recapture.png`, `{modern,forever}-party-detail.png`. Scoped Rust/core tests, native builds and changed-file formatting pass; later HUD-only fixture commits preserve portrait proof. Global shutdown RID/ObjectDB warnings remain; all owned processes exited.

### Modern chat / player drawn overlap

The same offline native investigation renders PlayerFrame and ChatFrame1 separately against an empty baseline. The original literal 1920×1080 canvas reproduces bounds intersection `(428,760,72,70)` and **596 shared drawn pixels**, bounding envelope `(447,782,53,33)`. This is visible art overlap, not merely invisible Control bounds. Native default UIParent scaling at physical 1920×1080 instead gives logical 1365.33×768, scale 1.40625: physical bounds intersection 326.25×98.4375, with 11,243 shared drawn pixels. Threshold: any RGB channel differs from the same empty render by >1/255.

Approved anchors are unchanged. Evidence: `portraitcam-2026-10-06/modern-hud-reported-overlap-crop.png` (literal reported canvas), `modern-hud-overlap-crop.png` (native default), corresponding `*-pixels.json`, isolated layers and annotated masks. Sources: [portrait renderer](../../../godot/rust/src/unit_portraits.rs), [model selection](../../../godot/core/src/player_model_data.rs), [camera parser](../../../godot/core/src/asset/m2_format/m2_camera.rs), [native overlap fixture](../../../godot/tests/modern_hud_overlap.gd), [scaling](../../../godot/rust/src/ui/ui_parent.rs).

## Acceptance status — bounded offline art/fill pass (2026-10-05)

Local CASC supplied Retail `4681512.blp` and Forever build 70205's `4631591.blp` (MD5 `fa74b4e688a6d03d856ab616238058fe`; Retail remains `73bb980b6a3b738fa8f07877419a1d5e`). The slot has a separate data directory; its two missing inputs are linked to canonical data without replacing Retail bytes. Raw BGRA decoding confirms 1024×512 and bar art at 69913's player-health, player-mana and conditional party bar coordinates. Evidence: `data/diagnostics/party1-2026-10-05/forever-70205-69913-bar-crops.png`, individual `70205-*-crop.png` files and source rows in `forever-70205-69913-crops.json`.

Forever's explicit party crops bind `data/forever-1.60.1.70205/textures/4631591.blp` through existing RSX `texture_file`, `TextureSource::File` and native `ui::assets::load_source`/`FileTextures`. File paths already key both decoded and projection caches. Modern keeps named Retail atlases and `data/textures/<fdid>.blp`; C60 set-1 FDIDs still use the existing importer/listfile and shared loader. No global FDID replacement or parallel loader: that would break unrelated Retail set-0 coordinates still used by the shared resolver. Forever mana now crops the 69913 player fill (815,326)–(939,336), rather than the resolver's Retail coordinates. Required files fail explicitly when absent; no alternate art.

Product binding and mana source regressions reproduced RED before implementation. At `dea89b5a`, targeted `portrait_party_frame` tests pass 9/9 (one regeneration test ignored), shared `status_text_bars` regressions pass 4/4, and the local native extension build exits 0. All use the build lock and `agent-run`. Logs and revision scope: `data/diagnostics/party1-2026-10-05/product-binding-proof.md`.

Owned Dozen/Weston recaptures `modern-party-recapture.png` and `forever-party-recapture.png` both exit 0 and pass inspected four-member art/fills: health 75/50/25/0%, mana 50/25/100/0%, crown, Offline/Dead and the correct skin frame. `*-party-recapture-detail.png` provides enlarged inspection; `recapture-fill-samples.json` records sampled colour runs (source-edge transparency changes counts, not reveal endpoints). All owned client/compositor/child PIDs exited. The initial capture setup failed on AF_UNIX path length, then on a diagnostics symlink resolving the repo root; the retained recapture script uses a short slot runtime and explicit repo slot.

Both native runs still emit texture RID/ObjectDB shutdown leaks (Modern 7 texture RIDs/10 instances, Forever 9/11). This is bounded offline member art/fill acceptance, not clean-resource acceptance, reference-image parity, pet raster acceptance or live/runtime portrait proof. Portrait slots intentionally remain empty. Compact remains default and steps 2–5 remain excluded.

### Previous acceptance blockers (2026-10-05)

At `1b875412`, step 1 was **not raster-accepted and must not be wired as complete**. Twelve targeted behavioral/registry tests pass (8 portrait + 4 shared status-bar regressions); the Modern code-generated registry golden passes, and extension build `91960f9f` succeeds. These prove state/geometry/source-record relationships, not matching physical texture contents.

Owned offline Dozen/Weston captures in `data/diagnostics/party1-2026-10-05/` exposed two asset blockers:

- `modern-party.png` / `modern-preview.log`: required Retail party sheet `data/textures/4681512.blp` is absent; art and fills are not drawn.
- `forever-party.png` / `health-source-pixels.png`: C60 frame art is correct, but conditional health/non-mana bar crops from 69913 address unrelated icons/text in the cached Retail `4631591.blp`. Same FDID, different layout: player health is (705,213)–(829,233) in Retail 69933, versus (693,238)–(817,258) in Forever 69913. Source-row tests alone did not catch the physical-sheet mismatch.

At that point, no matching 69913 base sheet was found in the examined local asset caches; only Retail CASC resolution cache was available. Required next input: matching Forever base-sheet bytes and a skin/version-specific asset binding (do not replace shared Retail FDID 4631591), plus the missing Retail party sheet. No alternate-art substitution or CDN extraction performed. Both captures exited 0 but emitted RID/ObjectDB shutdown leak warnings; neither is visual parity or clean-resource acceptance. Owned render processes were stopped.

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

`Blizzard_UnitFrame/Mainline/PartyMemberFrame.lua:13,71-118` explicitly adds the conditional `CharacterFrameOn` art branch. Portrait mask becomes the player portrait mask; frame/health use `CharacterFrameOnParty` atlases; mana moves to (46,−30), width 69. Component deliberately selects this approved branch for Forever; the cached standard template does **not** prove automatic activation. Mana uses the explicit player fill from `:106`; Modern selects Party fills. Spec powers reuse the existing unit-frame art (`Mainline/UnitFrame.lua:539-541`, `info.atlas`).

**Resolver boundary:** `ui-toolkit/core/src/atlas/db2.rs:77-80` imports Retail set 0 and Forever set 1 only. Conditional CharacterFrameOnParty health and non-mana basic fills exist solely in Forever set 0, so their names do not resolve. Component draws those exact source crops through the existing `AtlasArt` representation and shared status-bar body; it never tries alternate art. `UiTextureAtlasMember.csv:16607-16614` supplies the health/Energy/Focus/Rage/RunicPower crops on atlas 2060, FDID 4631591 (1024×512). Behavioral tests join the versioned source CSVs and compare actual drawn sheet/UVs, not fixed expected pins. This bounded workaround applies to the frozen 69913 geometry; retire the explicit crops when the shared resolver supports these Forever base names and their product-specific sheet bytes. No sibling toolkit changes are part of this task.

The conditional CharacterFrameOnParty DB2 relation in `data/db2/1.60.1.69913/` is:

| Source | Relation |
|---|---|
| `UiTextureAtlasElement.csv:17785` | element 33561 = `ui-hud-unitframe-characterframeonparty-portraiton` |
| `UiTextureAtlasMember.csv:17334` | set-1 member 38477, `...portraiton-c60`, element 33561 → atlas 3960; crop (1,457)–(121,506), 120×49 |
| `UiTextureAtlas.csv:2562` | atlas 3960 → FDID 8036204, set 1, 256×512 |

This is the **same sheet as Forever's player portrait-on** (`scripts/forever-atlas-listfile.csv:7`, `interface/hud/uiunitframec60.blp`), verified by a relation test rather than pinned FDIDs/UVs. The local raw-BGRA BLP is 256×512; its 120×49 crop was inspected and saved as `data/diagnostics/party1-2026-10-05/camelot-party-art.png`.

**`uipartyframec60` also exists, but is a different branch.** `scripts/forever-atlas-listfile.csv:49` maps `interface/hud/uipartyframec60.blp` to 8116745. Versioned `UiTextureAtlas.csv:2612` maps it to set-1 atlas 4019 (128×128); `UiTextureAtlasMember.csv:17817` maps member 39017 to element 21081 (`UI-HUD-UnitFrame-Party-PortraitOn`), crop (1,53)–(121,102), 120×49. `:17818` maps member 39018 to the vehicle variant. Thus ordinary Party art (and half-scale pet art) resolves to uipartyframec60 under Forever; the requested conditional CharacterFrameOnParty frame resolves to uiunitframec60. Do not conflate them.

The stale root CSVs/community listfile miss these set-1 relations; the project supplement and versioned DB2 directories the atlas loader actually reads establish them. The product-specific base-sheet binding above is limited to the explicit party crops; C60 mapping is unchanged.

### Proof and capture

`godot/ui-model/tests/portrait_party_frame.rs` tests four names, supplied bar fractions, Offline/Dead and leader visibility under both skins, and Forever's relationship to Camelot/player art. `capture_modern_portrait_party_fixture` generates the Modern semantic registry golden; normal golden test compares the resulting external registry tree. It does not prove raster parity or live wiring.

Standalone offline preview: `GODOT_CAPTURE_SCREEN=portrait_party` (Modern) or `forever_portrait_party`, `GODOT_CAPTURE_PATH=<png>`, with `res://tests/capture_ui_screen.gd`. `RegistryUi.show_portrait_party` / `show_forever_portrait_party` install deterministic four-member data without connecting a server or changing saved settings. This older static-only preview intentionally leaves portraits empty. Step-3 runtime-head captures use `res://tests/party_portraits.gd` instead.

## Sources

- [group-frames spec](../../specs/group-frames.md) — task boundary and retained default
- Local Retail/Forever XML/Lua and versioned DB2 citations above
- `godot/ui-model/src/ui/screens/portrait_party_frame_component.rs`, `godot/ui-model/tests/portrait_party_frame.rs` — component and behavioral capture

## See Also

- [[group-frames]] — live compact roster/raid implementation
- [[ui-system]] — authored registry screens and portrait machinery
