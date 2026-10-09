# Extracted-only asset runtime

P1 separates runtime reads of extracted files from development CASC extraction.
The [contract](../../specs/shipped-assets.md) owns selection, error semantics and
the remaining P2–P5 work; this page describes implementation boundaries.

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

## Sources

- [Shipped-assets contract](../../specs/shipped-assets.md)
- [Startup boundary](../../../godot/rust/src/asset_startup.rs)
- [Depot fixtures](../../../godot/depot-test-assets.txt)

## See Also

- [[forever-data]] — source-qualified model/appearance work
- [[build-hosts]] — depot fixture test environment
