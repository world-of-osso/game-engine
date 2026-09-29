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
4. **Missiles.** The missile is the visual's first `SpellVisualMissile`, travelling at `SpellMisc.Speed`. For Frostbolt that is model 1598570 at 35 yd/s, from attachment 56 to 34.
5. **Fallbacks.** `read_animation_fallbacks` loads `AnimationData.Fallback`. `WorldUnits` owns the table and resolves every combat, spell and stance clip a model lacks through it (e.g. 818 → 57 Special1H).

The catalog is cached as bincode at `data/cache/spell_visuals-12.1.0.69933.bin` (`db2_cache`).

## Runtime (`godot/rust/src/spell_effects.rs`)

- **Precast and channel.** A replicated `CastState` starts PrecastStart kits (Normal) or ChannelStart kits (Channel). They are held with their looping unit clip until the cast disappears or its `SpellGo` arrives.
- **`SpellGo` (new protocol message, TrinityCore `SMSG_SPELL_GO`).** It starts the Cast kits, then either launches the missile or starts the Impact kits. A missile's impact kits start on arrival.
  - The server broadcasts it from `execute_cast` to every connection in the caster's `NetworkVisibility`, or to every authenticated client for units without interest, so observers see other players' casts.
- **Kit model placement.** A model hangs from the unit model's `Skeleton3D/AttachmentBone{id}/Attachment{id}`; `-1` means the unit's origin. It plays its start clip once (the kit's, else Stand), Hold (158) while a held kit lasts, then Decay (159) at the end, and is freed after its particles' longest life. These defaults are inferred from how spell models author their emission. Battle Shout's buff 6194303 enables emitters at 133 ms of Stand, holds steady in Hold, and ramps down in Decay.
- **Virtual attachment 56.** VirtualSpellDirected is absent from character models. It is taken as the midpoint of SpellLeftHand 21 and SpellRightHand 22 (inferred). Any other missing attachment is reported and uses the origin.
- **Keyframed emission.** Spell effect emitters author their bursts as keyframed `emissionRate`/`emissionSpeed`/`enabledIn` tracks; the static first key is usually 0. `EmitterSim::set_animation` evaluates them at the placed model's playing sequence time, and pools are sized for the track's peak rate. Doodads get the same behavior.

## Proof (2026-09-29)

**Behavioral tests:**

- `godot/core/tests/spell_visual.rs`: Slam 1H/2H visuals, clips and impact model; Battle Shout roar, base and buff; Victory Rush by weapon; Frostbolt precast, hands, release and missile.
- `godot/core/tests/m2_particles.rs::keyframed_emission_follows_the_playing_sequence_time`.
- `godot/rust/src/animation/action_tests.rs` and `world_combat.rs` tests.
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

## Gaps

- The action layer is full-body when standing and upper-body when moving (SpineLow subtree). `AnimKitSegment` conditions, per-segment bone sets and priorities, and `AnimKit` blend times are not applied.
- Other effect types are not played: sound kits, camera shakes, procedural effects, shadowy/outline/dissolve effects.
- Aura (7/8) kits, area and destination kits, and positioner offsets for attachment -1 are not played.
- `ChrSpecializationIndex` is always treated as no spec, and remote players' spec is unknown.
- Missiles fly straight: `SpellMissileMotion` is not applied.
- Timed casts show no precast kits for observers until `CastState` replicates. Creature casts get kits only through `SpellGo`/`CastState`, like players.

## Sources

- `godot/core/src/spell_visual.rs`, `godot/rust/src/spell_effects.rs`, `godot/rust/src/world_combat.rs`, `godot/rust/src/animation/action.rs`
- WoWDBDefs (`~/Repos/wowless/vendor/dbdefs/definitions`) — layouts
- TrinityCore `ConditionMgr.cpp` `IsPlayerMeetingCondition`, `DBCEnums.h` `PlayerConditionFlags`
- WMVx `animation-names.csv` — animation ids

## See Also

- [[animation]] — the action layer, melee clips and stance
- [[spellbook-action-bar]] — casting UI and messages
- [[m2-format]] — particle emitter tracks
- [[db2-format]] — WDC5 export
