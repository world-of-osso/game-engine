# Spell Visuals

The Godot client plays each spell's Retail visual kits on replicated units. That means the caster's cast animation, the kit M2 models on attachment points, missiles, and impact kits on hit units. The data is the spell visual DB2 chain, exported from local CASC 12.1.0.69933. Server `SpellGo` and replicated `CastState` drive it. Melee swings and hit reactions come from `CombatEvent` (see [[animation]]).

## Data

`scripts/export_db2_csv.py` exports these tables from local CASC into `data/db2/12.1.0.69933/`. Each table is pinned to its WoWDBDefs layout:

| Table | FDID | Layout |
|---|---|---|
| SpellXSpellVisual | 1101657 | 7994A890 |
| SpellVisual | 897952 | 4B85C90F |
| SpellVisualEvent | 1685317 | 865F512E |
| SpellVisualKit | 897949 | C069D9C4 |
| SpellVisualKitEffect | 1140480 | E3206CA2 |
| SpellVisualKitModelAttach | 897953 | 02CF8554 |
| SpellVisualEffectName | 897948 | 2245CEE6 |
| SpellVisualAnim | 1140479 | F233613A |
| SpellVisualMissile | 897954 | AE389078 |
| AnimKitSegment | 1304324 | A6C970CA |
| AnimKitConfig / AnimKitConfigBoneSet / AnimKitBoneSet / AnimKitPriority | — | — |
| AnimationData | 1375431 | BBF66A3C |

Two export pitfalls:

- **Common-data fields.** SpellXSpellVisual and SpellVisualMissile use storage type 2 (per-record-id defaults), which the exporter now reads.
- **Pallet width.** Pallet entries are 32-bit even for `u8`/`u16` DBD columns, and their high bits are garbage. Narrow columns are masked (`i8`/`u8`/`i16`/`u16` sources). Without the mask, `AnimKitSegment.OrderIndex` reads as 38400+i and `BlendInTimeMs` as 142606486.

Encrypted sections with unknown TACT keys are zero-filled and dropped. SpellXSpellVisual cross-checks: it agrees on every shared row with the wago-sourced CSV already in `data/db2` (248,474 rows, 0 diffs).

Gotcha (2026-09-29): half the CASC index buckets were only in Syncthing `.idx.tmp` files while the WoW install was re-syncing, so `casc-local` reported "Archive location not found". The tables were read with an MD5-verified reader that also consults the temp indices. The runtime asset path needs a completed sync.

## Resolution (`godot/core/src/spell_visual.rs`)

1. **Pick the visual.** `SpellXSpellVisual` gives the visual: the highest `Priority` among `DifficultyID` 0 rows whose `CasterPlayerConditionID` holds.
   - `PlayerCondition` follows TrinityCore `ConditionMgr::IsPlayerMeetingCondition` for the fields it checks: Disabled/Invert flags, level, race and class masks, gender, `ChrSpecializationIndex` (passes without a spec), and `WeaponSubclassMask` against the main-hand `Item.SubclassID`.
   - A condition with any other requirement never holds.
   - Example: Slam with a one-hand sword resolves to visual 97446 (condition 87161, warrior + 1H mask 768145). With a two-hander it resolves to 51946.
2. **Start kits.** A `SpellVisualEvent` row starts a kit at `StartEvent` and ends it at `EndEvent`:
   - Events: 1 PrecastStart, 2 PrecastEnd, 3 Cast, 6 Impact, 7/8 Aura, 11/12 Channel, 13 OneShot.
   - `TargetType`: 1 caster, 2 every hit unit (`SpellGo.hit_targets`), 4 explicit target only. This is why Slam's blood does not land on the caster, although Slam's plan also hits the caster. 3 and 5 are area and missile kits and are not played.
3. **Kit effects.** `SpellVisualKitEffect` type 2 is a `SpellVisualKitModelAttach` (model from `SpellVisualEffectName`, attachment, offset, yaw/pitch/roll, scale × name scale, `StartDelay`, start/loop/end clips). Type 6 is the unit animation:
   - `SpellVisualAnim` with an `AnimKitID` uses its first `AnimKitSegment` clip. It loops when a segment loops back.
   - Otherwise it uses `LoopAnimID`, held unless the kit is a one-shot, else `InitialAnimID` once.
   - Examples:
     - Slam (1H): AnimKit 8451 → 818 CombatAbility1H01
     - Battle Shout: loop 55 BattleRoar, once
     - Frostbolt: precast AnimKit 13465 → 51 ReadySpellDirected (looped); cast AnimKit 13464 → 53 SpellCastDirected
