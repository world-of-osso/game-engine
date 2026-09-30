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
| SoundKit | 1237434 | A7FB0451 |
| SoundKitEntry | 1237435 | 8F82FF7D |

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

The catalog is cached as bincode at `data/cache/spell_visuals-12.1.0.69933-v{CACHE_FORMAT}.bin` (`db2_cache`). The format is in the name because checkouts share `data/`: with one file name, a branch at another format rebuilt over it on every run. `SpellEffects` loads the catalog on a worker thread when the client starts. The first cast waits only if the worker has not finished. A rebuild parses 1.3M `SoundKitEntry` rows. When that ran on the main thread at the first cast, it froze the client through the cast (see Proof, cast timing).

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
- **Kit sounds (`godot/rust/src/spell_sounds.rs`).** A `SpellVisualKitEffect` of type 5 is a `SoundKitID` (WoWDBDefs `SpellVisualKitEffect.dbd`).
  - When its kit starts on a unit, the kit plays one of the SoundKit's `SoundKitEntry` files, weighted by `Frequency` (0 never plays), at `SoundKit.VolumeFloat` × entry `Volume` × master × effects.
  - The sound plays on an `AudioStreamPlayer3D` named `SpellSound{fdid}` under the unit's node. `unit_size` = `MinDistance` (Godot's inverse attenuation is 1 there) and `max_distance` = `DistanceCutoff`. This approximates retail's full-volume radius and cutoff; the retail curve is not documented.
  - A SoundKit with Flags 0x200 (looping, WoWDBDefs `SoundKit.dbd`) in a held kit loops until the kit's end event.
  - Files come from local CASC, cached at `data/sounds/spells/{fdid}.ogg`.
  - No spell kit's `SoundKitEntry` has a `PlayerConditionID` in 12.1.0.69933 (429,342 entries across 88,635 referenced kits).
  - **Missile travel sound.** `SpellVisualMissile.SoundEntriesID` is a SoundKit the missile plays under its own node while it flies (Fireball 349558, looping). The sound is freed with the node on landing. Frostbolt's is 0.
  - **Aura kits.** `SpellEffects::sync_auras` (`spell_auras.rs`) starts a spell's AuraStart kits when a replicated `UnitAuras` instance appears. The aura's unit is the hit unit, and its caster (the unit when unknown) is the caster. Held kits (end AuraEnd) last until that instance leaves, which stops their loops, models and clips; the spell's AuraEnd-start kits then play. Example, Frost Nova 122: kit 266131 loops 350097 and plays 350098 while rooted, and AuraEnd kit 85719 plays 85938.
  - **Unit voice** (`spell_visual_voice.rs`).
    - A kit effect of type 10 (WoWDBDefs enum `UnitSoundType`) plays the kit's unit's own `CreatureSoundData` sound.
    - The value → field mapping is undocumented. 34-40 are inferred from the 12.x field order: Windup, WindupCritical, Charge, ChargeCritical, BattleShout, BattleShoutCritical, Taunt. The evidence is Charge's kit 44000 using 36 and Battle Shout's 43995 using 38. Other values are not played.
    - A unit's row is `CreatureDisplayInfo.SoundID`, else its model's `CreatureModelData.SoundID`. A player's display comes from `ChrRaceXChrModel` → `ChrModel.DisplayID`. For example, Human male → display 57899 → model 7661 → CreatureSoundData 49 → BattleShout 58088 (7 files); female → 50 → 58100.
    - The cast clip's `$SCD` M2 event (the action layer reports it) plays `SpellCastDirectedSoundID`. Only 3 rows set it in 12.1.0.69933, and Human 49/50 are 0.
  - **Synthetic outcomes.** The Godot client plays no synthetic CastStart sweep (removed) and no synthetic Impact/Heal outcome; impact kits carry those sounds. Miss and Interrupt outcomes stay synthetic ([[sound]]).
  - **SoundKits by spell (12.1.0.69933):**

    | Spell | Kit sounds |
    |---|---|
    | Slam 1H | cast 128672 → 57845, impact 62452 → 60935 |
    | Battle Shout | cast 43995 → 114049 plus the caster's BattleShout voice |
    | Fireball | precast 349555, cast 349556, missile 349558 (looping), impact 349557 |
    | Frost Nova | cast 350096; aura 350097 (looping) and 350098; aura end 85938 |
    | Flash of Light | precast 349350 (looping), cast 349352 and 349351, impact 349357 and 349355 |
  - Frostbolt (visual 64829) resolves to these SoundKits:

    | Kit | Event | SoundKit | Files |
    |---|---|---|---|
    | 81575 | PrecastStart | 85501 (vol 0.30) | 1631391-1631394 `spell_ma_revamp_frostbolt_precast_start_01-04` |
    | 81575 | PrecastStart, looped until PrecastEnd | 85500 (vol 0.35, 0x200) | 1631387-1631390 `precast_loop_01-04` |
    | 81337 | Cast (`SpellGo`) | 85502 (vol 0.35) | 1631379-1631382 `frostbolt_cast_01-04` |
    | 80718 | Impact (missile arrival) | 85503 (vol 0.8, MinDistance 25, cutoff 55) | 1631383-1631386 `frostbolt_impact_01-04` |
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
- The Godot-workspace tests later ran on Depot (see Frostbolt kit sounds).
- After merging master (`81a7ce02`), a real-time run (no movie, Depot build) passed with `release_delay=0.219 distance=7.71 flight=0.221 expected=0.220` (`final-merge/`).

