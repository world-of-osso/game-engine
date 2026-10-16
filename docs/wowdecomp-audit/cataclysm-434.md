# Additional 4.3.4 source comparisons

Source-only expansion, **2026-10-15**. Five additional conditional gaps: A6, A7, C1, C2 and S3. No reference executable, client, server, shader, build or test was run. Fixtures below are acceptance requirements, not results.

## Pins and historical boundary

| Alias | Checkout | Revision |
|---|---|---|
| C | `~/Repos/wowdecomp15595` | `959b0b85c5f55c565328d7c3ca33027a65e41327` |
| E4 | game-engine | `3abdb6e6cd3273e08d65c97ddfa76090372af01a` |
| S | sibling game-server, inspected metadata only | `0dcaa23bde680678d72b3c43a786eb574bd809b8` |

`C/README.md:3–17` identifies **Cataclysm 4.3.4/build 15595**, principally x64, not Wrath or current Retail. `:21–37` explicitly separates address-tag coverage from behavioral correctness. `:46–52` says `%` and `#` address tags identify bodies carried from builds 13623 and 12340 without reworking them for 15595. Inspect tags per function; repository membership alone does not prove build-specific behavior. Its progress table is dated **October 16, 2026**, later than this audit's date; do not treat that table as a verified measurement made here.

