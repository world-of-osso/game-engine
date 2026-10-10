# Extracted-only asset runtime

P1 separates runtime reads of extracted files from development CASC extraction.
The [contract](../../specs/shipped-assets.md) owns selection, error semantics and
the remaining P3–P5 work; this page describes P1 implementation boundaries.
[P2 model/product isolation](../../specs/product-isolated-model-assets.md) owns
source-qualified model chains and their extracted-only native acceptance.

## Runtime boundary

`AssetStartup::start` configures the process policy before any client asset
reader is constructed. Extracted-only returns an immediately ready startup
result without creating a CASC worker. Local-CASC retains asynchronous startup
and its explicit initialization failure.

The sibling `asset-resolver` checks extracted paths before loading an extraction
listfile. Its immutable process mode applies to direct constructors too; an
explicit extracted-only resolver remains restrictive even in a development
process. `guard_casc_access` protects CASC state acquisition, install discovery
and direct cache/extraction entry APIs before their filesystem work. The
monotonic denied-entry counter is updated before a hook can panic or callers can
swallow errors.

The subprocess regression starts with a clean environment/HOME and a small copy
of depot fixtures. It exercises the actual startup boundary, BLP/M2/WMO/ADT/DB2
parsers, and spell Ogg loader. It also tests an explicit required-file miss and
zero forbidden entries. A separate child deliberately swallows a tripwire panic;
the counter still makes that child fail. This is CPU asset-startup proof, not a
rendered gameplay or complete asset-closure proof.

## Detached legacy misses (2026-10-09)

Independent P1 verification found that the legacy cache API panicked on a shipped
miss inside detached AssetLoader workers. No result reached the main thread, so
its key remained Loading indefinitely. The legacy APIs now carry the checked
error in `Result<Option<PathBuf>, String>`; the inner Option preserves local-CASC
optional extraction. Ground-detail, WMO, model texture/animation and other worker
readers propagate the error. The parser's optional animation callback retains
its first required-file error for the enclosing model read to return.

`AssetLoader::poll` records Failed for Err completions, retaining Done for success.
Existing consumers log each handed-out error; repeated requests do not enqueue
known keys. Cold-process worker tests reproduce the missing-texture boundary and
load a real present BLP in an authored product/build namespace. RED at engine
`23586a033` + resolver `8fc769b`: worker panicked, key stayed Loading; present BLP
passed. GREEN at engine `f809ad04a` + resolver `cb64094`: startup7/7,
including both worker subprocesses; missing completion carries the exact checked
error, marks Failed/loading0 and is not repeated. The six engine packages passed
2,658/0/7 with `--no-fail-fast`, zero warnings. Resolver all-target testing with
`--no-fail-fast` passed31 and failed2 unchanged prerequisite boundaries:
`initialize.rs` has no WoW install, `negative_cache.rs` has no canonical listfile
at its hard-coded container path. Product-identity7/7 covers legacy scoped hits
and exact checked/legacy miss equality. Full logs live in canonical
`data/diagnostics/shippedassets-p1b-2026-10-09/`. Changed-file format/diff checks
pass; independent read-only verification could not start because Claude OAuth
expired. No claim of a clean resolver integration gate or rendered gameplay.

## Original P1 proof (2026-10-09)

Engine code `5dbe8b3c0` and resolver `8fc769b` (on modelisolation's `3157d12`)
passed the locked depot startup target 5/5 and all six engine packages: 2,656
passed, zero failures, seven existing ignored, no warnings. The RED subprocess
on the original startup returned `WoW install not found`. Changed-file rustfmt
checks and manual readability review pass. No rendered-game or complete-closure
claim follows from these CPU tests.

A combined run also passed resolver unit22/bin2, then stopped at its pre-existing
install-dependent `tests/initialize.rs`: depot has no WoW install. Default CASC
behavior remains unchanged; that environmental failure is not suppressed.
Evidence lives under
`data/diagnostics/shippedassets-p1-2026-10-09/` in the canonical checkout.

## Sources

- [Shipped-assets contract](../../specs/shipped-assets.md)
- [Startup boundary and cold-worker regressions](../../../godot/rust/src/asset_startup.rs)
- [Detached completion state](../../../godot/core/src/asset_loader.rs)
- [Ground-detail worker](../../../godot/rust/src/terrain/ground_detail.rs)
- [Depot fixtures](../../../godot/depot-test-assets.txt)

## See Also

- [[forever-data]] — source-qualified model/appearance work
- [[build-hosts]] — depot fixture test environment
