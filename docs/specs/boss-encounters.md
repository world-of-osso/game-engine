# Boss encounters (client)

> Root `src/` paths and `cargo test --bin game-engine` selectors below name files and tests deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Dungeons phase 3: what the client shows of a scripted boss fight. Server contract: game-server `docs/specs/boss-scripts.md`; protocol shared-protocol `protocol/encounter_messages.rs` and `ChatType::Monster*`. Code: `src/game/networking/encounter.rs`, `src/ui/raid_warning.rs`, `src/scenes/raid_warning_frame/`, boss frames in `godot/ui-model/src/ui/screens/inworld_unit_frames_component.rs` and `src/rendering/ui/unit_frames.rs`.

## What it must do
- [x] Creature chat reads as Retail lines: `MonsterSay` "Name says: text", `MonsterYell` "Name yells: text", `MonsterEmote`/`RaidBossEmote` the text with `%s` replaced by the speaker, in the Retail default ChatTypeInfo colours (say 1/1/0.624, yell 1/0.251/0.251, emote 1/0.502/0.251, boss emote 1/0.867/0).
- [x] A `RaidBossEmote` also shows center screen in `RaidWarningFrame` (TOP −182, 800 wide): fades in 0.2 s, holds 10 s, fades out 3 s; at most 4 lines, a fifth evicts the oldest.
- [x] `EncounterEngageUnit` puts the unit on a boss frame (`boss1..5`, lowest priority first, repeats ignored); `EncounterDisengageUnit` removes it; `EncounterStart`/`EncounterEnd` track the running encounter; the loading screen clears both.
- [x] `Boss1TargetFrame`..`Boss5TargetFrame` stack on the right (60 px from the right edge, first at 300 px, 10 px apart) with the boss's name, level, health and power on the target frame art; unused ones hide; clicking one targets its boss (`FrameUnits::for_root`).

## How it works
- `docs/wiki/systems/ui-system.md` (unit frames)

## Implementation inventory
- `src/game/networking/encounter.rs` — `EncounterFrames`, encounter message handlers
- `godot/ui-model/src/game/chat_data.rs`, `godot/ui-model/src/ui/chat_frame.rs`, `src/game/networking/messages.rs` — monster chat types and lines, boss emote → RaidWarnings
- `src/ui/raid_warning.rs`, `src/ui/screens/raid_warning_frame_component.rs`, `src/scenes/raid_warning_frame/mod.rs` — RaidWarningFrame
- `godot/ui-model/src/ui/screens/inworld_unit_frames_component.rs`, `src/rendering/ui/unit_frames.rs` — boss frames

## Tests asserting this spec
`cargo test --bin game-engine encounter`, `creature_texts`, `raid_boss_emote`, `boss1`, `raid_warning`, `hud_layout`.

## Known gaps (current cycle)
- [ ] No chat bubbles over speaking creatures.
- [ ] Boss frame flair art (`Target-Boss-Small`), boss cast bars and alternate power bars are not drawn; the frame position is the reference-resolution slot, not the edit-mode right-managed layout.
- [ ] Creature text sounds (BroadcastText SoundKit) are not played.

## Out of scope
- Encounter journal integration, boss timers (encounter warnings), `RAID_BOSS_WHISPER`.

## Live evidence (2026-09-27)
Headless client against the stockadebosses server on :5085, evidence in `data/diagnostics/stockadebosses-20260927/`: "Hogger yells: Forest just setback!" and Boss1TargetFrame "Hogger 32" (`ui-02`, `02`); "Hogger enrages!" center screen and in chat (`04`); the boss frame hid on the reset and on death; ENCOUNTER_START/END 1144, 1145, 1146 in the client log with success false after resets and true after kills; Lord Overheat's boss frame showed his mana (`13`); Randolph Moloch's vanish emote and Mortimer's yell and collapse emote (`22`, `ui-26`).
