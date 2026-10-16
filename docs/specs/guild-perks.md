# Guild perks

Retail Mainline Benefits perks in Modern and Forever. Definitions: installed `db2/active/GuildPerkSpells.csv`; presentation: `Blizzard_Communities/GuildPerks.lua`, `GuildPerks.xml`, `CommunitiesFrame.xml`. Membership grants/removal: pinned TrinityCore a352b1fa `Guild.cpp:2322,3002`. [Guild systems](../wiki/systems/guilds.md).

## What it must do

- [ ] Display the five installed perk definitions in source order with real spell names/icons: Mount Up78633, Hasty Hearth83944, Mobile Banking83958, The Quick and the Dead83950, Guild Mail83951. Missing source data fails explicitly; never fabricate definitions.
- [ ] Benefits uses the source270×44 perk rows,39px icons, source background/list chrome and padding in both skins. Other tabs hide this list.
- [ ] Hover renders the actual spell tooltip; modified chat-link clicks retain the actual spell identity. Ordinary clicks do not cast perks.
- [ ] Actual guild membership grants the installed spells; leaving/removal revokes them. Login and ordinary class/spec rebuilds reconcile membership without dropping unrelated spell knowledge.
- [ ] Distinguish catalog/known-spell authority from implemented spell effects; never claim a functional buff, mail change or guild chest from its displayed name alone.

## How it works

- [Guild systems](../wiki/systems/guilds.md).

## Implementation inventory

| File | Role |
|---|---|
| `godot/ui-model/src/guild_communities.rs` | Benefits composition |
| `godot/ui-model/src/guild_perks.rs` | Actual installed catalog and source perk list |
| `godot/rust/src/guild_ranks.rs` | Explicit catalog readiness before native Benefits sync |

## Tests asserting this spec

- `godot/ui-model/tests/guild_rank_widgets.rs::guild_perks_benefits_lists_the_five_installed_perk_spells_both_skins` — actual Benefits output in both skins.

## Known gaps (current cycle)

- [ ] Installed catalog, source UI/tooltips, membership reconciliation and authentic effect coverage still require implementation/proof.
- [ ] Private actual join/leave/relog and inspected1920×1080 both-skin captures.

## Out of scope

Guild rewards require their separate runtime import and reputation/achievement eligibility; existing62 TDB rows are not a working reward system. Invitation tabards follow this slice. Unimplemented perk effects remain named gaps rather than fabricated functionality.
