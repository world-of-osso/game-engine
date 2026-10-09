# Forever 1.60.1.70205 script fixtures

CPU-only, repository-owned inputs. No CASC extraction, native process, external
server checkout, or `data/` tree is needed to run the item tests.

## Items

- `items/source/*.db2.gz`: lossless gzip (mtime 0) of the eight original local
  CASC snapshots tracked in game-server `scripts/reference/forever-1.60.1.70205`.
  Exporter/server production SHA-256 pins still validate the decompressed bytes.
- `items/definitions/`: original matching DBDs and schema provenance from that
  same reference directory. Original production schema pins remain enforced.
- `items/server/scripts/`: frozen first-party reader implementation and compressed
  SQLite schema from game-server; not a fake reader or mock. Its dependency
  `zephras_creatures.py` supplies readable ID enumeration. No server is launched.
- `items/scaling/*.csv`: genuine completed 70205 item export files copied from
  engine `data/db2/1.60.1.70205/items/`, after checking every output SHA-256 against
  `items/original-manifest.json`. Complete exported row sets are preserved.

Original scaling DB2s/DBDs were unavailable on this host. `forever_fixture.py`
re-encodes these verified CSV values into explicit uncompressed WDC5 records and
matching test DBDs, retaining arrays and original row IDs from the manifest.
Layout `12345678` denotes this reconstructed test encoding, **not** the original
CASC layout. The child process overrides only scaling source/schema pins, like
commit 94249b031's name fixture. Item/loadout source/schema pins, production code,
and build 70205 remain unchanged. Generated fixture hashes are fixed before the
corruption tests mutate their copies. Scaling CSV output must match the preserved
CSV bytes exactly.

This proves exporter selection, real item decoding, reconstructed scaling decoding,
exact exported values, provenance, deterministic publication, and corruption
rejection. It does not prove decoding of unavailable original compressed scaling
snapshots. Snapshot reader updates should come from game-server, never test stubs.

## Liquids

`liquids/*.csv` are complete genuine exports copied verbatim from engine
`data/db2/1.60.1.70205/`; `*.retail-header.csv` are original headers copied from
`data/db2/12.1.0.69933/`. `sha256.json` records their snapshot digests. No numeric
or string values are invented. Counts: LiquidType 53, LiquidObject 1270,
LiquidMaterial 5, LiquidTypeXTexture 1028. Original extracted liquid DB2s and DBDs
were unavailable; tests reconstruct explicit WDC5 records/array schemas from these
preserved values, just as the scaling fixture does. Every encoded CSV byte must
round-trip, in addition to the original lake/object/header assertions. This proves
array decoding and Retail-header adaptation, not original compressed-source
layouts or extraction. Production build and importer code remain unchanged.

At e7c9fb890, `test_importer_decodes_liquids_with_retail_headers` errors at
`extracted_path`: no FDID 1371380 exists under the worktree's
`data/cache/forever-skyborne-extract/`. The test was coupled to untracked local
extraction/schema/header caches, not an observed importer defect.

## Failure evidence

At e7c9fb890, the three item success tests fail because
`/home/osso/.worktrees/game-server-skyborn/scripts/skyborne_data.py` is absent.
The two corruption tests error before invoking the exporter because the hard-coded
`/syncthing/Sync/Projects/world-of-osso/game-engine/data/forever-1.60.1.70205/`
source and definitions paths do not exist. No exporter/value-expectation defect was
observed after replacing those host dependencies.
