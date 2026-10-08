# Skyborne known model chain

Preserve the original Skyborn branch's known WoW Forever 70058 body mapping through the existing [`player_model_fdids`](../../godot/core/src/player_model_data.rs) API. This is partial known-map chain proof, not current full-catalog support.

## What it must do

- [ ] Resolve races 95 and 96, sex 0 to FDID 7478487 and sex 1 to FDID 7478494 from self-contained temporary CSVs through the real API.
- [ ] Retain existing API coverage for races 23, 24, 26, 52, 76 and 84.

## How it works

Known rows: race 95/96 → ChrModel 218/219 → display 139407/139408 → CreatureModelData 16480/16240 → FDID 7478487/7478494, respectively for sex 0/1.

The old runtime path-to-FDID patch is superseded by master's authoritative DB2 chain. Rebase keeps master's historical `character_model_data` module and tests unchanged; the incomplete hardcoded runtime table and retired Bevy paths are not restored. No production data overlay is installed.

## Implementation inventory

- `godot/core/src/player_model_data.rs` — existing DB2-chain API and known-map fixture.

## Tests asserting this spec

- `player_model_data::tests::skyborne_known_forever_models_follow_db2_chain` — four exact known race/sex mappings via temporary CSVs; no installed Forever data dependency.
- `player_model_data::tests::every_race_reaches_its_chr_model_body` — retained build-pinned retail API coverage.

## Known gaps (current cycle)

- [ ] Main-owned verification and integration remain pending; fixture proof does not establish installed catalog completeness or native rendering.

## Out of scope

Current full Forever catalog support, production data changes, new runtime APIs, factions, masks, UI, Bevy resurrection, server changes and deployment.
