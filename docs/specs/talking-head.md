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
- `godot/rust/src/{talking_head,encounter,lib}.rs`: native owner/map-validated UI host, event routing and local dismissal; model/audio pending.

## Tests asserting this spec

- `godot/ui-model/tests/talking_head.rs`: actual source payload/elapsed expiry/private-owner refusal and dismissal.
- `godot/network/src/wire_tests.rs`: real UDP/Host ordered source snapshot and clear.
- `godot/tests/talking_head.gd`, `godot/rust/src/talking_head_fixture.rs`: native source-payload projection, authored model host and unscaled1920 PNGs; actual model/camera paint is an explicit prerequisite, not waived by chrome rendering.

## Known gaps (current cycle)

- [ ] Exact native UiCamera/GetUICameraInfo/model-space conversion and PlayerModel projection semantics remain unqualified; do not treat DB2 Pos as the camera eye or fit the mesh heuristically.
- [ ] Native source-model paint, voice playback/stop, authored animation/fades and joint protocol/server/engine acceptance remain open.

## Bounded proof

- Ordered real UDP relay and pure private-owner/dismissal state are implemented; these do not prove native world ingress or rendered models.
- Original1920×1080 Modern/Forever PNGs were inspected at `250064b00`: authentic atlas/name/dialogue render, native wrapping RED4→GREEN4. Model rectangle remains empty and the process fixture remains RED2. Evidence: `data/diagnostics/follower-dungeons-2026-10-21/talking-head-native-wrap-{red,green}/`.
- Exact selected70448 voice rows resolve250774/775/776→5834729/730/731 at volume0.799999952 (direct-kit RED→GREEN1). This metadata proof is not playback or audible-output proof; this agent host selected Godot's dummy audio driver.

## Out of scope

Invented timing, dialogue, camera fitting or source-independent UI. Missing source assets remain explicit failures. This is the approved presentation slice after the server runtime proof, not a replacement for it.
