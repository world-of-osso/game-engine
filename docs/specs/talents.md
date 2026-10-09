# Read-only Talents

The PlayerSpellsFrame Talents page displays the local Retail class/spec talent tree in Modern and Forever skins. Core loading lives in `godot/core/src/talent_data.rs`; [data/layout provenance](../wiki/systems/talents.md) describes the projection.

## What it must do

- [x] Map character class to its category-7 SkillLine, then SkillLineXTraitTree (variant 0), not to a hardcoded tree ID.
- [x] Filter nodes/entries by TraitCond Visible conditions and SpecSetMember. Preserve sufficient (OR) hero-spec conditions.
- [x] Display class nodes left, specialization nodes right, eligible hero subtrees in the middle; retain ordered choice entries and DB2 edges.
- [x] Translate PosX/PosY using Blizzard's /10 scale and pan offsets. Hero nodes use normalized top/center positions and 0.85 scale.
- [x] Show TraitDefinition spell icons, or OverrideIcon; missing art stays missing, never a substitute icon.
- [x] Only matching Granted conditions show learned ranks. Zero purchase cost alone is not a grant; no starter-loadout allocations are applied.
- [x] Hover uses shared spell GameTooltip name/description; choice entries each expose their spell tooltip. No node click purchases, casts or mutates a character.
- [x] Mage Arcane (62) selects tree 658, class 43/spec 38 nodes; three cited nodes and choice 62087 match CSV. Eligible Sunfury/Spellslinger have 14 nodes each.
- [x] Native cage captures show both skins at actual 1920×1080.

## How it works

- [Local DB2 mapping, Blizzard layout and witnesses](../wiki/systems/talents.md).
- [Shared PlayerSpellsFrame geometry](spellbook-action-bar.md).

## Implementation inventory

- `godot/core/src/talent_data.rs`: local DB2 graph projection, no engine/network dependency.
- `godot/core/tests/talent_data.rs`: concrete Mage Arcane CSV contract.
- `godot/ui-model/src/talents.rs`: entry icon metadata and explicit missing-art projection.
- `godot/ui-model/src/ui/screens/spellbook_frame_component/talents_page.rs`: read-only nodes, edges, ranks and stacked eligible hero previews.
- `godot/ui-model/src/spellbook_preview.rs`: offline local Mage snapshot (Arcane for Talents).
- `godot/rust/src/spells/spellbook.rs`: cached class/spec loading in the live PlayerSpellsFrame.
- `godot/rust/src/tooltip_sources.rs`: shared live spell hover path.
- `godot/rust/src/ui/spellbook_preview.rs`: offline native hover through the shared spell GameTooltip.
- `godot/rust/src/ui/icon_masks.rs`: passive talent circle clipping using the existing authored mask.
- `godot/rust/src/ui/parts.rs`: missing-source no-paint and solid-color rotation/tint preservation.
- `godot/ui-model/src/ui/screens/spellbook_frame_component.rs`: shared PlayerSpellsFrame state/page dispatch.

## Tests asserting this spec

- `godot/core/tests/talent_data.rs`: mapping/counts, three nodes/edges, choice entries, grants, hero gating.
- `godot/ui-model/tests/talents.rs`: both-skin node/icon/edge/choice display and no purchase/cast actions.
- `godot/tests/talents_page.gd`: real 1920×1080 native cage geometry, spell name/description hover, missing-icon/diagonal-edge pixels.
- `godot/rust/src/ui/parts_tests.rs`: native missing-source and solid edge projection.

## Known gaps (current cycle)

- [x] Native nofake/edge pixel acceptance: corrected primitive GREEN16/16, zero warnings; native rebuild/cage exit0. Both skins/hover captures inspected at actual1920×1080. Core5/5 and UI14/14 proof scopes unchanged; exact ledger in [provenance](../wiki/systems/talents.md).
- [ ] Cage capture and hover proof pending.
- [ ] 103 local icon FDIDs remain unavailable, logged with reasons to `/home/osso/.worktrees/logs/extract-wanted.tsv`; absent art stays empty. Availability is sampled at graph load; restart the client after extraction.
- [ ] Wowdev retrieval returns HTTP 403; local schemas and Blizzard enums are directly inspected instead. URLs are recorded as requested references, not as successfully read sources.

## Out of scope — not yet: learning, loadouts, server

- Learning/spending, refunds, rank editing, activation of hero choices: later slice.
- Loadout application/import/export, saved allocations: later slice. TraitTreeLoadout rows are provenance only, never learned-state input.
- Server persistence, validation and protocol changes: later slice; this page is explicitly read-only.
