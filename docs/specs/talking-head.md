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

- `godot/ui-model/src/talking_head.rs`: private source payload/elapsed line state and local dismissal.
- `godot/ui-model/src/talking_head_component.rs`: authored Retail TalkingHead chrome/name/text/model host geometry.
- `godot/network/src/lib.rs`: ordered ConversationUpdate relay on EncounterChannel.
- `godot/rust/src/{talking_head,encounter,lib}.rs`: native owner/map-validated UI host, event routing, shared UI-scale/pointer registration and local dismissal; model framing remains absent.
- `godot/rust/src/talking_head/voice.rs`: source-kit/file loading off-frame, elapsed seek, one active voice and local/world-exit stop. Native source extraction is blocked below.

## Tests asserting this spec

- `godot/ui-model/tests/talking_head.rs`: actual source payload/elapsed expiry/private-owner refusal and dismissal.
- `godot/network/src/wire_tests.rs`: real UDP/Host ordered source snapshot and clear.
- `godot/tests/talking_head.gd`, `godot/rust/src/talking_head_fixture.rs`: native source-payload projection, authored model host and unscaled1920 PNGs; actual model/camera paint is an explicit prerequisite, not waived by chrome rendering.

## Known gaps (current cycle)

- [ ] `TALKING_HEAD_MODEL_PROJECTION_UNSOURCED`: model rendering stays disabled. Source Pos→model translation, LookAt→yaw/pitch/roll and fixed camera(4,0,0)→target(0,0,0) are retained; flags→useModelCenter and native PlayerModel projection remain unknown. Do not treat Pos as the eye, substitute an M2 portrait camera, or fit the mesh.
- [ ] Native source-model paint, voice playback/stop, authored animation/fades and joint protocol/server/engine acceptance remain open.

## Bounded proof

- Ordered real UDP relay and pure private-owner/dismissal state are implemented; these do not prove native world ingress or rendered models.
- Original1920×1080 Modern/Forever PNGs were inspected at `250064b00`: authentic atlas/name/dialogue render, native wrapping RED4→GREEN4. Model rectangle remains empty and the process fixture remains RED2. Evidence: `data/diagnostics/follower-dungeons-2026-10-21/talking-head-native-wrap-{red,green}/`.
- Exact selected70448 voice rows resolve250774/775/776→5834729/730/731 at volume0.799999952 (direct-kit RED→GREEN1). This metadata proof is not playback or audible-output proof; this agent host selected Godot's dummy audio driver.
- Native voice RED6 was reproduced before implementation. The implemented consumer reaches local CASC extraction in both skins, but actual FDID5834729 fails BLTE decryption: encoding43dee1e72eb96dcd59fcfd62b353630d, IV size8 versus decoder's expected4. No replacement clip/CDN or suppressed error. See `talking-head-native-voice-diagnostic/process.log`; voice playback/stop remain unproved. The IV fix already exists in asset-resolver's vendored `cascette-formats` (`898cef7`, provenance/retirement: sibling `vendor/README.md`). Cargo ignores a path dependency's workspace patches, so the Godot workspace explicitly selects that same existing fix; no second crypto implementation.

## Camera reference boundary

- Retail70448 `CameraDocumentation.lua:31–41` and `ModelFrames.lua:70–99`: authored model transform and fixed camera/target. API `useModelCenter` is a returned Boolean, not a documented mapping from DB2 Flags.
- WebWowViewerCpp `1a8cccbe`, `wowViewerLib/src/engine/camera/m2TiedCamera.cpp:16–17,31–35`: M2-camera selection, authored clips and diagonal FOV; not native PlayerModel defaults. `persistance/header/M2FileHeader.h:120–135` describes M2 portrait camera types, not UiCamera flags.
- wow.export `c2fd7bde`, `src/js/components/model-viewer-gl.js:163–184,502`: viewer-specific bounds fitting and70° camera. These are not Retail framing evidence and are deliberately not copied.
- WoWDBDefs `definitions/UiCamera.dbd:6,27,58,81,116,148,179`: Flags column and storage widths only; no model-centering semantics. Saved schema SHA/qualified reference revisions: `data/diagnostics/follower-dungeons-2026-10-21/talking-head-camera-reference-receipts.json`.

## Out of scope

Invented timing, dialogue, camera fitting or source-independent UI. Missing source assets remain explicit failures. This is the approved presentation slice after the server runtime proof, not a replacement for it.
