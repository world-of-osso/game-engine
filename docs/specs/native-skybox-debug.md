# Native skybox debug

Offline Godot `--screen skyboxdebug` must preserve the original authored-sky debug behavior in `src/scenes/skybox_debug/`, subject to the no-fallback requirement below. CLI parsing lives in `src/startup_args_data.rs`; launcher routing lives in `launcher/src/main.rs`. This is a feature contract within [Godot conversion](godot-conversion.md), not native completion evidence. See [skybox architecture](../wiki/systems/skybox.md) for source lookup and rendering boundaries.

## What it must do

### CLI

- [ ] Accept exactly the original paired `--skybox-fdid <u32>`, `--light-skybox-id <u32>`, `--skybox-time-ms <u32>` and valueless `--skybox-verify`. IDs and milliseconds accept 0 through 4294967295; parsing preserves the exact numeric value without conversion or clamping.
- [ ] Forced FDID and LightSkybox ID are mutually exclusive in either order, with an explicit error naming both flags. Missing values, invalid unsigned 32-bit values and unknown options produce meaningful errors.
- [ ] Preserve first-target and first-value semantics of existing client options. Repeated paired options still require a value; later values do not replace the first. Verification defaults false and repeated verification remains true.
- [ ] Route these options after Godot's `--`, preserving engine order, client order, explicit-separator behavior, non-UTF8 argument transport and existing routing errors. Verification consumes no following engine argument. Introduce no new flags.

### Offline scene and source selection

- [ ] Enter the selected authored-sky debug scene without authentication, server contact or global UI. Use the selected authored source unless an original forced FDID or LightSkybox ID selects another source; apply the millisecond time override when supplied.
- [ ] Unknown or unavailable authored sources fail explicitly. Never substitute a hardcoded fallback asset or invent a default model or light data.
- [ ] Default composition preserves original reference objects and authored/procedural composition semantics. `--skybox-verify` isolates authored sky against black without reference objects, visible procedural baseline or procedural fog.

### Camera and live rendering

- [ ] Start orbit focus at (0, 1, 0), distance 7.5; honor the existing camera FOV option. Left drag orbits and wheel zooms. Add no new controls.
- [ ] Display live authored sky geometry, textures/materials and animation, maintaining the original camera-relative sky behavior and time-dependent evaluation.
- [ ] Prove rendered authored content and real orbit/zoom input independently in the running native scene. Parser acceptance, scene dispatch or a nonempty screenshot alone cannot establish parity.

## How it works

- [Skybox architecture](../wiki/systems/skybox.md)
- [Godot conversion architecture and evidence](../wiki/systems/godot-conversion.md)
- [Original authored lookup](../skybox-authored-lookup.md)

## Implementation inventory

- `src/startup_args_data.rs` — shared flat `StartupArgs` fields and explicit CLI parsing.
- `launcher/src/main.rs` — original options routed to Godot user arguments.
- `src/main.rs` — preserved Bevy skybox resource insertion.
- `src/scenes/skybox_debug/mod.rs` — original scene, composition and camera behavior.
- `src/scenes/skybox_debug/resolution.rs` — original authored lookup and existing source-fallback boundary.
- `godot/rust/src/startup.rs` — native screen dispatch and consumer integration boundary; implementation/proof remains separate from parser support.

## Tests asserting this spec

- `godot/core/src/startup_args_data_tests.rs` — parsed values, boundaries, verification, first-value/target semantics, missing/invalid values and mutual exclusion. Native core GREEN must be established on Depot.
- `launcher/tests/process.rs` — three `skybox` process cases asserting exact recorded Godot argv with fake build/Godot executables; not native runtime proof.
- `src/scenes/skybox_debug/tests.rs` — legacy source, composition, FOV and camera-relative behavior references; not native acceptance.

## Known gaps (current cycle)

- [ ] Establish current parser GREEN and native startup consumption separately; CLI GREEN does not establish native parity.
- [ ] Resolve the existing legacy source-fallback incompatibility as a data/semantic boundary. Current default-source correctness remains unproved; no replacement default asset is authorized.
- [ ] Prove native default composition and rendering special cases, authored material/animation behavior and independent render/input behavior before marking native conversion complete.

## Out of scope

- Authentication, global UI, new controls, new CLI flags and platform expansion: not part of original offline debug behavior.
- Hardcoded fallback asset substitution: forbidden, not a compatibility option.
- Whole-client conversion acceptance: governed by the broader conversion spec.
