# Source-backed TalkingHead

Consume the private source conversation payload and render the authored70448 Retail `TalkingHeadFrame` in Godot. The Forever skin changes art only. Source: cached `Blizzard_FrameXML/TalkingHeadUI.xml/.lua` and `Blizzard_SharedXML/Mainline/ModelFrames.lua`; server contract lives in game-server `docs/specs/conversations.md`.

## What it must do

- [ ] Receive the approved ConversationUpdate through the real ordered EncounterChannel relay, respecting map copy and private owner. Expiry/clear or leaving the world removes the presentation.
- [ ] Preserve source actor/display/camera, line/name/text/sound and authored line sequence; use payload elapsed time instead of restarting an existing line on refresh.
- [ ] Render the Retail570×155 frame, bottom96 anchor,115×115 model viewport at21/21, authored atlas chrome/name/text/close anchors and Lua fade timings. Both skins use the same controls/layout.
- [ ] Use actual source display116692/UIcamera1751 and line67085/67091/67092 with authored name/text/sound; never replace with an unrelated static portrait or fabricated dialogue.
- [ ] Close button/right-click dismiss locally without replaying the same active conversation on unchanged snapshots. Loading-screen/world exit clears it.
- [ ] Native1920×1080 PNGs prove both Modern and Forever after real message consumption, with inspected original pixels. CPU registry tests alone are not presentation acceptance.

## How it works

- [Native UI system](../wiki/systems/ui-system.md).

## Implementation inventory

- `godot/ui-model/src/talking_head.rs` and `talking_head/presentation.rs`: private source/elapsed state remains the voice authority; painted name/text follows Lua's250ms replacement and500ms Fadein callbacks without delaying the source line. An authoritative clear retains the no-voice1s Close painting; local dismissal/world clear remain immediate. Playing-voice close delay/fade is not implemented yet.
- `godot/ui-model/src/talking_head_component.rs`: authored Retail TalkingHead chrome/name/text/model host geometry and opening alphas from the conversation's elapsed time. Lua delays name500ms/text750ms; XML durations name/text250ms, portrait/model750ms, background750ms after400ms and close750ms after750ms. Repeated snapshots do not restart them. Painted line/close transitions live in the separate presentation state; playing-voice close and glow animation remain open.
- `godot/network/src/lib.rs`: ordered ConversationUpdate relay on EncounterChannel.
- `godot/rust/src/{talking_head,encounter,lib}.rs`: native owner/map-validated UI host, event routing, shared UI-scale/pointer registration and local dismissal; model framing remains absent.
- `godot/rust/src/talking_head/voice.rs`: source-kit/file loading off-frame, elapsed seek, one active voice and local/world-exit stop. Native source extraction is blocked below.

## Tests asserting this spec

- `godot/ui-model/tests/talking_head.rs`: actual source payload/elapsed expiry/private-owner refusal/dismissal and source opening alphas at625/875/1500ms in both skins. Opening-alpha RED: Modern625ms name remained1 instead of0.5 at `6c9b5da7a`. At `742b7bf1e`, all five TalkingHead model tests pass, including both-skin alpha samples at625/875/1500ms; native4/4 then checks source750→1500ms with216 checked steps and eight inspected1920×1080 phase/settled PNGs. The normal run emitted a4-instance ObjectDB teardown warning; unchanged verbose replay did not reproduce it. Its cause remains open, not fixed or suppressed. Logs: `talking-head-opening-red-exact.log`, `talking-head-opening-green.log` under the diagnostic evidence root.
- `godot/network/src/wire_tests.rs`: real UDP/Host ordered source snapshot and clear.
- `godot/tests/talking_head.gd`, `godot/rust/src/talking_head_fixture.rs`: native source-payload projection, authored host/source-camera metadata, voice playback/stop and unscaled1920 PNGs. Model paint is not claimed: the approved named projection gap is asserted explicitly.
- `godot/core/tests/talking_head_camera.rs`: exact source1751 Pos/Euler/raw Flags and authored fixed camera/target; no inferred native projection.

## Known gaps (current cycle)

- [ ] `TALKING_HEAD_MODEL_PROJECTION_UNSOURCED`: model rendering stays disabled. Source Pos→model translation, LookAt→yaw/pitch/roll and fixed camera(4,0,0)→target(0,0,0) are retained; flags→useModelCenter and native PlayerModel projection remain unknown. Do not treat Pos as the eye, substitute an M2 portrait camera, or fit the mesh.
- [ ] Playing-voice Close (source1s delay/2s sound fade), glow/scale/translation animation and joint protocol/server/engine acceptance remain open. Model paint is explicitly excluded under the approved projection gap, not silently certified.

## Bounded proof