4. **Missiles.** The missile is the visual's first `SpellVisualMissile`, travelling at `SpellMisc.Speed` in yards per second (TrinityCore `Spell.cpp:2515` `hitDelay += std::max(dist / m_spellInfo->Speed, m_spellInfo->MinDuration)`). For Frostbolt 116 that is model 1598570 at 35 yd/s, from attachment 56 to 34. `SPELL_ATTR9_MISSILE_SPEED_IS_DELAY_IN_SEC` (0x10) is unset for it. Frostbolt's `SpellVisualEvent` rows have no TravelStart kits and all start/end offsets are 0.
5. **Fallbacks.** `read_animation_fallbacks` loads `AnimationData.Fallback`. `WorldUnits` owns the table and resolves every combat, spell and stance clip a model lacks through it (e.g. 818 → 57 Special1H).

The catalog is cached as bincode at `data/cache/spell_visuals-12.1.0.69933.bin` (`db2_cache`).

## Runtime (`godot/rust/src/spell_effects.rs`)

- **Precast and channel.** A replicated `CastState` starts PrecastStart kits (Normal) or ChannelStart kits (Channel). They are held with their looping unit clip until the cast disappears or its `SpellGo` arrives.
- **`SpellGo` (new protocol message, TrinityCore `SMSG_SPELL_GO`).** It starts the Cast kits, then either readies the missile or starts the Impact kits. A missile's impact kits start on arrival.
- **Missile release.** Retail releases a pending missile on an M2 animation event of the caster's cast clip, not on `SpellGo`. wowdev.wiki/M2 "Events": `$CSL` and `$CSR` are "release_missiles_on_next_update if has_pending_missiles (left/right)", fired on "SpellCast*, ChannelCast*", and `$CST` the same without a hand.
  - The events are the MD20 array at 0x100 (`src/asset/m2_format/m2_event.rs`). For `.skel` models the definitions stay in the `.m2`, their tracks are indexed by the skeleton's sequences, and an external sequence's timestamps come from its `.anim` file's AFM2 chunk.
  - HumanMale HD and HumanFemale HD SpellCastDirected (53) fire `$CSL` at 200 ms (`$SCD`, the cast sound, at 200/133 ms). ReadySpellDirected (51) fires nothing.
  - The action layer marks a cast clip with a release event as awaiting it until its clip time reaches the event (`AnimationState::awaits_missile_release`). `SpellGo` readies the missile while the Cast kit's clip awaits. The missile leaves the first frame the clip no longer awaits: at the event, or when the clip is stopped or replaced first. Missiles released that frame start at the attachment and fly from the next frame.
  - A cast whose caster plays no clip with a release event (no Cast kit animation, a missing clip, or a higher-priority action keeping the layer) launches at `SpellGo`. No reference documents retail's behavior for that case. This rule is inferred.
  - `SpellEffects::flights` records each missile's `SpellGo`-to-release delay, launch distance, speed and flight time for automation.
- **Server timing.** game-server resolves Frostbolt's damage in the same tick as `SpellGo`: `execute_cast` calls `resolve_effects` right after `broadcast_spell_go`, and TRIGGER_MISSILE runs the damage spell 228597 immediately. `SpellMisc.Speed`/`LaunchDelay` are loaded but unused. The damage number therefore shows before the missile is thrown. TrinityCore delays each unit hit by `max(dist, 5) / Speed` (`Spell.cpp:2514-2515`, and `CalculateDelayMomentForDst` `:880-896` for destinations, plus `LaunchDelay`). That clock also starts at the cast and ignores the client's animation event, so even there the damage lands about 200 ms before the visual missile does. Reported, not changed.
  - The server broadcasts it from `execute_cast` to every connection in the caster's `NetworkVisibility`, or to every authenticated client for units without interest, so observers see other players' casts.