C supplements, rather than replaces, the [original W/E/S pins](README.md#sources-and-applicability). E4 differs from E only in documentation; earlier implementation evidence remains pinned to E. New citations below use E4. None of the original 23 findings has been revalidated wholesale against C. Client reconstruction supplies no authoritative server policy.

## A6: low bone inheritance bits do not affect native pose composition

**Reference.** `C/Engine/Source/Model2/M2Model.cpp:5960–6047` combines authored and runtime bone flags. For a child with any low three bits, it copies the parent matrix and adjusts its basis/translation before composition. In particular, case `boneFlags & 6 == 2` normalizes each parent basis row and scales it to the corresponding model/world basis length; case `4` uses model/world basis directions with inherited lengths; case `6` uses the model/world basis directly. Bit `1` selects model/world translation; otherwise translation compensates around the already transformed child pivot. `:6082–6119` applies local animation and composes with this selected parent matrix. These are distinct from billboard bits `0x78` handled afterward (`:6123` onward).

**Native difference.** `E4/godot/core/src/asset/m2_format/m2_anim.rs:52–75` retains bone flags. `E4/godot/rust/src/assets/mod.rs:385–413` creates the authored parent hierarchy with pivot-relative rests. `E4/godot/rust/src/animation/mod.rs:223–235,323–347,1023–1038` builds local pivots, samples TRS and writes local bone poses without the low-bit-dependent parent adjustment. Billboard handling is real, but not an implementation of these three low bits.

**Required change.** Preserve the flag-controlled parent contribution when computing the final skinned pose. Do not reverse-engineer bit names from intuition: case `2` removes inherited scale relative to model scale, not inherited rotation. Do not disable ordinary hierarchy inheritance for unflagged bones.

**Acceptance fixture.** Use identity model transform and zero parent/child pivots. Animate only the parent's uniform scale to `2`; child local scale stays `1`. A vertex one unit from the child origin has transformed length `2` with child low bits `0`, but `1` with low bits `2` in the reference composition. The current ordinary hierarchy gives length `2` in both cases. Add independent parent translation and rotation cases for bits `1`, `4` and `6`, asserting positions and basis vectors after coordinate conversion, not internal function calls. Include a root-bone control, since this branch requires a parent.

**Limit.** No affected-model census or native skinning fixture. Runtime-added flags and modern inheritance contracts require their own evidence; the conditional omission is the authored parent-adjustment branch, not all animation hierarchy or billboarding.

## A7: authored attachment activation is discarded

**Reference.** `C/Engine/Source/Model2/M2Model.cpp:5463–5504` samples the attachment's `m_animateAttached` byte track and skips child `AnimateMT`/`AnimateMTSimple` when its current value is zero. `:7280–7305` likewise gates recursive child callbacks. `:8489–8498` excludes inactive attached children from recursive shadow-map batch gathering. Track state starts at `1` (`:7839–7846`). A zero value does not detach the child; skipping an animation call alone is not proof of hiding it in every color pass.

**Native difference.** `E4/godot/core/src/asset/m2_format/m2_attach.rs:7–12,20–60,63–88` reads attachment ID/bone/position but not the trailing animation block, including the SKA1 path. `E4/godot/rust/src/assets/attachments.rs:10–51` builds persistent bone/point nodes from those static records. `E4/godot/rust/src/assets/equipment.rs:175–203` reparents equipment there and explicitly shows it; no sampled attachment activation can reach this consumer because the track was discarded.

**Required change.** Retain and sample the attachment activation track and honor its applicable child participation gates. Preserve attachment identity and ownership. Do not turn this into unconditional node deletion, detachment or hiding, or assume the child clock freezes rather than catching up when reactivated; those outcomes need consumer/lifecycle proof.

**Acceptance fixture.** Parent with one attachment active at `0 ms`, inactive at `100 ms`, active at `200 ms`; attach a child containing a marker event and shadow-casting geometry. Observe child update/callback eligibility and shadow participation on each side of the keys, plus always-active and missing-track controls and a parent-sequence switch. Separately measure color-pass behavior and reactivation clock semantics before prescribing either. Exercise a real native equipment/spell attachment, not only parser retention.

**Limit.** No identified real asset using the track and no native lifecycle measurement. The established loss is activation-controlled child participation, not a proven screenshot defect or an invented general visibility rule.

## C1: camera obstruction probes have no lateral extent

**Reference.** `C/WoW/Source/UIUtil/Camera.cpp:3524–3563` first traces the center line, then calls `GetCameraDistance`. `:3688–3710` constructs eight corners around camera and target positions; this is a constant-cross-section corridor, despite the `CFrustum` name, not the camera's perspective viewing frustum. `:3713–3760` gathers world facets, transforms and clips them, and accumulates the collision extent. `:3764–3821` invokes this with size `0.25` (and size `0.3` for a separate terrain-flag branch), then shortens the camera distance. This is not a generic sphere sweep.

**Native difference.** `E4/godot/core/src/camera_follow_data.rs:56–122,128–181` uses terrain samples along the orbit line and a supplied mesh ray, including a second ray for the smoothed pose. `E4/godot/rust/src/camera.rs:132–187,193–223,241–274` supplies `PhysicsRayQueryParameters3D` on the terrain/WMO/doodad mask. The `0.3` collision offset subtracts distance after a hit; it does not give the query lateral width. Native WMO portals and camera collision already exist.

**Required change.** For an applicable camera policy, account for obstacles beside the center ray when resolving camera clearance, including the final smoothed pose. Determine target dimensions and filters from the supported client/product; do not copy historical constants or call this exact near-plane/FOV matching.

**Acceptance fixture.** Identity camera/target orientation with a nonvertical orbit. Put a small triangle/post inside the reconstructed corridor's side band but off the center line. Sweep lateral offset from clear to intersecting while holding orbit, terrain and collision masks fixed. Record final camera distance and geometry clearance; both center-ray-hit and completely-clear controls must still work. Use the reference corner construction and facet clipping as the historical oracle, not a claimed sphere/FOV equivalence. Repeat while smoothing moves around a corner.

**Limit.** No geometry or original-binary fixture was run; reconstruction math and supported camera dimensions remain unvalidated. This is a demonstrated query-shape difference, not a claim that every ray-based camera clips visibly.

## C2: close-camera target fade has no native camera-to-player binding

**Reference.** `C/WoW/Source/UIUtil/Camera.cpp:2431–2501` derives a byte fade from camera distance minus near clip, with pitch/vehicle qualifications. `C/WoW/Source/UIUtil/CameraModeLegacy.cpp:20–34` transfers it to the camera shot. `CameraManager.cpp:369–396` clamps the selected shot fade and calls `SetPlayerFadeCameraValue`; `C/WoW/Source/Ui/WorldFrame.cpp:1587–1603` resolves the target/root vehicle and calls `SetMaxAlpha`. `C/WoW/Source/Object/ObjectClient/Unit_C.cpp:11907–11933` writes `m_alphaScale`, and `:10722–10770` applies that scale to model fade alpha. Thus the fade has a model consumer, not just a computed field.

**Native difference.** `E4/godot/rust/src/lib.rs:2879–2895` passes the local player to the world camera. `E4/godot/rust/src/camera.rs:132–187` outputs camera position/look-at/current state, but no target opacity. The active visual attachment/replacement paths in `E4/godot/rust/src/world.rs:523–566,573–623` and player creation/light binding in `world_models.rs:40–61,519–550` receive no camera-derived opacity. Mount construction in `world_mount.rs:198–243` likewise attaches a visual without that input. Other ghost, creature-display and spell alpha systems are not evidence of camera-distance fading.

**Required change.** Bind applicable close-camera opacity to the local camera target's rendered visual, including its supported attached visual hierarchy and restoration after zooming out. Keep it distinct from authored/display/ghost alpha; do not replace those values. Vehicle fade policy and exact thresholds require target-build evidence rather than copying historical constants.

**Acceptance fixture.** Stationary player with opaque body and attached equipment against a high-contrast world background. Move camera from a clear third-person distance through body overlap to close zoom, then back out; repeat with collision pulling the camera inward and with shallow/steep pitch. Measure target-body/equipment alpha and background visibility, not only camera distance. Keep authored alpha and ghost state fixed; an opaque distant player and unrelated NPC provide controls. Verify mount/root-vehicle behavior separately where supported.

**Limit.** Bounded active local-player path inspection, not a repository-wide assertion that no opacity systems exist. No native pixels, affected-character census or exact modern fade equation established.

## S3: authored MCSH shadows do not reach terrain-surface shading

**Reference.** `C/WoW/Source/WorldClient/MapChunk.cpp:160–161` assigns the MCSH payload to `m_shadowMap`. `MapRenderChunk.cpp:267–297` allocates the terrain batch's shadow texture when world shadow enable bit `0x10` and a mask are present; `:308–311,402–403` also passes the mask to layer/shader texture unpacking. `MapRenderChunkUnpack.cpp:23–34,230–236` maps individual mask bits to shadow texels (`0x0000` versus `0xFFFF`). Most directly, `MapChunkRender.cpp:432–437` draws the terrain batch again using the shadow modulation texture and the authored mask. The pixel-shader path binds its combined texture at `:444–479`. This establishes a terrain-surface consumer independently of detail doodads.

**Native difference.** `E4/godot/core/src/asset/adt_format/adt/parsing.rs:702–713,752–765` parses/resolves shadow bytes; `E4/godot/core/src/adt.rs:51,98–122` retains them on chunks. But `E4/godot/rust/src/terrain/material.rs:269–300,351–400` constructs layer/alpha/animation inputs without the chunk shadow mask. `E4/godot/shaders/terrain.gdshader:5–29,105–175` consumes those layer inputs, vertex colors and dynamic light attenuation, not MCSH. `E4/godot/core/src/ground_detail.rs:166–169,293–299` samples the mask for detail-doodad shading, a separate implemented consumer.

**Required change.** Carry the applicable authored shadow mask into terrain-surface shading. Preserve layer alpha, vertex color and dynamic lighting rather than treating any of them as an interchangeable replacement. Establish the target renderer's modulation equation, enable policy and split-file ownership before choosing blending constants.

**Acceptance fixture.** Two otherwise identical textured chunks with identical geometry, MCCV, MCAL, lighting and dynamic shadows: one MCSH payload of 512 zero bytes, one of 512 `0xff` bytes. With the reference surface-shadow path enabled, mask texels and resulting shadow-pass coverage differ; the current native terrain inputs are invariant to this change. Add a half-mask pattern to verify orientation/chunk UVs, a missing-mask control, and disabled-shadow control. Inspect surface pixels separately from grass/doodad pixels. Exact brightness requires the reference modulation texture/equation, not an invented black-versus-white output assertion.

**Limit.** This upgrades the earlier uncompleted terrain-surface comparison using C; it does not retroactively prove the old W citation or all historical render modes. No native raster or modern authored-asset applicability proof.

## Matched mechanisms and leads not promoted

| Slice | Paired evidence / disposition | Outstanding proof |
|---|---|---|
| M2 texture rotation, scale and translation | `C/Engine/Source/Model2/M2Model.cpp:5051–5113` and `CM2SceneRender.cpp:52–120`; E4 `godot/core/src/m2_material.rs:166–185,204–225,367–458`, `godot/rust/src/assets/uv_animation.rs:117–139,148–169`, `assets/material.rs:230–247`, `godot/shaders/m2.gdshader:131–132` sample, compose, bind and apply centered affine transforms. Rejected missing-rotation/pivot claim. | Source match only; no rendered fixture or exhaustive vertex-shader variants |
| WMO liquid hidden tiles | `C/WoW/Source/WorldClient/LiquidGeomFactories.cpp:189–229,251–262,1157–1158` rejects low-nibble `0xf` tiles in ordinary/clipped geometry paths; E4 `godot/core/src/wmo_liquid.rs:106–128` omits hidden tile geometry, bound by `godot/rust/src/terrain/wmo_liquid.rs:42–61`. C `MapObjGroup.cpp:1235–1271` separately rejects those tiles in `QueryLiquid`. No blanket missing-hole claim. | Exact gameplay liquid queries, high-bit clipping behavior, boundary interpolation and flags are not paired by this bounded rendering match |
| Animated authored M2 cameras | `C/Engine/Source/Model2/M2Model.cpp:3269–3286,6306–6335` samples and outputs camera tracks; E4 `godot/core/src/asset/m2_format/m2_camera.rs:47–48,83–85,106–148` stores snapshots, used for framing at `godot/rust/src/char_create/scene.rs:192–220` and `unit_portraits.rs:470–483`. | No identified native scene requiring time-varying authored cameras or affected asset; do not promote parser capability difference alone |
| WMO MOUV animation | Native parser/shader limitation remains the [previous unresolved comparison](coverage.md#mouv-and-the-original-terrain-surface-mcsh-lead). No corresponding C WMO producer/equation was established. Liquid texture scrolling is not that evidence. | Target-build formula, animated UV set, real asset and native pixels |
| M2 shadow-batch eligibility | `C/Engine/Source/Model2/M2Model.cpp:4865–4869,8436–8486` has explicit material/batch/alpha gates. Ordinary native meshes are insufficient evidence either of parity or a specific defect. | Full native Godot shadow-pass eligibility/alpha behavior and target material applicability; no general shadow rewrite task |

## Coverage and next boundary

New paired omissions cover bone parent adjustment, attachment activation, camera clearance/fade, and terrain-surface authored shadows. Existing animation, material UVs, collision rays, liquid rendering and dynamic shadows must not be described as absent. Three unresolved leads above—authored camera animation, MOUV and shadow eligibility—remain distinct from the five previously numbered [validation candidates](validation-candidates.md).

Spell visual timing, audio attenuation/concurrency, networking, UI and server authority were not completed by this expansion. No numeric completeness or Retail parity follows from five more findings. Rework still requires the applicable product contract, real assets and a native counterexample.
