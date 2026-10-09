# Shipped assets

## P1 runtime contract

Select `GAME_ENGINE_ASSET_MODE=extracted-only` in the launcher/player environment.
Unset means `local-casc`, preserving development behavior. Unknown values fail
startup. Selection precedes every asset reader and is immutable for the process.
All files needed by a shipped game are extracted ahead of time; runtime must not
require a WoW install or CASC.

`asset-resolver::AssetRuntimeMode::{LocalCasc, ExtractedOnly}` and
`AssetResolverConfig::with_runtime_mode` are shared with model/appearance isolation.
Extracted-only skips the startup CASC worker. Resolver initialization is a no-op;
local cache hits load normally, without loading a listfile for extraction.
The single `guard_casc_access` policy check protects low-level CASC entry and
install discovery, including direct resolver construction and terrain raw DB2
requests. No archive, root, encoding, keyring or listfile-driven extraction occurs.

Required misses return errors containing mode, FDID (and authored product/build
when present), and the exact expected path. Checked APIs propagate these errors.
Legacy Option APIs must fail explicitly in extracted-only mode, not return a
silent None. `forbidden_casc_access_count()` records forbidden low-level entries;
a test hook can panic immediately, and cold-process tests assert zero even if a
caller catches or logs an error. An ordinary extracted-file miss is not a CASC
entry and does not increment this count.

## Later phases (not P1)

- **P2:** Authenticated product/build identities, receipts and manifest-qualified
  bytes; prove same-FDID chains coexist without legacy-path substitution.
- **P3:** Offline fixed-point closure for one map and complete character/NPC/item/
  spell/audio/UI slice; pristine no-install bundle and deterministic missing-file
  failure.
- **P4:** Extend deterministic seeds to all supported content; zero unresolved
  closure, scenario recordings contained in the manifest, valid receipts.
- **P5:** Package only verified manifest members; remove deployment's install
  requirement, compile out CASC in release, sandbox startup/scenarios and validate
  download hashes. No deployment changes are part of P1.
