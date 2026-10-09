# Read-only Talents data/layout

Contract: [Talents](../../specs/talents.md). Verified local inputs: 2026-10-08, Retail DB2 `12.1.0.69933`.

## Sources

Requested wowdev references: [TraitTree](https://wowdev.wiki/DB/TraitTree), [TraitNode](https://wowdev.wiki/DB/TraitNode), [TraitNodeEntry](https://wowdev.wiki/DB/TraitNodeEntry), [TraitDefinition](https://wowdev.wiki/DB/TraitDefinition), [TraitEdge](https://wowdev.wiki/DB/TraitEdge), [TraitNodeGroup](https://wowdev.wiki/DB/TraitNodeGroup), [TraitCond](https://wowdev.wiki/DB/TraitCond), [TraitSubTree](https://wowdev.wiki/DB/TraitSubTree), [SkillLineXTraitTree](https://wowdev.wiki/DB/SkillLineXTraitTree), [TraitTreeLoadout](https://wowdev.wiki/DB/TraitTreeLoadout). All returned HTTP 403 on this host. Do not treat their unread text as evidence; saved responses live in the evidence directory below. Join-table schemas are directly read from local CSV headers.

The provided Blizzard_ClassTalentUI path no longer exists in this cache. Actual source: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_PlayerSpells/ClassTalents/` and `Blizzard_SharedTalentUI/`:

- `Blizzard_ClassTalentsFrame.xml:26-42`: basePanOffsetX 49, basePanOffsetY 24, bottomPadding 82. `Blizzard_ClassTalentsFrame.lua:684-700`: ordinary nodes centered on ButtonsParent TOPLEFT.
- `Blizzard_SharedTalentUtil.lua:494-509`: `newX = (posX / 10) - offsetX`; y-up `newY = (-posY / 10) + offsetY`. Our canvas uses y-down.
- `Blizzard_TalentButtonArt.xml:26-34` and `Blizzard_TalentButtonSelect.xml:8-31`: 36×36 icons, 40×40 choice button.
- `Blizzard_HeroTalentsContainer.xml:12-15`: hero NodesContainer scale 0.85. `Blizzard_SharedTalentUtil.lua:206-215`: normalize hero x against subtree center, y against subtree minimum. `Blizzard_HeroTalentsContainer.lua:504-516`: anchor TOP, offset 0,8.
- `Blizzard_ClassTalentUtil.lua:126-169`: Selection + ShowMultipleIcons chooses split-icon template.
- `Blizzard_ClassTalentImportExport.lua:375-384`: granted first entry rank kept separate from purchased ranks; first entry is the grant recipient.
- `Blizzard_APIDocumentationGenerated/TraitConstantsDocumentation.lua:75-95,130-135,216-245`: Granted=2, Visible=1, IsSufficient=4; currency UseClassIcon=4, UseSpecIcon=8. None of the node flags means “grant this node.” No-cost is not learned-state evidence by itself.

## Local mapping / concrete witnesses

Mage ChrClasses ID 8 / Name Mage matches category-7 SkillLine ID 904. SkillLineXTraitTree row 40 maps 904→658. TraitTree row 658 exists. SpecSetMember rows 7/66/232/256/289/324 assign spec 62 to sets 2/21/160/161/162/173. Visible condition 18037 (group 7143) selects Arcane; roots 62117/62119/62121 overlap but Visible conditions choose only 62121. Condition 18032 grants its one rank for set 21. TraitTreeLoadout row 991 names tree658/spec62, but its allocations must NOT be applied.

Class currency2801 flags4/group7139 covers 42 visible paid nodes; the free/granted Arcane barrier root adds one =43. Spec currency2800 flags8/group7136 covers38. Hero selector99830 entries123344/123341 refer to subtrees40/39, not spells; these are navigation metadata, not icons. Sufficient visibility conditions26681/26682 keep Sunfury for Arcane OR Fire, not both. Neither eligible hero tree is activated/learned in this slice.

| Node | DB2 position | Entry → definition → spell | Outgoing edge IDs |
|---|---|---|---|
|62121|3900,1500|80180→85183→235450|127166,130124|
|62084|2100,3900|80140→85143→30449|130503,130504|
|102439|11100,3300|126509→131335→1241462|126342,126497|
|62087 (choice)|2700,6300|80143→85146→386763;134199→138979→157997|127184|

Exact CSV line numbers, rows and SHA256 hashes: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/talenttree-2026-10-08/csv-witnesses.txt`.

## Proof ledger

Evidence root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/talenttree-2026-10-08/`.
RED/GREEN and rendered inspection pending. Handoff: `/home/osso/.worktrees/handoff-talenttree.md`.