**Frostbolt kit sounds (2026-09-29).** The mage fixture now requires Frostbolt's four SoundKits, with the right FDID range, unit and looping flag, at their events:
- the precast pair with the cast bar;
- 85502 at `SpellGo`;
- 85503 at the missile's landing;
- no precast-loop player left once the cast resolves.

Each time is checked within two of the longest frames since the press.

- **Real-time run** (`sounds/mage-fixture.log`, exit 0):
  - press at 197.533;
  - 85500/85501 (1631390 looping, 1631392) at 197.669;
  - 85502 (1631381) at `SpellGo` +0.000;
  - 85503 (1631386) on the dummy at +0.540, which is release 0.291 + flight 0.250.
- **`--fixed-fps 30` run** (`sounds-fixed30/mage-fixture.log`): cast sound exactly at `SpellGo` (108.224), impact exactly at release + flight (108.690).
  - That run failed the fixture's own "no dummy reaction before the impact" check. In movie mode the real-time server resolved the 1.75 s cast one movie frame after the precast, so the missile landed during the fixture's 20-frame precast wait.
- Real-time runs failed that same check 3 times out of 6. In those runs the client clock showed 0.03-0.14 s from the precast kit to `SpellGo`, against a 1.75 s cast. **Cause: a main-thread catalog build at the first cast** (2026-09-29, `data/diagnostics/spellcast-anim-2026-09-29c/`).
  - **Server timing is right.** The client logs the wall time at which it first sees `CastState` and `SpellGo`. When the catalog cache was fresh, `CastState` → `SpellGo` took 1.771 s wall (`elapsed` 0.000 at first sight).
  - **When the catalog cache was stale,** the same gap was 25.3, 26.3 and 29.8 s wall, but only 0.07-0.10 s on the client clock.
  - **eu-stack of the main thread** during that gap, sampled every ~4 s for 30 s, is in `SpellEffects::catalog` → `SpellVisualCatalog::load` → `build` (`Table::read`/`parse_csv_line`, `read_speeds`, `read_sound_kits`). The first `CastState` asked for the catalog, and the cache missed, so the client rebuilt it from the CSVs inside one frame while the whole cast resolved.
  - **The cache missed every run** because checkouts share `data/cache/spell_visuals-12.1.0.69933.bin`. This branch writes format 5 and master clients write format 4, so each rebuilt over the other: the file held format 4 (first byte `04`) right after a master client ran.
  - **The client clock shows ~0.1 s for a ~26 s frame** because Godot 4.7.2 drops time from the process delta past `max_physics_steps_per_frame` physics ticks (`main/main.cpp:4955-4958`: `process_step -= (advance.physics_steps - max_physics_steps) * physics_step`; the default of 8 gives 133 ms). That is why `longest_frame` read 0.13-0.15 s.
  - **Fix** (client):
    - The catalog loads on a worker thread from client start.
    - Its cache file is named per format (`spell_visuals-12.1.0.69933-v5.bin`), so branches no longer rebuild over each other.
  - A real player would also hit a multi-second hitch at their first cast after any CSV update (a cold cache), since the build ran on the main thread.
  - Sound playback costs 1-1.5 ms per start (instrumented run), so it is not the cause.
  - The fixture checks now use the effects clock, not frame counts:
    - Dummy reactions must come after the landing.
    - The precast sounds must start with the replicated `CastState`.
    - Every tolerance is two of the longest frames since the press.
