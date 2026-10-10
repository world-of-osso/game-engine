# Skyborne NPC bodies — private native capture, 2026-10-10

Eight inspected PNGs show **Elatrell Featherlight, Uualia Suncrest, Blademaster Ren and Myriaal Mistwake** with their authentic baked body textures in both Modern and Forever. No renderer change was necessary. Scope is four photographed NPCs, seven mapped/attached displays, not full Zephras appearance acceptance.

## Data chain

CreatureDisplayInfo and Extra below are readable Forever70338 exports; TextureFileData is the existing Forever70205 export. Runtime appearance-cache rows independently agree on all seven bakes. Material IDs are not FDIDs: Extra.HDBakeMaterialResourcesID joins TextureFileData.MaterialResourcesID to obtain FileDataID.

| Bake FDID | Display | Extra | HD material | NPC entries/names |
|---|---|---|---|---|
|7344132|137339|162203|1067150|251366 Aetheen of the Gales;252478 Xy'aaria Streamrunner|
|7414459|136970|162551|1071483|251537 Uualia Suncrest;251991 Taleen Shimmerthread|
|7414802|138156|162557|1071617|251365 Jolee Brightmeadows|
|7415753|138170|162558|1071640|251964 Blademaster Ren|
|7474260|136967|162977|1081262|251368 Elatrell Featherlight|
|7507379|136981|163375|1085894|251389 Akeri Duskblade|
|7759343|143516|164474|1105184|263113 Myriaal Mistwake|

Read-only `content_creature_template_model` → `content_creature_template` → `content_creature` joins in canonical server `data/world.db` identify these map2991 spawns. Coordinates are **WoW x,y,z**, not Godot x,height,−y. `game-server-admin teleport` also takes WoW coordinates.

| NPC entry | Spawn GUID | x,y,z |
|---|---|---|
|251365|3251365000|4088.500790,1817.125,978.785686|
|251365|3251365001|4081.084130,1817.125,978.732921|
|251366|3251366000|4081.084130,1861.625,976.328572|
|251368|3251368000|4036.584171,1817.125,979.635409|
|251389|3251389000|4058.834151,1806,979.211034|
|251537|3251537000|4051.417491,1850.5,977.584969|
|251537|3251537001|4044.000831,1850.5,977.628488|
|251964|3251964000|4058.834151,1817.125,978.994581|
|251964|3251964001|4058.834151,1806,979.211034|
|251991|3251991000|3317.168159,1739.25,823.091281|
|252478|3252478000|2086.002614,715.75,665.144070|
|263113|3263113000|4066.250811,1806,979.781325|

Machine-readable source rows: `data/diagnostics/skynpc-2026-10-10/{bake-chains,npc-spawns}.json`.

## Runtime publication boundary

The seven newly published unqualified `textures/<FDID>.blp` files were insufficient for current master's [product-isolated model paths](../specs/product-isolated-model-assets.md): native preparation rejected matching NPCs with **No verified asset receipt** for their independently required hair/material textures. Files were not white replacements; the NPC visuals failed to attach.

Used the maintained `model_asset_frozen_extract` helper against authenticated local70338 resolution/archive bytes, then `import_model_asset_chains.py`. Initial NPC-sweep/body dependency traversal authenticated919 payloads; observed native receipt misses added15. No failed extraction, invented texture, legacy-byte relabeling, CDN or parser relaxation. All seven bake payloads match content-key MD5, SHA256 and size. Full traversed chains and SFID/SKID aliases were published to shared canonical `data/products/wow_classic_beta/f6e309c700cea095978aeb5d85210df4/` and `cache/model-asset-index.json`. The final canonical files were independently rehashed against their receipts. Source identity is70338, never70205.

## Proof ledger

Evidence root: canonical engine `data/diagnostics/skynpc-2026-10-10/` (not target).

- Engine source `2d04c70caf0f2f74d749a9af3ff51226a61c02dc`; server master `1b5d310e3f77f0ff5b327d029bb6a8afed3cc4bf`; both slots fetched/rebased origin/master, already current. Requested shared-data repair exit0.
- Native `agent-run skynpc python3 scripts/depot-build.py --root <engine-slot> --cli` exit0; server helper default binaries exit0; frozen extractor fixture build exit0. Logs: `engine-build.log`, `server-build.log`, `extractor-build.log`. No production Rust changes, no test-suite claim.
- Private fresh redb, UDP5487, `fb_skynpc`, race95/class3 `Skynpcview` ID31/level10. Server ground points at engine `data/`, not server `data/ground`. First cold client needed the normal headless import before extension classes were registered.
- All seven named displays logged `prepared_result_ok=true visual_attached=true` after publication. `client-modern-confirmed.log` and `client-forever-confirmed.log` prove native Options → HUD layout selection: `SKYNPC_LAYOUT_SELECTED Modern true` / `Forever true`. Final captures were retaken after explicit native selection; earlier configured-skin shots are not credited.
- Four NPCs × two skins captured through live IPC, each FFmpeg-downscaled to960px and individually inspected: colored skin and patterned body/clothes, no white/untextured bodies. `acceptance.json` records executable/bake/capture hashes and the8 user-visible paths. `capture-*-confirmed.json` retains capture/FFmpeg results.

## Captures and gaps

Only requested PNGs are published to `/syncthing/AgentShared/2026-10-10/skyborne-npcs/`:

`{elatrell,uualia,ren,myriaal}-{modern,forever}.png`.

Aetheen shares his spawn with Ryff/Tai'ree; both Jolee placements overlap Destin; Akeri overlaps Ren's first placement. Their native visuals attach, but overlapping pilot shots do not prove isolated body appearance and were not published. No spawn mutation or hiding of other units was used. The distant same-display Xy'aaria/Taleen spawns were mapped, not visited.

Unrelated native misses remain: Ventaari display139694 needs scoped receipt7487478.blp; display136974 needs Retail143167.m2. These are unpublished receipt gaps, not established local-CASC absence. Seven requested bakes have no missing-file gap.

Owned clients/server stopped by exact PIDs; `agents-skynpc.slice` inactive and private5487 listener absent. UDP5000 and other accounts/services untouched. Both slot targets are deleted at final handoff; evidence, shared products and branch remain.

## Sources

- [Forever data](../wiki/systems/forever-data.md) — runtime appearance policy and historical limits.
- [70338 recovery](../wiki/systems/offline-asset-closure.md#recorded-gap-recovery--verified-2026-10-10) — seven unqualified bake publications.
- [Server Skyborne system](../../../game-server/docs/wiki/systems/skyborne.md) — reconstructed map2991 population, starts and provenance.
