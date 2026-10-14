# Guild tabards

Guild tabard appearance and registration follow Retail Mainline `Blizzard_UIPanels_Game/Mainline/TabardFrame.xml/.lua`, `GuildUtil.lua`, and TrinityCore a352b1fa `Guild::HandleSetEmblem`. [Guilds](../wiki/systems/guilds.md) owns membership and charter state.

## What it must do

- [ ] Communities uses the guild snapshot's registered five-ID design, the installed palette and source large emblem UVs/56×64 width. Unregistered guilds keep the exact no-design branch.
- [ ] GuildTabardVendor opens source338×424 TabardFrame with NPC portrait/name, actual player model, five32px left/right variation controls, authored texture plates, cost/personal money and80×22 Accept/Cancel anchors. Both skins implement the same controls and layout.
- [ ] Variation changes are local drafts. Accept sends one authoritative vendor registration and disables repeat writes while pending; failure reports the server reason without changing guild appearance or spending local money. Cancel/Escape closes the matching interaction.
- [ ] Only a guild leader can accept. Others can browse, with the source non-GM greeting and disabled Accept.
- [ ] Designed guild tabards use installed GuildTabard background/emblem/border torso layers on the actual model. Source customizable items5976,69209,69210 use tiers0,1,2 (WoWModelViewer WoWItem.cpp863–868 and TabardDetails.cpp209–217); unrelated tabards remain unchanged.
- [ ] Entering world, guild design changes and membership removal refresh replicated appearance. Character selection renders the persisted design too.

## How it works

- [Guild lifecycle](../wiki/systems/guilds.md).
- [Communities no-design contract](gameplay-panel-fidelity.md#communities-guild-portrait).

## Implementation inventory

- `godot/ui-model/src/guild_communities.rs`: Communities portrait and shell.
- `godot/ui-model/src/guild_tabards.rs`: palette loading, five-variation drafts and pending authority.
- `godot/ui-model/src/guild_tabard_frame.rs`: exact TabardFrame XML art/anchors and source button postsetup.
- Fractional authored attributes are preserved. Shared macro fix `645c0b8`, landed on macros master, removes literal truncation.
- `godot/network/src/replica/codec.rs`: authoritative design component decoding.
- `godot/rust/src/guild_tabards.rs`: vendor lifecycle, pending requests, player draft viewport and held-button rotation.
- `godot/rust/src/account.rs`, network bridge: typed GuildChannel requests and authoritative reply dispatch.
- `godot/rust/src/game/equipment/equipment_appearance_data.rs`: source guild-tabard tiers and replacement torso layers before clothing-priority compositing.
- `godot/rust/src/world*.rs`, `assets/player.rs`, `character_select.rs`: replicated/roster designs enter model loading and invalidate stale visuals.
- Shared `guild_tabards::GuildTabardCatalog`: installed palette and torso-layer resolution.

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs`: no-design and registered Communities appearance.
- `godot/tests/guild_no_design_portrait.gd`: native both-skin no-design texture/color/rectangle capture.
- `godot/ui-model/tests/guild_tabards.rs`: source controls, draft isolation, leader gating and pending replies.
- Native equipment `guild_tabard_body_layers` fixture: all three source tiers and unchanged ordinary tabards.
- `godot/tests/guild_tabard_fidelity.gd`: physical controls, pending/failure/save/close boundaries, held rotation, production detached player models for all three tiers, masked Jondor portrait and designed Communities; replies simulated, not live network proof.
- `godot/tests/guild_tabard_character_select.gd`: both-skin production roster projection and `CharacterPreview` campsite/model capture with persisted design IDs; offline fixture, not authenticated networking.
- Shared wire `guild_tabard_character_selection_payload_retains_registered_design` and server `guild_tabard_roster_exposes_durable_design_without_changing_character_appearance`: character-selection design serialization and persisted roster projection.

## Known gaps (current cycle)

- [ ] Designed portrait/native pixels.
- [ ] Native TabardFrame vendor lifecycle and draft/save reply flow.
- [ ] Equipped and character-selection appearance proof.

## Out of scope

- Personal tabards (Retail item210469); assigned feature is guild registration.
- Other Communities service backing and pet actors/cameras remain separately owned.