- **Tests (Depot `--test -p game-engine-godot --lib`, `depot-godot-lib-tests.log`):** `action_tests.rs` (8, with the two release tests), `spell_effects_tests.rs` (2) and `spell_sounds_tests.rs` (2) pass. The library run has 64 other failures, all reading `data/` files the Depot snapshot does not stage (e.g. `Emotes.csv`, `NameGen.csv`). `--test -p game-engine-core --test spell_visual --test m2_events` passes 5/5 + 1/1 (`depot-core-tests.log`), including `frostbolt_kits_play_the_frostbolt_precast_cast_and_impact_sound_kits`, once SoundKit/SoundKitEntry are staged in `godot/depot-test-assets.txt`.

## Polymorph (2026-09-29)

The server (game-server branch `polymorph`, see its auras wiki "Crowd control") turns a Polymorph 118 target into the Polymorphed Sheep by changing its replicated `ModelDisplay` (TRANSFORM 56 → template 16372, display 856 or 857) and restores it when the aura ends. The client needs no Polymorph-specific code:

- `world.rs` `sync_unit_visual` rebuilds only the visual under the same unit node, so position, facing, selection and nameplate stay; the sheep plays Stand 0 and Walk 4 from the replicated `CreatureMotion` (the server walks a confused creature within 2 yd).
- The target circle is resized when the target's pick box changes (`targeting.rs` `TargetCircle.shape`), so it fits the sheep and then the humanoid again.
- A creature model holds only the virtual items whose slot attachment it has (`assets/creature.rs` `held_items`): the sheep holds none of the spy's weapons (inferred from the retail client, which shows none), instead of logging missing-attachment errors.
- The poof is Polymorph's own impact kit: model 166650 on the target (and 166524, 1709417 on the caster), played from `SpellGo` like any kit. There is no separate transform effect in the data.
- Fixture API: `unit_display(id)` → `display_id`, `visual`, `animation`.

