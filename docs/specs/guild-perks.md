# Guild perks

Retail Mainline Benefits perks in Modern and Forever. Definitions: installed `db2/active/GuildPerkSpells.csv`; presentation: `Blizzard_Communities/GuildPerks.lua`, `GuildPerks.xml`, `CommunitiesFrame.xml`. Membership grants/removal: pinned TrinityCore a352b1fa `Guild.cpp:2322,3002`. [Guild systems](../wiki/systems/guilds.md).

## What it must do

- [x] Display the five installed perk definitions in source order with real spell names/icons: Mount Up78633, Hasty Hearth83944, Mobile Banking83958, The Quick and the Dead83950, Guild Mail83951. Missing source data fails explicitly; never fabricate definitions.
- [x] Benefits uses the source270×44 perk rows,39px icons, source background/list chrome and padding in both skins. Other tabs hide this list.
- [x] Hover renders the actual spell tooltip; modified chat-link clicks retain the actual spell identity. Ordinary clicks do not cast perks.
- [x] Actual guild membership grants the installed spells; leaving/removal revokes them. Login and ordinary class/spec rebuilds reconcile membership without dropping unrelated spell knowledge.
- [x] Distinguish catalog/known-spell authority from implemented spell effects; never claim a functional buff, mail change or guild chest from its displayed name alone.

## How it works

- [Guild systems](../wiki/systems/guilds.md).

## Implementation inventory

| File | Role |
|---|---|
| `godot/network/src/lib.rs` | Ordered owner spell snapshots/deltas alongside TalentChannel specialization/config replies |
| `godot/ui-model/src/guild_communities.rs` | Benefits composition |
| `godot/ui-model/src/guild_perks.rs` | Actual installed catalog and source perk list |
| `godot/rust/src/guild_ranks.rs` | Catalog readiness and modified row input |
| `godot/ui-model/src/chat_draft.rs` | Real spell/item/achievement identities behind editable labels |
| `godot/ui-model/src/ui/chat_frame.rs` | Labeled received spell links, source color and tooltip identity |
| `godot/rust/src/chat.rs` | Native source-link insertion at actual selection/caret |
| `godot/rust/src/guild_perks_preview.rs` | Read-only actual owner knowledge/chat receipt |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs::guild_perks_benefits_lists_the_five_installed_perk_spells_both_skins` — actual Benefits output in both skins.
- `chat_draft::tests::guild_perks_real_spell_link_draft_hides_markup_and_preserves_existing_subjects` — real spell identity and visible text coexist with item/achievement links.
- `godot/tests/guild_perks_live.gd` — real owner spell receipts, source Benefits rows, physical tooltip/modified-link input and fresh-process observation.

- `godot/network/src/wire_guild_perks_tests.rs` — real UDP5891 held-worker login snapshot, grant, quit revocation, class snapshot and rejoin grant retain channel order.

- `godot/network/examples/native_guild_perks_membership.rs` — private5891 actual login/invite acceptance, forced removal, rejoin and quit; verifies received knowledge and exact class conservation.

## Known gaps (current cycle)

- [x] Source Benefits names/order/tab visibility pass against the actual installed five-definition catalog in both skins.
- [x] Real native spell tooltips, physical Shift-click draft/server echo and fresh-process owner knowledge pass both skins.
- [x] Physical ordinary clicks on all five rows send no spells in either skin. Actual modified insertion at an existing caret survives the next model sync and server echo.
- [x] Bounded catalog/membership/UI and caret follow-up landed; individual effect gates remain separate.
- [x] Supported Mount Up172/211 and map suppression landed; both-skin actual movement/removal proof and ten inspected mounted originals pass. Controlled-NPC flight210 remains explicitly blocked.
- [x] Hasty Hearth is independently qualified below; [effect matrix](../wiki/systems/guilds.md#perk-native-authority-and-effect-boundary-2026-10-16).
- [ ] Mobile Banking83958 remains source-blocked: script77 has no authored trigger;88304/88306 summon actual206602/206603, but no pinned dispatch binding or Friendly guild-reputation eligibility authority is established. The description's88306 duration reference is not authority to invent either.
- [ ] Quick/Dead83950 remains source-blocked: aura340/+50% has TC HandleNULL and a guild-member-resurrector enum name, unlike its unqualified installed tooltip; dummy3/+10% dead movement has no pinned script. Health formula/recipient/guild and dead-speed mechanics remain unclaimed.
- [x] Actual UDP invitation/forced removal/rejoin/quit, fresh-process native owner/tooltip readback and inspected1920×1080 both-skin captures.

## Out of scope

Guild rewards require their separate runtime import and reputation/achievement eligibility; existing62 TDB rows are not a working reward system. Invitation tabards follow this slice. Unimplemented perk effects remain named gaps rather than fabricated functionality.

## Hasty Hearth native effect proof

- [x] Physical backpack use of real6948 completes8690 and reports charge2309. Actual Human458619 remains known: two charges,25-minute nonmember and10-minute guild recovery (isolated base without racial is30/15 minutes).
- [x] Actual invitation acceptance/removal does not retime an already-consumed Hearthstone charge. Modern/Forever observe the same owner authority.
- [x] Fresh server/client scenarios establish both baseline and member consumption without injecting or shortening charge clocks. `guild_hasty_hearth_live.gd` reads actual charge timers and cast-success receipts. Each scenario physically consumes one of the Human's two charges in each skin. Twelve inspected full-HD originals show backpack/item/cast context; numeric recovery comes from actual owner receipts, not a rendered timer label. Pending-clock persistence across restart is not claimed.

## Mount Up native effect proof

`guild_mount_up_live.gd` observes the local player's actual replicated MovementSpeed/Mounted/PlayerMotion. One rendered ordinary member and a real UDP officer perform invitation acceptance and forced removal, while real Golden Gryphon casts and physical forward/takeoff input exercise mounted ground/flight speed in both skins. Admin fixture setup teaches only the authored riding/mount spells and selects steady flight; no speed or perk/aura is injected. Expected source baseline→guild→removed is14→15.4→14 run and22.4→24.64→22.4 flight. Authored210's flying-vehicle context remains a separately named decision/proof gap, never folded into the normal player multiplier.
