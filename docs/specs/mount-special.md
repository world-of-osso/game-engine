# MountSpecial

Mounted players perform their mount's authored special, not an ordinary jump while stationary. User correction2026-10-15 defines the stationary/moving input distinction. Retail's `Blizzard_ChatFrameBase/Mainline/ChatEmoteConstants.lua` names `MOUNTSPECIAL`; `Blizzard_PerksProgramModel.lua` previews it through AnimKit1371, whose installed segment selects AnimationData94. [Mount rendering](../wiki/systems/mounts.md).

## What it must do

- [ ] Stationary, grounded mounted jump triggers exactly the same MountSpecial action as `/mountspecial`, without leaving the ground. Moving mounted jump retains normal jump behavior; unmounted movement and flight remain unchanged. No hold-duration threshold.
- [ ] Play the mount model's AnimationData94 sequence through the authored AnimationData fallback chain; do not substitute a fabricated special or Stand. Return smoothly to ordinary mount/rider playback on completion or movement/dismount.
- [ ] Apply the mounted identity's `MountSpecialRiderAnimKitID` and supported effects of `MountSpecialSpellVisualKitID`, including all authored rider segments, delay/reverse playback and bone scope. Zero means no additional kit. Do not infer mount identity from a shared creature display. Lead decision2026-10-15T00:17 permits the explicitly named unsupported procedural/rope gaps below; do not invent their visuals.
- [ ] Other players on the same visible map copy observe the mount, rider and kit action through the existing emote path. The server identifies the actor; callers cannot animate arbitrary entities. No invented chat text or achievement TextEmote ID.
- [ ] Native proof covers at least three authentic mounts in both skins, including a nonzero rider kit, plus moving-jump preservation and peer observation. Forever changes art only.

## How it works

- [Mount rendering and journal](../wiki/systems/mounts.md).
- Protocol `EmoteIntent`/`EmoteEvent` and `Mounted` carry action and mount identity. TC a352b1fa `MiscHandler.cpp:1173-1180` broadcasts a special mount animation to the observer set; this project reuses its emote transport rather than adding an opcode.

## Implementation inventory

- `godot/rust/src/gameplay.rs` — native movement input and prediction.
- `godot/rust/src/world_mount.rs` — mount model, rider seating and animation.
- `godot/rust/src/world.rs`, `lib.rs` — replicated emote presentation.
- `godot/ui-model/src/ui/chat_frame.rs` — slash command parsing.
- Shared protocol `components.rs`, `protocol/core_messages.rs`; server `auras/mount.rs`, `messaging.rs` — authoritative mount identity and emote transport.

## Tests asserting this spec

- Protocol `tests/mount_special.rs` — actual emote and mounted-identity wire round trips, RED→GREEN2.
- `godot/ui-model/src/ui/chat_frame_tests.rs::mount_special_slash_uses_the_emote_action` — slash input reaches the unified emote, RED→GREEN1.
- Core `mount_special::{tests,kit_tests}` — stationary/moving/air selection; installed Grrloc600ms delay/2000ms offset/upper-body mask; all four Spelltome segments with reverse/overlap; simultaneous DND masks and visual19705 loop; explicit missing-clip error. GREEN6.
- Native `animation::mount_special_tests` — actual HumanMale HD rider bone poses and mount sequence ownership against per-frame locomotion. Final native proof remains pending.

## Known gaps (current cycle)

- [ ] Native both-skin three-mount/observer proof and final integration remain open. Native mount sequence ownership and actual late Spelltome rider poses pass. Lead decision2026-10-15T03:52 allows phase wrapping only for held/self-looping playback; oversized one-shot offsets remain explicit named gaps. HumanMale FishingCast133 lasts1000ms while Grrloc AnimStartTime is2000ms: report an error rather than silently suppress the entire segment or invent one-shot wrapping. Grrloc19345 and visual19705 therefore lack full rider acceptance on this model. Native proof uses Soaring Spelltome1532 for the complete four-segment rider case.
- [ ] AnimKit condition enums are not published in the allowed sources. Scheduling is explicitly inferred: Start0=kit start,1=referenced start,2=referenced end; End0=clip completion,1=held,4=referenced start+delay. Reverse speed starts at the clip end minus authored offset; zero blend times are immediate. Held/self-loop clocks wrap modulo resolved clip duration (WebWowViewerCpp `animationManager.cpp:924-926` wraps time beyond duration); applying that rule to initial AnimKit offsets is explicitly inferred, not a sourced native AnimKit rule. Installed-row timeline tests establish this implementation, not parity with unavailable client C code. Segment/config flags are retained; their unpublished bit semantics are not independently established.
- [ ] Installed additional mount visuals also reference effect type12; existing supported model/sound/animation dispatch does not implement that discriminant. No all-VFX claim.
- [ ] Retail `Bindings_Standard.xml:53-59` calls `JumpOrAscendStart` on key-down and `AscendStop` on release; the native C implementation is unavailable. The user-confirmed stationary/moving distinction governs this client; do not invent a hold timer.
- [ ] Current installed70391 has three distinct rider kits across six mount rows:19345 (Grrloc),23323 (Soaring Spelltome),3012 (DND test). Grrloc segment37708 starts600ms late at clip offset2000ms;23323 includes dependent/overlapping segments and reverse speed. Existing first-segment-only spell animation resolution does not establish full authored kit playback.
- [ ] Grrloc visual124416 includes SpellProceduralEffect10/34/17 and RopeEffect4. Lead source lookup2026-10-15T00:17 found no implementation defining these procedural/rope semantics in allowed sources. Explicit decision: retain these named gaps, implement animation and supported kit types only; do not fabricate procedural/rope rendering or claim full VFX fidelity. Rider segment condition/flag interpretation must also retain its actual source/proof boundary.

## Out of scope

Unrelated mount acquisition, UI redesign, new flight physics, fabricated ownership/data, canonical DB2/world writes, production deployment and unrelated full suites.
