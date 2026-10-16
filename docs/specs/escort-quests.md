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

Merged-server escort gate:36 passed,1 explicit private-account setup ignored; both Rust cargo checks and format checks pass. Modern and Forever native real-account runs prove acceptance, rendered authored speech and movement to the first source ambush, with inspected1920x1080 PNGs. Full earned native completion is NOT proved. No new client protocol or production UI behavior was added.

Missing verified payload receipts for source assassin textures923044/919332/1105445 block complete visual ambush selection; the private CSV appearance import does not solve payload ownership. Native attempts also exposed removed Healing Touch5185 (absent70391, Unknown spell); fixture now uses independently CSV-verified Regrowth8936 (heal10/periodic-heal8), whose protective live reproof remains open. Do not label these partial PNGs as a completed escort or claim full appearance parity.

## Authored ambusher appearance preparation

Native first-wave RED: server spawned four3879 and they attacked Feero, but client rejected actual displays19829/4227/4229 outside imported appearance coverage. This is missing data, not permission to substitute models. `scripts/prepare_escort_appearance.py --root <engine-slot> --source-world <read-only-world.db> --output <new-private.sqlite>` feeds the existing strict importer from active70391 Extra/Option/Geoset CSVs; selects all eleven actual template-model alternatives for Feero, ender and five ambusher templates; preserves the base cache, rejects conflicts and writes input/output SHA256 manifest. Private import:8 profiles/88 choices/11 coverage rows. Slot cache is de-linked into an owned overlay; canonical cache remains untouched. Runtime asset receipts and successful rendering remain separate proof.