- Ordered real UDP relay and pure private-owner/dismissal state are implemented; these do not prove native world ingress or rendered models.
- Original1920×1080 Modern/Forever PNGs were inspected at `250064b00`: authentic atlas/name/dialogue render, native wrapping RED4→GREEN4. Model rectangle remains empty and the process fixture remains RED2. Evidence: `data/diagnostics/follower-dungeons-2026-10-21/talking-head-native-wrap-{red,green}/`.
- Exact selected70448 voice rows resolve250774/775/776→5834729/730/731 at volume0.799999952 (direct-kit RED→GREEN1). This metadata proof is not playback or audible-output proof; this agent host selected Godot's dummy audio driver.
- Native voice RED6 was reproduced before implementation. The implemented consumer reaches local CASC extraction in both skins, but actual FDID5834729 fails BLTE decryption: encoding43dee1e72eb96dcd59fcfd62b353630d, IV size8 versus decoder's expected4. No replacement clip/CDN or suppressed error. Historical failure: `talking-head-native-voice-diagnostic/process.log`; playback/stop were unproved at that revision. The IV fix already exists in asset-resolver's vendored `cascette-formats` (`898cef7`, provenance/retirement: sibling `vendor/README.md`). Cargo ignores a path dependency's workspace patches, so the Godot workspace explicitly selects that same existing fix; no second crypto implementation.
- With the workspace override (`59b8d416a` native extension), both-skin source-voice playback, actual clip identity,750ms source seek and local-close stop pass. `talking-head-native-voice-ready/process.log`:228 checks, only two remaining model-paint failures. No ObjectDB leak warning on this completed-readiness path. Cold CASC worker readiness uses bounded30s wall time, not an arbitrary render-frame count; source lifetime/elapsed clocks stay unchanged. The agent host uses dummy audio: playback state is proved, audible output is not. Local Ogg provenance: `talking-head-voice-clip-receipt.json`.

- Source camera/voice reader batch at `a19238fff`:2/2 passed,0 warnings. Native host built with0 warnings; manual sampled fixture completed4/4 skin/input cases and204 checked steps. That manual launcher did not retain child exit status and its host audio initialization emitted ERROR before selecting Dummy: it is observational behavior evidence, **not strict native acceptance**. The fixture is registered with `gdscript_completion_fixture` at an independent expected4 and emits one `ALL_CASES_DONE` only after checked success. Its offline invocation selects Dummy explicitly and must pass complete child-stream/exit validation without an error allowlist.

### Strict native child proof — 2026-10-22

At `e60ea055a`, maintained `gdscript_completion_fixture` accepted4/4 with foreground runner exit0, exactly one `ALL_CASES_DONE 4/4`,204 checks and no ERROR in either complete Godot child stream. All four original1920×1080 PNGs were inspected: both skins render authored chrome/name/wrapped dialogue; button/right-click dismiss, source voice playback/750ms seek/stop pass. Evidence: `data/diagnostics/follower-dungeons-2026-10-21/talking-head-strict-native/`.

This is sampled offline presentation, not real world ingress, authored fades/glows, model paint or audible-output proof. Dummy audio is explicit. Outer Cage stderr reports unavailable XWayland; the tested Godot child uses native Wayland and has only the recorded platform/model-gap warnings. VRAM preflight sampled card0 rather than the selected NVIDIA adapter; post-run NVIDIA free7472MiB does not establish a matching preflight. The helper build log records both successful Cargo phases/zero warnings; its enclosing detached adapter failed before writing an exit-status artifact, so no helper exit-code claim is made.

### Transition RED boundary

`c595b0b2a` adds exact source705 lines1683/1684 (additional500ms each, missing locale duration retained) and actor98075's pinned TDB name; fixture map/entity IDs are controls. Source rows are recorded in `talking-head-705-source-rows.json`; canonical runtime actor-name coverage is not claimed. Source-model target tests reproduce2RED: new text replaced immediately at625ms instead of after the source250ms callback, and source clear hid chrome immediately instead of retaining the no-voice1s Close. Existing opening/layout controls remain GREEN. At `4abef5a71`, all seven TalkingHead model controls pass (0.11s, zero warnings), including both repaired boundaries. Native transition proof and playing-voice Close remain pending. `talking_head_source_close.gd` is registered with an independent expected2 and checks the actual source AudioStreamPlayer, gain/lifetime and1s UI delay/1s Close; it is the next unrun native RED, not acceptance.

## Camera reference boundary

- Retail70448 `CameraDocumentation.lua:31–41` and `ModelFrames.lua:70–99`: authored model transform and fixed camera/target. API `useModelCenter` is a returned Boolean, not a documented mapping from DB2 Flags.
- WebWowViewerCpp `1a8cccbe`, `wowViewerLib/src/engine/camera/m2TiedCamera.cpp:16–17,31–35`: M2-camera selection, authored clips and diagonal FOV; not native PlayerModel defaults. `persistance/header/M2FileHeader.h:120–135` describes M2 portrait camera types, not UiCamera flags.
- wow.export `c2fd7bde`, `src/js/components/model-viewer-gl.js:163–184,502`: viewer-specific bounds fitting and70° camera. These are not Retail framing evidence and are deliberately not copied.
- WoWDBDefs `definitions/UiCamera.dbd:6,27,58,81,116,148,179`: Flags column and storage widths only; no model-centering semantics. Saved schema SHA/qualified reference revisions: `data/diagnostics/follower-dungeons-2026-10-21/talking-head-camera-reference-receipts.json`.

## Out of scope

Invented timing, dialogue, camera fitting or source-independent UI. Missing source assets remain explicit failures. This is the approved presentation slice after the server runtime proof, not a replacement for it.
