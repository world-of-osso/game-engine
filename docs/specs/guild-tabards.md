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
- Current RSX macro truncates bare float literals (`ui-toolkit-macros/src/lib.rs:364–371`). Fractional authored attributes use the supported expression form until the lead-owned macro fix lands; no source values are rounded.
- `godot/network/src/replica/codec.rs`: authoritative design component decoding.
- Shared `guild_tabards::GuildTabardCatalog`: installed palette and torso-layer resolution.

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs`: no-design and registered Communities appearance.
- `godot/tests/guild_no_design_portrait.gd`: native both-skin no-design texture/color/rectangle capture.

## Known gaps (current cycle)

- [ ] Designed portrait/native pixels.
- [ ] Native TabardFrame vendor lifecycle and draft/save reply flow.
- [ ] Equipped and character-selection appearance proof.

## Out of scope

- Personal tabards (Retail item210469); assigned feature is guild registration.
- Other Communities service backing and pet actors/cameras remain separately owned.
