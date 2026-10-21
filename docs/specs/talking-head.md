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
- Network relay and Godot model/audio host pending.

## Tests asserting this spec

- `godot/ui-model/tests/talking_head.rs`: actual source payload/elapsed expiry/private-owner refusal and dismissal.
- Ordered relay and native both-skin fixtures pending.

## Known gaps (current cycle)

- [ ] Protocol consumer, source TalkingHead model/chrome, audio and dismissal.
- [ ] Joint protocol/server/engine gate and native PNG inspection.

## Out of scope

Invented timing, dialogue, camera fitting or source-independent UI. Missing source assets remain explicit failures. This is the approved presentation slice after the server runtime proof, not a replacement for it.