- **Kit model placement.** A model hangs from the unit model's `Skeleton3D/AttachmentBone{id}/Attachment{id}`; `-1` means the unit's origin. It plays its start clip once (the kit's, else Stand), Hold (158) while a held kit lasts, then Decay (159) at the end, and is freed after its particles' longest life. These defaults are inferred from how spell models author their emission. Battle Shout's buff 6194303 enables emitters at 133 ms of Stand, holds steady in Hold, and ramps down in Decay.
- **Virtual attachment 56.** VirtualSpellDirected is absent from character models. It is taken as the midpoint of SpellLeftHand 21 and SpellRightHand 22 (inferred). Any other missing attachment is reported and uses the origin.
- **Keyframed emission.** Spell effect emitters author their bursts as keyframed `emissionRate`/`emissionSpeed`/`enabledIn` tracks; the static first key is usually 0. `EmitterSim::set_animation` evaluates them at the placed model's playing sequence time, and pools are sized for the track's peak rate. Doodads get the same behavior.

## Proof (2026-09-29)

**Behavioral tests:**

- `godot/core/tests/spell_visual.rs`: Slam 1H/2H visuals, clips and impact model; Battle Shout roar, base and buff; Victory Rush by weapon; Frostbolt precast, hands, release and missile.
- `godot/core/tests/m2_particles.rs::keyframed_emission_follows_the_playing_sequence_time`.
- `godot/rust/src/animation/action_tests.rs` and `world_combat.rs` tests.
- `godot/core/tests/m2_events.rs`: SpellCastDirected `$CSL` at 200 ms on HumanMale/HumanFemale HD. `action_tests.rs`: `cast_clip_releases_missiles_at_its_release_event` (awaiting through frame 11 at 60 fps, released by frame 13) and `stopping_the_cast_clip_before_its_event_stops_awaiting_the_release`. `spell_effects_tests.rs`: travel time = distance / speed.
- Server test `spell_go_reaches_every_client_replicating_the_caster_with_its_hit_units`.

**Live fixture:** `godot/tests/spellcast_anim.gd`, run in a headless cage against a private server (UDP 5077). It covers a level-10 Human warrior (Worn Shortsword and shield) and `SPELL_SCENARIO=mage` (level-10 Human mage). Videos and stills are in `data/diagnostics/spellcast-anim-2026-09-29/`.

| Scenario | Observed |
|---|---|
| Warrior | Attack1H 17 swings, dummy CombatWound 9, Slam 818 with impact 1283017 on the dummy, Battle Shout BattleRoar 55 with 1138011 and 6194303 on the warrior |
| Mage | ReadySpellDirected 51 with hand models 1598571, SpellCastDirected 53, missile 1598570, impact 1599028 with the dummy's wound 9 |

**Auto-attack re-recording (2026-09-29b).** Before the fix, the server started auto-attack on every target change. Tab-cycling the mage swung unarmed (16) at each dummy it selected, which wounded them (9). With `AttackSwing`-only auto-attack, the fixture now:

- waits 4.5 s after Tab-targeting and requires no melee swing (16-19) by the player, no wound or crit (9/10) on any dummy it selected, and no auto-attack victim;
- has the warrior right-click the dummy in melee range (`AttackSwing`; the drawn-triangle pick chooses the dummy, and the warrior's own body still picks the warrior), stop with Escape (no swing for 4.5 s), reselect with a left-click (no attack), and restart with its Attack action (88163);
- requires the mage to show no swing and no dummy reaction until Frostbolt's impact.

Private server UDP 5079, game-server `23b9688`; mage at game-engine `f64ea296`, warrior at `9cb2d3a1`; both exit 0.

| Scenario | Observed |
|---|---|
| Warrior | Nothing during selection; right-click: Attack1H 17 and CombatWound 9; no swing after Escape; Attack 88163 swings again; Slam 818 with impact 1283017, Battle Shout 55 |
| Mage | Actions seen: player {51, 53}, dummy {9} only after the impact |

Videos (1x and half speed), stills and 2 fps contact sheets are in `data/diagnostics/spellcast-anim-2026-09-29b/`.

**Missile release timing (2026-09-29c).** Before this change the missile launched on `SpellGo`, while the hands were still at the chest, and hit before the arm thrust.

- The fixture now requires Frostbolt's `SpellGo`-to-release delay to be 200 ms (+1 frame at 15 fps) and its flight to be distance / 35 yd/s (+1 frame).
- Run: private server UDP 5083, game-server `23b9688`, fresh DB, account `fb_missile`, level-10 Human mage `Fbmissile` at (-8969.78, -154.5, 81.5). `SPELL_CAMERA_DISTANCE=9 SPELL_ORBIT=1.25`, `--write-movie --fixed-fps 30`. Exit 0.
- Result: `release_delay=0.233 distance=7.68 speed=35.0 flight=0.233 expected=0.219`. The 7.68 yd flight really lasts 0.22 s, so the fast-looking flight matched `SpellMisc.Speed`. The earlier problem was the launch time.
- Frames (30 fps, `release-sheet-1836-1859.png`):
  - 1845: `SpellGo`; the server's damage number 101 appears.
  - 1848: hands at the chest.
  - 1849-1850: the arm thrusts.
  - 1851: the missile leaves the hand, pointing at the dummy.
  - 1852-1857: flight.
  - 1858: impact 1599028 on the dummy.
- The damage number showing 13 frames before the impact is the server's same-tick damage (see Server timing).
- Evidence is in `data/diagnostics/spellcast-anim-2026-09-29c/`: `mage-frostbolt-1x.mp4` (release at about 61.5 s), `mage-frostbolt-halfspeed.mp4` (about 123 s), `mage-fixture.log` and `stills-mage/`.
- Root `cargo test -p game-engine --lib m2_` passes 92/92, including `m2_event::tests::human_male_hd_cast_clips_fire_their_release_events` (`tests/unit/asset/m2_event_tests.rs`: 53 fires `$CSL` bone 209 and `$SCD` bone 215 at 200 ms, 54 fires `$CST` at 200 ms, 51 fires nothing). Log: `root-m2-tests.log`.
- The Godot-workspace tests (`godot/core/tests/m2_events.rs`, the `action_tests.rs` release tests, `spell_effects_tests.rs`) are written but not run, because Cargo may not run in `godot/`.
- After merging master (`81a7ce02`), a real-time run (no movie, Depot build) passed with `release_delay=0.219 distance=7.71 flight=0.221 expected=0.220` (`final-merge/`).

## Gaps

- The action layer is full-body when standing and upper-body when moving (SpineLow subtree). `AnimKitSegment` conditions, per-segment bone sets and priorities, and `AnimKit` blend times are not applied.
- Other effect types are not played: sound kits, camera shakes, procedural effects, shadowy/outline/dissolve effects.
- Aura (7/8) kits, area and destination kits, and positioner offsets for attachment -1 are not played.
- `ChrSpecializationIndex` is always treated as no spec, and remote players' spec is unknown.
- Missiles fly straight: `SpellMissileMotion` is not applied. `SpellVisualMissile` `CastOffset`/`ImpactOffset`/`Flags` and `SpellVisual.Flags` are not applied (Frostbolt's offsets are 0).
- The missile starts at the missile attachment, not at the release event's bone and position. wowdev notes `$CSL/R/T are also used in CGUnit_C::ComputeDefaultMissileFirePos`, which is undocumented.
- Other events (`$SCD` cast sound, `$SHK` camera shake, `$FSD` footfall) are parsed but not played.
- Timed casts show no precast kits for observers until `CastState` replicates. Creature casts get kits only through `SpellGo`/`CastState`, like players.

## Sources

- `godot/core/src/spell_visual.rs`, `godot/rust/src/spell_effects.rs`, `godot/rust/src/world_combat.rs`, `godot/rust/src/animation/action.rs`
- WoWDBDefs (`~/Repos/wowless/vendor/dbdefs/definitions`) — layouts
- TrinityCore `ConditionMgr.cpp` `IsPlayerMeetingCondition`, `DBCEnums.h` `PlayerConditionFlags`
- WMVx `animation-names.csv` — animation ids
- wowdev.wiki [M2](https://wowdev.wiki/M2) "Events" and ".anim files", [M2/.skel](https://wowdev.wiki/M2/.skel); WebWowViewerCpp `M2FileHeader.h:97-104` (`M2Event`)
- TrinityCore `a352b1fa` `Spell.cpp:880-896, 2507-2515` — missile hit delay

## See Also

- [[animation]] — the action layer, melee clips and stance
- [[spellbook-action-bar]] — casting UI and messages
- [[m2-format]] — particle emitter tracks
- [[db2-format]] — WDC5 export