**Live fixture:** `godot/tests/polymorph_mob.gd` (env `GODOT_TEST_SERVER`, `POLY_ACCOUNT`, `POLY_CHARACTER`, `POLY_SHOTS`, optional `POLY_GRAB`). Private server UDP 5081 (game-server `7742201`), a fresh level-10 Human mage at `-8966.63 -194.0 80.0`, 12 yd south of Blackrock Spy 20279977 (humanoid, level 1-30 scaled). Sequence, all asserted: Tab to the spy; Frostbolt pulls it and it swings (Attack1H 17, the mage's CombatWound 9); Polymorph shows its cast bar, then display 856/857 appears within 2.5 yd of the spy, still selected with its ring; for 10 s the sheep plays only Stand/Walk and never swings; a second Frostbolt breaks it: display 36654 returns and it swings again. Exit 0 at game-engine `8f9a2730`.

Recordings in `data/diagnostics/polymorph-2026-09-29/`:
- `polymorph-1x.mp4`, `polymorph-halfspeed.mp4`, `contact-sheet.png`, `stills-movie/`: Movie Maker (`--write-movie --fixed-fps 30`). The client renders about 8 fps here, and Movie Maker steps 1/30 s of game time per frame while the server runs on wall-clock time, so server events come about 3.6x early in the video: the 10 s sheep phase lasts about 3.7 s and the 1.7 s cast bar only partly fills.
- `polymorph-realtime-1x.mp4`, `polymorph-realtime-halfspeed.mp4`, `contact-sheet-realtime.png`, `stills-realtime-grab/`: the same fixture with `POLY_GRAB`, one JPEG per 100 ms of wall-clock time; timing is true, motion is choppy (about 8 fps).
- `stills-realtime/`: an earlier run with the camera behind trees, which shows the full Polymorph cast bar.

**Sounds for every spell (2026-09-29, `data/diagnostics/spellcast-anim-2026-09-29c/sounds-all/`).** The fixture logs each spell's SoundKit, file, unit, source (`kit`, `missile`, `voice`) and start/stop times. Setup: game-server dd45440, shared-protocol d252965, fresh DB, UDP 5083. All three scenarios exit 0.

- **Paladin** (`paladin/`):
  - Flash of Light on self: the precast loop 349350 runs 100.184 → 101.760 (1.58 s; the cast is 1.5 s). At `SpellGo` 101.760 the cast sounds 349352 and 349351 and the impact sounds 349357 and 349355 play.
  - Judgment on the dummy: cast 218258 and 349488 at 111.308. The missile's travel sound 53649 starts at release 111.510 and is logged stopped at 111.744. The impact sounds 218257, 221597 and 224414 play at the landing, 111.718. The stop is logged one frame late: the player is freed with the missile node, and the next frame records it.
  - Hammer of Justice: cast 53854, impact 221582, and the stun aura's loop 349372 on the dummy from 113.518 to the aura's end at 118.553.
- **Mage** (`mage/`):
  - Frostbolt: precast loop 85500 from 53.234 until `SpellGo` 54.879, cast 85502, impact 85503 at 55.334 (flight 0.229 s).
  - Frost Nova: cast 350096. On each of the 8 rooted dummies, loop 350097 and 350098 run 56.414 → 61.394, then the aura-end sound 85938 plays at 61.394.
- **Warrior** (`warrior/`, Worn Shortsword and shield, 2 yd from the dummy):
  - Slam: 57845 on the warrior, 60935 on the dummy.
  - Battle Shout: 114049 and the warrior's voice 58088 (source `voice`).
- **First-use run** (`paladin-first-use/`): the precast lasted 0.58 s on the client clock. The first use of each model and sound file extracts it from local CASC on the main thread, and that frame froze.
- **Earlier "dummies missing" runs (19:05-19:45)** were a protocol split.
  - shared-protocol d252965 (18:47) registered a new replicated component, `UnitThreatList`. Clients built after it ran against a server built at 18:2x with 2cf99a1.
  - The connection passed lightyear's check, which reported only "message protocol" mismatches when it failed earlier. The server logged "Granted immediate visibility for 81 nearby entities", but the client kept 1-8 units.
  - The same client passed against servers 9b4d1bd and c542b6d once both sides were rebuilt on d252965.
  - Rebuilding server c542b6d on 2cf99a1 reproduced the failure against a d252965 client: 2 units, no protocol error.
  - The server ticked 20/s in the bad runs, so machine load was not the cause.
  - An unmatched component registry is not caught at connect.

## Gaps

- The action layer is full-body when standing and upper-body when moving (SpineLow subtree). `AnimKitSegment` conditions, per-segment bone sets and priorities, and `AnimKit` blend times are not applied.
- Other effect types are not played: camera shakes, procedural effects, shadowy/outline/dissolve effects.
- Aura (7/8) kits, area and destination kits, and positioner offsets for attachment -1 are not played.
- `ChrSpecializationIndex` is always treated as no spec, and remote players' spec is unknown.
- Missiles fly straight: `SpellMissileMotion` is not applied. `SpellVisualMissile` `CastOffset`/`ImpactOffset`/`Flags` and `SpellVisual.Flags` are not applied (Frostbolt's offsets are 0).
- The missile starts at the missile attachment, not at the release event's bone and position. wowdev notes `$CSL/R/T are also used in CGUnit_C::ComputeDefaultMissileFirePos`, which is undocumented.
- Other events (`$SHK` camera shake, `$FSD` footfall, `$AH*`/`$BRT`/`$FD*` voice events) are parsed but not played. Type-10 unit sound values outside 34-40 are not played (mapping unknown).
- The first use of a spell extracts its models and sounds from CASC on the main thread, a one-time hitch per asset.
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
