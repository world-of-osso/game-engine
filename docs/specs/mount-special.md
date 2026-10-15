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

- Protocol `tests/mount_special.rs` — emote and mounted-identity wire round trips (RED pending).
- `godot/ui-model/src/ui/chat_frame_tests.rs::mount_special_slash_uses_the_emote_action` — slash input reaches an emote (RED pending).

## Known gaps (current cycle)

- [ ] New behavior and native proof are not implemented/verified yet.
- [ ] Retail `Bindings_Standard.xml:53-59` calls `JumpOrAscendStart` on key-down and `AscendStop` on release; the native C implementation is unavailable. The user-confirmed stationary/moving distinction governs this client; do not invent a hold timer.
- [ ] Current installed70391 has three distinct rider kits across six mount rows:19345 (Grrloc),23323 (Soaring Spelltome),3012 (DND test). Grrloc segment37708 starts600ms late at clip offset2000ms;23323 includes dependent/overlapping segments and reverse speed. Existing first-segment-only spell animation resolution does not establish full authored kit playback.
- [ ] Grrloc visual124416 includes SpellProceduralEffect10/34/17 and RopeEffect4. Lead source lookup2026-10-15T00:17 found no implementation defining these procedural/rope semantics in allowed sources. Explicit decision: retain these named gaps, implement animation and supported kit types only; do not fabricate procedural/rope rendering or claim full VFX fidelity. Rider segment condition/flag interpretation must also retain its actual source/proof boundary.

## Out of scope

Unrelated mount acquisition, UI redesign, new flight physics, fabricated ownership/data, canonical DB2/world writes, production deployment and unrelated full suites.
