# TalkingHead conversations

[Contract](../../specs/talking-head.md). Retail70448 `TalkingHeadUI.lua/.xml` and `ModelFrames.lua` govern the UI; Forever is art only.

## Runtime

The ordered EncounterChannel relay projects owner-scoped ConversationUpdate snapshots into `TalkingHeadState`. State advances authoritative elapsed time, retains dismissed object identities across unchanged snapshots, and clears on authoritative removal/world exit. RegistryUi hosts the source chrome/name/dialogue and shared scale/pointer registration.

`talking_head/voice.rs` reads the source SoundKit/Entry on a worker, extracts the actual Ogg from local CASC, seeks to line elapsed and stops on source replacement/local close/world exit. The Godot workspace must select asset-resolver's existing vendored IV8 patch: a dependency's own Cargo patches do not apply in its consumer workspace. Native playback state is proved in both skins; this host's dummy audio is not audible-output proof.

`talking_head_camera` reads Pos as model translation and LookAt as yaw/pitch/roll; ModelFrames supplies camera-space4→target0. The native host retains these source values as diagnostics. No model/camera rendering nodes are made: `TALKING_HEAD_MODEL_PROJECTION_UNSOURCED` covers unresolved Flags→useModelCenter and default PlayerModel projection. Allowed reference file/line receipts and bounded proofs live in the contract; no viewer-specific bounds fit/FOV is substituted.

## Sources

- [Contract and reference receipts](../../specs/talking-head.md) — exact Retail source paths, exclusions and proof bounds.
- [Native host](../../../godot/rust/src/talking_head.rs) — state, chrome, source-camera diagnostics and voice integration.
- [Source transform reader](../../../godot/core/src/talking_head_camera.rs) — raw source fields, fixed camera-space values and explicit projection gap.

## See Also

- [[ui-system]] — shared registry, native projection and art skins.
