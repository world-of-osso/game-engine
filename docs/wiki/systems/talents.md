# Talents data/layout and server wiring

Contract: [Talents](../../specs/talents.md). Verified local inputs: 2026-10-08, Retail DB2 `12.1.0.69933`.

## Sources

Requested wowdev references: [TraitTree](https://wowdev.wiki/DB/TraitTree), [TraitNode](https://wowdev.wiki/DB/TraitNode), [TraitNodeEntry](https://wowdev.wiki/DB/TraitNodeEntry), [TraitDefinition](https://wowdev.wiki/DB/TraitDefinition), [TraitEdge](https://wowdev.wiki/DB/TraitEdge), [TraitNodeGroup](https://wowdev.wiki/DB/TraitNodeGroup), [TraitCond](https://wowdev.wiki/DB/TraitCond), [TraitSubTree](https://wowdev.wiki/DB/TraitSubTree), [SkillLineXTraitTree](https://wowdev.wiki/DB/SkillLineXTraitTree), [TraitTreeLoadout](https://wowdev.wiki/DB/TraitTreeLoadout). Join references: [TraitNodeGroupXTraitNode](https://wowdev.wiki/DB/TraitNodeGroupXTraitNode), [TraitNodeGroupXTraitCond](https://wowdev.wiki/DB/TraitNodeGroupXTraitCond), [TraitNodeGroupXTraitCost](https://wowdev.wiki/DB/TraitNodeGroupXTraitCost), [TraitTreeLoadoutEntry](https://wowdev.wiki/DB/TraitTreeLoadoutEntry). All14 returned HTTP403 on this host. Do not treat their unread text as evidence; saved responses live in the evidence directory below. Join-table schemas are directly read from local CSV headers.

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

Class currency2801 flags4/group7139 covers 42 visible paid nodes; the free/granted Arcane barrier root adds one =43. Spec currency2800 flags8/group7136 covers38. Hero selector99830 entries123344/123341 refer to subtrees40/39, not spells; these are navigation metadata, not icons. Sufficient visibility conditions26681/26682 keep Sunfury for Arcane OR Fire, not both. Eligibility alone does not activate a hero tree; the server snapshot's selected entry determines activation.

| Node | DB2 position | Entry → definition → spell | Outgoing edge IDs |
|---|---|---|---|
|62121|3900,1500|80180→85183→235450|127166,130124|
|62084|2100,3900|80140→85143→30449|130503,130504|
|102439|11100,3300|126509→131335→1241462|126342,126497|
|62087 (choice)|2700,6300|80143→85146→386763;134199→138979→157997|127184|

Exact CSV line numbers, rows and SHA256 hashes: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/talenttree-2026-10-08/csv-witnesses.txt`.

## Shared rendering

Main node centers use `/10` minus pan49,24, with 40px buttons and 36px icons. Capstones use64px buttons/61px icons (`Blizzard_TalentButtonArt.xml:247-262`, `Blizzard_SharedTalentUtil.lua:326-361`); tiered node110420 totals entry137026/137027/137028 ranks1+2+1=4. Its border is the cited `CapstoneCircle` art set (`Blizzard_TalentButtonArt.lua:186-204`). Class/spec currency labels anchor at372,45 and1211,45 (ClassTalentsFrame.xml:215-229). Choice buttons retain separate spell hover targets; clicks open a staged selection flyout, never a spell cast.

Both skins intentionally use the Retail talent-border atlas crops over the shared extracted Retail4556093 sheet; only window chrome follows the skin. Hero previews normalize center x/min y at0.85 scale. First preview top follows HeroTalentsContainer.xml:181-190 and12-15 (HeroSpecButton TOP102, height108; tree container above its bottom by34; NodesContainer down90 =>266). Second eligible preview is stacked304px below, retaining the original preview presentation. A hero selector stages one entry through the same pending model; local validation prevents purchasing in any unselected subtree.

Missing icon FDIDs remain metadata, while their textures bind `None` before asset discovery. This deliberately bypasses the global question-mark icon fallback. `icon-audit.json` and shared `logs/extract-wanted.tsv` record missing FDIDs; no fake art. Existing circle-mask composition clips passive icons. A missing optional spec background is not fabricated: the cited ClassTalentsFrame BlackBG remains black.

## Server wiring (2026-10-09)

The account owns `TalentEditor`, independent of whether PlayerSpellsFrame is open. NetworkBridge receives `TraitConfigSnapshot` and `TraitCommitResult`; the account sends `CommitTraitConfig` on the existing TalentChannel. Snapshot and commit ranks are **total ranks**, not purchased ranks (`game-server/class_progression.rs:466-480,822-837`). Core conversion subtracts grants before validating purchases.

`talent_data/rules.rs` ports the existing server evaluator from game-server master `7691f6b`; CSV rule metadata includes entry/node/group conditions and costs, required/sufficient parents, gates and level/conditional currency sources. Keep this port aligned with the server; persistence and final validation remain server-owned. Pending point displays start from server unspent amounts and apply cost/conditional-source deltas. A new snapshot is the commit boundary and replaces pending allocations; a success result alone does not invent committed state.

Retail click/commit/rollback and shared Undo/Reset-anchor citations live in the [contract](../../specs/talents.md#retail-references-local-source). Both skins share the same model and page.

## Proof ledger

Evidence root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/talenttree-2026-10-08/`.
Core RED `763f021b1`: five failed/zero passed, exit101. Core GREEN `cc9531ed8` (unchanged core in `ce90b9610`): five passed/zero failed, exit0, logs `/home/osso/.worktrees/logs/talenttree-{red,green-core}.log`. Native baseline fixture RED: actual1920×1080, exit1 at missing class/spec nodes (`native-red-capture.log`, existing extension SHA in `native-red-library.txt`). Queued `talenttree-red-ui.log` captured an intermediate module declaration before its new file existed: compile failure, NOT behavioral RED evidence. UI projection/legacy page checks at3de03af64:14 passed (12 frame +1 preview +1 talents), exit0; exposed shared-copy headers warning, corrected column() to use its accessor in58c3a6598. Capstone RED at58c3a6598:1 failed/0passed (40px versus64px), exit101, no warning. Capstone sizing/tier count correction80c2b32e3: final bounded UI GREEN14/14, exit0, zero warnings (`talenttree-green-ui-final.log`). Native extension build80c2b32e3: exit0, zero compiler warnings (`talenttree-native-build.log`); library SHA256 in `native-green-library.txt`. First capture stopped at a fixture-only one-pixel origin error: frame_layout rounds(1080−919)/2=80.5 to81. Correct native rectangles are472,210,40,40 and1192,390,40,40 (`native-geometry-oracle-red.log`). Corrected fixture candidate90500fdc9 cage exit0/hover text in both skins; inspection found fake white paint for missing icons and white/unrotated edges. Pixel decoding of all8 original/replay PNGs disproved apparent background overdraw; grant pixel stayed115,154,255 and layer indices stayed403/404/405. No background fix. Native `parts::base_image` collapsed None/SolidColor into white unrotated quads. Exact primitive RED57e079a3e:2failed/0passed,688filtered,zero warnings (`talenttree-red-parts.log`). Correctiondab4f624c: absent source paints nothing unless explicit background; solid color preserves tint/rotation; talent edge angle negates y-down atan2 for native counter-clockwise convention. New cage checks missing135729 at312,470 is black and diagonal127166 off-center527,265 is gray (rejects an unrotated horizontal line). Final primitive GREEN16/16,674filtered, zero warnings atcf82c94b0 (`talenttree-green-parts-2.log`); native build exit0/no compiler warnings (`talenttree-native-build-2.log`). Shared protocol advanced during the queue: one existing canonical52253c0d7 client initializer (`subcategory_filters: Vec::new()`) restored build compatibility; no protocol/server file edits. Final cage exit0 (`talenttree-capture-final.log` and evidence `capture.log`), actual1920×1080 with exact main node geometry, shared Prismatic Barrier tooltip and missing-icon/diagonal-edge pixels in both skins. All4 final PNGs read directly: class left, Arcane right, Sunfury/Spellslinger stacked; gold1/1 Barrier, grayunlearned nodes/diagonal edges,64px0/4 capstone; no white/question-mark substitute icons. Source/artifact hashes in `native-green-library-final.txt`; format check20changedRust files exit0 in `rustfmt-current.log`. Earlier core5 and UI14 proof paths are unchanged (later renderer changes verified by primitive/native pixel proof). Handoff: `/home/osso/.worktrees/handoff-talenttree.md`.
