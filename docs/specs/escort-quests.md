# Escort quest native acceptance

Source fixture: TDB1210 Feero Ironhand4484, quest976 Supplies to Auberdine, path35873. Extend existing authoritative server SmartAI; no client-invented quest completion or escort movement. Server contract and pinned TC source hashes: `../game-server/docs/specs/escort-quests.md` in the sibling checkout.

## Required proof

- Native fixture `godot/tests/escort_quests_live.gd` extends existing real-account quest pixel-input harness. `QF_STEPS=check_skin,check_escort`, `QF_SKIN=modern|forever`, `QF_ACCOUNT=fb_escort_modern|fb_escort_forever`, `GODOT_TEST_SERVER=127.0.0.1:5931`; `QF_ADMIN` points at owned server admin binary with matching GAME_SERVER_ADMIN_SOCKET.
- A real greeting/details/Accept click must precede observed walking, authored ambusher identities and server quest completion. Admin cannot fill976 objectives. Ordinary casts protect the escort; setup may teach local-source Wrath5176/Regrowth8936.
- Inspected1920x1080 PNGs in `/syncthing/AgentShared/2026-10-15/escort-quests/` required in both skins. Captures alone do not certify movement, combat, source speech or earned credit.
- Source speech and failure must have explicit observed proof; presence of script rows is insufficient. Cosmetic SmartAI emote5 remains a named unimplemented gap.

## Private fixture boundaries

Private consistent backup of supplied read-only skyborn-handoff world.db; exact legacy source spawn32783 for4484 is restored to content_creature. Ambient map1 spawns are excluded, retaining Feero and quest ender3663; all ten source ambushers still originate from authored summon groups. No actor stats, waypoints, scripts or quest rules change. Disposable accounts seed source prerequisite973, not escort976. This is not prerequisite-progression or reconciled Retail-world-population proof.

Cage's headless output defaults1280x720 and has no output-mode CLI on this host. Diagnostic `data/diagnostics/escort-quests-2026-10-15/cage-fullhd.c` interposes only compositor `wlr_headless_add_output` to request1920x1080 (no game/Godot symbols). Inspect original PNG dimensions; never upscale1280 evidence. Retire this test-only display adapter when Cage exposes output mode selection. No host display, service or package changes.

## Current state

Modern and Forever real private accounts earned authoritative976 credit after acceptance, rendered authored speech, walking, four assassins, three Scouts and all three final named ambushers. Both fixtures printed PASS ESCORT976 with all five identities and server_completed=true. Nine inspected original1920x1080 PNGs per skin are published as modern-earned-* / forever-earned-* under the required AgentShared directory. Runs used level80 players, ordinary Wrath5176/Regrowth8936 casts and no976 completion override. Server death-failure proof remains automated; these successful runs do not prove native death failure or universal appearance/VFX parity. No client protocol or production UI behavior changed.

Raw923044/919332/1105445.blp files existed and matched fresh local CASC extraction. ModelAssetIndex::receipt rejected them because both canonical and slot caches lacked(wow,FDID,blp) entries. Existing import_model_asset_chains tooling authenticated the bytes against a frozen334a36cf local resolution snapshot and published slot-private namespaced assets/receipts; no substitutes or receipt bypass. Package and exact proof are under data/diagnostics/escort-quests-2026-10-16/. Canonical publication is lead-owned.

The first unblocked native run then reproduced Feero becoming dormant at the second camp3756.44, two grids from his spawn. Server fix keeps active ScriptedPath actors awake, including paused ambushes and range-failure timers; same-coordinate grid RED→GREEN plus ordinary dormancy controls cover it. Final both-skin native runs cross that boundary and complete. The1200-second fixture deadline allows ordinary casts against ten authentic combatants plus48-node walking; it changes no gameplay timer. Earlier Healing Touch5185 was absent70391; corrected Regrowth8936 has separate236→484 native health proof. Missing player texture/VFX receipts and cosmetic emote5 remain named visual gaps, not escort credit substitutes.

## Authored ambusher appearance preparation

Native first-wave RED: server spawned four3879 and they attacked Feero, but client rejected actual displays19829/4227/4229 outside imported appearance coverage. This is missing data, not permission to substitute models. `scripts/prepare_escort_appearance.py --root <engine-slot> --source-world <read-only-world.db> --output <new-private.sqlite>` feeds the existing strict importer from active70391 Extra/Option/Geoset CSVs; selects all eleven actual template-model alternatives for Feero, ender and five ambusher templates; preserves the base cache, rejects conflicts and writes input/output SHA256 manifest. Private import:8 profiles/88 choices/11 coverage rows. Slot cache is de-linked into an owned overlay; canonical cache remains untouched. Runtime asset receipts and successful rendering remain separate proof.
