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
| CreatureSoundData | 1344466 | E5EE765B |
| WeaponSwingSounds2 | 1267068 | 8CC18B68 |
| WeaponImpactSounds | 1267648 | A77CBD9D |

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
- **Server timing.** Since game-server `fa5e689`, missile spells land at TrinityCore's delay (`Spell::HandleDelayed`; game-server `crates/server/src/spell_cast/missiles.rs`). `SpellGo` goes out at launch. Damage, auras (Chilled), threat, procs and the combat log follow each unit's hit delay: `LaunchDelay + max(max(dist, 5) / Speed, MinDuration)` (`Spell.cpp:2486-2518`). A destination spell uses one missile over the caster's exact distance (`CalculateDelayMomentForDst`, `:880-896`). Frostbolt at 20 yd lands 0.57 s after `SpellGo`. Speed-0 spells still land in the launch tick, and the delay is fixed at launch, as in TC. **Residual (unfixed):** the server's clock starts at the cast, but the client releases the missile at the M2 cast event (`$CSL`/`$CST`, 200 ms into the HD cast clip). So the damage number still shows about 200 ms before the visual impact, as in TrinityCore. Before `fa5e689` it showed with `SpellGo`, about 0.4 s early.
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
  - **No synthetic outcomes.** The Godot client plays no synthetic PCM for casts, impacts, heals, misses or interrupts. Spell results sound through their kits. An interrupt sounds through the interrupting spell's kits: Pummel (6552) visual 47968's impact kit 59620 plays SoundKit 53711 on its target. Melee outcomes are below.
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

## Melee sounds

Melee swings play their Retail swoosh, impact, exertion and wound sounds. `godot/core/src/spell_visual_melee.rs` resolves them and `godot/rust/src/spell_melee.rs` plays them through `SpellSounds` (sources `swing`, `impact`, `voice`).

**Timing.** The attacker's attack clip times the sounds with its M2 events. wowdev.wiki/M2 "Events" gives `$CSS` as "PlayWeaponSwooshSound ... sound played depends on CGUnit_C::GetWeaponSwingType" and `$CAH` as "CGUnit_C::HandleCombatAnimEvent".
- A melee `CombatEvent` is held as its attacker's pending swing, and voices the attacker's exertion. `$CSS` plays the swoosh, and `$CAH` lands the swing on the victim.
- A hit or crit plays the weapon's impact and the victim's injury voice. A parry plays the impact on the parrying weapon. A miss or dodge is the miss whoosh alone.
- The events: HumanMale HD Attack1H (17) `$CSS` 300 ms, `$CAH` 400 ms; kobold2 Attack1H 233/366 ms. Attack clips of both models carry them.
- A hit reaction no longer cuts the victim's own swing short (`ActionPriority::Reaction` below swings). The server lands both sides' swings in the same tick, so before this the victim's wound replaced its own attack clip before `$CSS`.

**Sources.** No retail source gives the rules beyond the table layouts. The 1.12.1 client's are reverse-engineered by benilla (github.com/samwhosung/benilla `4772489a`, `crates/benilla-app/src/sound/combat.rs`, cited by its disassembly addresses); the client applies them to the retail tables and marks where retail's tables outgrow 1.12.

**Resolution.**
- Swoosh (`0x624ca0`): a miss or dodge swings `(DONOTRENAME)Combat Miss 1H/2H`, SoundKit 7080/7081 by whether the weapon subclass is two-handed (Axe/Mace/Sword 2H, Polearm, Staff, Spear). Retail keeps both kits, now `fx_misswhoosh_revamp_*` (1455928-1455936). A connecting swing (hit, crit, parry) swooshes `ItemDisplayInfo.OverrideSwooshSoundKitID` of the main hand (10 displays set it), else `WeaponSwingSounds2` of (`ItemSubClass.WeaponSwingSize` as `SwingType`, crit). wowdev.wiki/DB/WeaponSwingSounds2: SwingType "match with ItemSubClassRec::m_WeaponSwingSize". Bare hands swing Light (`0x623892`); a held non-weapon nothing.
  - **Dagger and Fishing Pole (size 8): unknown.** 1.12 plays nothing past its three types (`0x457f63`); retail's `SwingType` enum is 0-5 (Light, Medium, Heavy, Agile, Pierce, Large Monster: WoWDBDefs `WeaponSwingType`). Size 8 and types 3-5 both arrive in 7.3.5 (wago.tools history; 3.4.3 has Dagger 0). Nothing maps 8, so a dagger swooshes only when it misses. The Agile or Pierce kits would be a guess.
- Impact (`0x6247d0`): the `WeaponImpactSounds` row of the attacker's (`WeaponSubClassID`, `ParrySoundType`, `ImpactSource`), its `ImpactSoundID` or `CritImpactSoundID` array at the victim's slot.
  - Slots (benilla `weapon_impact.rs:15-25`, and the files of every row): 0 flesh, 1 chain, 2 plate, 3 metal shield, 4 wood shield, 5 metal parry, 6 wood parry, 7 wood, 8 stone, 9 ethereal. Slot 10 (7.3.0) holds `*_hit_leatherarmor_*` files; no known rule reaches it.
  - `ParrySoundType` is the weapon's `Material.Flags & 1` (`0x457e80`; WoWDBDefs `MaterialFlags` 0x1 metal, 0x2 plate, 0x4 chain). Local `Material` (FDID 1294217, layout BE3E0E4C): 1 Metal 1, 2 Wood 0, 3 Liquid 0, 4 Jewelry 1, 5 Chain 5, 6 Plate 3, 7 Cloth 0, 8 Leather 0. Leather and cloth weapons are wood.
  - A player victim presents its chest item's `Material.Flags` (`0x62fb70`): plate slot 2, else chain slot 1, else flesh (leather, cloth). 1.12 reads only its own player's inventory; retail replicates every player's visible items, so every player's chest counts here.
  - A creature victim's `CreatureSoundData.CreatureImpactType` maps through `s_creatureIpactSounds` = {FLESH 0, STONE 8, WOOD 7, ETHEREAL 9} (wowdev.wiki/DB/CreatureSoundData). 1.12 refuses types from 4 (`0x6238f0`), which then land on flesh. **Retail types 4-6: unknown** (137 + 2 + 3 rows; 4 is mechanical models: mechagnomes, golems, Lightforged mech suits; 5 gnomecopter, clickable box; 6 a croc mount, a soulbinder). They land on flesh.
  - A parry strikes the parrying weapon, not crit-tiered: slot 5 metal, 6 wood. Bare hands are not metal.
  - `ImpactSource` (inferred from the files): 1 rows hold the player sets, 0 rows the `*_npc_*` and pre-revamp sets. Players swing 1, creatures 0. A subclass without the exact row takes its closest row: same source first, then same parry material.
  - **`Pierce*` columns: unknown.** They arrive in 7.3.0 with `ImpactSource`; 12 of 45 rows set them, nearly all equal to the plain columns (rows 2, 3, 10, 213, 226 differ in a slot or two). Not played.
- Bare hands strike with the display's `CreatureDisplayInfo.UnarmedWeaponType` subclass (11 Bear Claws, 12 Cat Claws). -1 (120,471 displays) is Fist Weapon (13), 1.12's unarmed row.
- `Item.Sound_override_subclassID`, when set, replaces the weapon's subclass.
- Vocals roll a chance (`0x623520`: `MulHi32(101, rand)` in 0..=100 passes at most the class threshold, so P = (t + 1) / 101; benilla `kit.rs:150-174`):
  - Exertion: the attacker's `SoundExertionID` when the swing arrives, unless it missed (`0x62476a`: victimState 0), 70 for a creature and 35 for a player. A crit always plays `SoundExertionCriticalID` (no fallback: Human 49 sets none).
  - Injury: the victim's `SoundInjuryID` when a swing lands a hit, 60 for a creature and 30 for a player; a crit always, `SoundInjuryCriticalID` (or `SoundInjuryID` when 0: Human 49's injury kit 2942 holds the `woundcrit` files).
  - `SoundDeathID` when an NPC's death clip starts (wowdev `$DTH`: m_soundDeathID "is just always triggered as soon as the death animation plays").

**Worked example (12.1.0.69933, core test `godot/core/tests/melee_sounds.rs`).**

| Case | Chain | SoundKit | Files |
|---|---|---|---|
| Warrior swoosh | Worn Shortsword `Item` 25 → Sword (7) → WeaponSwingSize 1 → SwingType Medium | 235 (236 crit) | 1302596-1302605 `fx_whoosh_medium_revamp_01-10` |
| Warrior hit on a Kobold Vermin | row 8 (sub 7, metal, player) × CreatureSoundData 5042 impact type 0 → index 0 | 53248 (53249 crit) | 1247339-1247348 `1h_sword_hit_flesh_01-10` |
| Warrior parried by the kobold's staff | row 8 index 6 | 53263 | `1h_sword_hit_wood_parry_01` |
| Kobold swoosh | `Item` 5276 → Staff (10) → size 2 → Heavy | 237 (238 crit) | 567936, 567943, 567941 `mwooshlarge1-3` |
| Kobold hit on the warrior | row 10 (sub 10, wood, creature) × Human 49 impact 0 | 61562 (61563 crit) | 1394207-1394212 `staff_wood_npc_hit_flesh_01-06` |
| Kobold parried by the sword | row 10 index 5 | 61557 | `staff_wood_parry_09` |
| Kobold wound / crit / death | display 10913 → model 8379 → CreatureSoundData 5042 | 53725 / 53726 / 53727 | 1255506-1255513 `mon_kobold_v2_wound_01-08` |
| Warrior wound / death | CreatureSoundData 49 | 2942 / 2944 | 16 files incl. `humanmalemainwoundcrita` 542371 |
| Blackrock Worg (49871) swoosh, hit, parried | display 40147, no item → Light; Fist (13) row 13 | 233; 1014; 1019 | `fx_whoosh_small_revamp_*`; `unarmedattacksmalla`, `unarmedparrymetala` |
| Miss or dodge | sword (1H) / staff (2H) | 7080 / 7081 | `fx_misswhoosh_revamp_*` |
| Worn Dagger (`Item` 2092) connects / misses | Dagger (15) → size 8 | none / 7080 | |
| Kobold hit on a plate / chain / leather chest | `Item` 3242 / 285 / 60 → row 10 slot 2 / 1 / 0 | 61561 / 61567 / 61562 | |
| Leather dagger (`Item` 111415) by a creature | Material 8 → wood → row 22 | 1151 | |
| Hit on display 19162 (CreatureSoundData 2431, impact type 4) | → flesh, row 8 slot 0 | 53248 | |
| Kobold / Human exertion | CreatureSoundData 5042 / 49 | 53723 (53724 crit) / 2941 (none) | |
| Worg wound / death | CreatureSoundData 2482 | 11908 / 11910 | |

The live Northshire content has no Kobold Vermin (retail phase: Blackrock Worg, Invader, Spy, Goblin Assassin), so the live fixture fights a Blackrock Worg.

**Live proof (2026-09-30).** `godot/tests/melee_sounds_live.gd`, private game-server `c47217b` on UDP 5097 (fresh redb, shared-protocol `aa848af`), Human warrior `Fbmelee` with the Worn Shortsword (granted: a fresh character spawns with no equipment) next to a Blackrock Worg; exit 0, log `data/diagnostics/melee-sounds-2026-09-30/fixture.log`.
- Warrior swings swoosh 235 (`fx_whoosh_medium_revamp_*`, FDIDs 1302597-1302605) and land 53248 (1247344, 1247347) with the worg's wound 11908 (559528-559532); gaps 89-92 ms against Attack1H's 100 ms.
- Worg swings swoosh 235 and land 1014 (567919-567928) with the warrior's wound 2942 (951376-951390, 542369); gaps 54-154 ms (longest frame 0.48 s).
- The warrior's Avoided swing at 18.643 swooshed (1302597 at 19.046) with no impact.
- An earlier run on the same revision logged the worg's death 11910 (559527); that run had no avoided swing in 47 events.
- A first-use sound starts when its file finishes loading (async asset loader), so its logged start can trail the event: one swoosh started 72 ms after its own impact.

**Not modelled:** blocks (the server sends a block as MeleeDamage, so the shield slots are unreachable), crits (the server sends a crit as MeleeDamage, so `CriticalHit` is handled but never arrives), 1.12's natural-weapon `$AH0-3`/`CustomAttack` impacts, its bus caps, the half-volume swoosh under `HITINFO_MISS`, and its suppression of creature vocals under a server-pushed sound.

## Asset loading (`godot/rust/src/spell_assets.rs`, `godot/core/src/asset_loader.rs`)

Kit models, missile models and kit sound files load on two `spell-assets` worker threads. Nothing on the main thread waits for them.

- **Worker:** local-CASC extraction (`cache_model_files`, `cache_model_textures`, sound `ensure_cached`), the M2/skin/skel/`.anim` parse (`read_model_file`, which makes no engine calls), and BLP decode (`blp::decode_gpu`) of the model's batch and particle textures. A sound's Ogg bytes are read and checked for `OggS`.
- **Main thread:** `SpellAssets::update` collects results each frame. Arrived models get their textures uploaded into the shared texture cache (`material::insert_shared_texture`) and their particles set up, within a 4 ms budget per frame (at least one model per frame). Godot objects are created as before: `build_model`, particle placement, and `AudioStreamOggVorbis::load_from_buffer` at play time.
- **Priority:** a load a kit needs now goes ahead of prefetches. A prefetch still queued moves up when a cast needs it (`Priority::Now`/`Later`).
- **Failures:** each failed asset is logged once when it arrives (`godot_error!`). Every kit that then uses it reports `Spell {id} kit {id}: …`.
- **Late assets (this client's rule):** a kit whose model or sound file is still loading starts it on arrival, at the point of its timeline the kit has reached, as if it had started on time. A one-shot model joins its start clip, end clip or particle tail (`phase_at`). A held model joins its Hold loop. A sound starts that far into the file, and a loop wraps (`late_offset`). A one-shot already over is not shown and is logged ("arrived … late, after it would have ended"). A missile flies on schedule and shows its model once loaded, and its impact keeps its time.
  - Retail's rule is undocumented. The wow_client reimplementation keeps a late model's sequence start at request time (`src/gx/m2.c:2966`: `instance->sequence_started = global_time;`, set before the model loads), which is the same rule. WebWowViewerCpp instead starts the animation at arrival (`m2Object.cpp:1146` returns early while unloaded).
- **Prefetch:** once the catalog is loaded and the local player is replicated, every known spell's visual (for the player's race, class and weapon) has its PrecastStart, ChannelStart, Cast, Impact, AuraStart and AuraEnd kit models, sound files and missile queued as prefetches. No retail source for spell-asset prefetching was found; retail's `preloadPlayerModels` CVar covers racial models only (warcraft.wiki.gg Console_variables). A weapon that replicates after the prefetch can resolve another visual, which then loads when it is first cast.
- **Automation:** `spell_visuals_state()` reports `assets_pending`, each sound's `late` (seconds after its kit asked for it), and `frame_ms`, the main-thread time spell visuals took in the last frame (`SpellGo` handling plus the per-frame update).

### CASC startup initialization (source audit, 2026-09-30)

`c0165d28` moves process-wide resolver initialization ahead of asset-using client startup. `GameClient::init` starts a named `casc-startup` thread; `AssetStartup::start` calls the shared local resolver's public `initialize()` there. This initializes resolver state, not the spell catalog or every spell asset; their workers and prefetch remain separate.

`ready` configures display/focus without waiting for CASC. Each `process` polls the worker: while pending, it finishes physical-input bookkeeping and returns before normal client steps. Input handlers also return while startup is pending. `AssetStartup::poll` checks `is_finished` before joining and delivers completion once; the main thread does not join a running initializer.

Only successful completion clears the pending gate and resumes sound initialization, startup intent (including attaching the asset-backed login UI and opening the requested screen), then UI-scale synchronization. Worker-spawn errors, initialization errors and worker panics reach the startup error path, which logs `Cannot initialize client: …` and requests exit code 1. There is no alternate startup path on failure.

**Evidence boundary:** this is a source audit of `asset_startup.rs`, `lib.rs` and `startup.rs`, not a live startup-latency measurement. Worker tests in `asset_startup.rs` describe pending/non-caller-thread execution, one-shot completion, error and panic behavior; this docs audit did not run them. The historical first-use measurements below predate this explicit startup gate and do not prove its latency, rendered readiness, or elimination of later terrain/creature/equipment extraction stalls.

**Live A/B (2026-09-30, `data/diagnostics/firstload-2026-09-30/ab/`).** Back-to-back first-use paladin runs under the same machine load (load average 11-33) used game-server `c47217b`, shared-protocol `aa848af`, a fresh redb, UDP 5094 and `SPELL_WORLD_TIMEOUT_S=0`. B is the engine at `e0000dd3`. C is the same build with the startup worker doing nothing (`initialize()` not called). A is master `73042fbc`, which already had both.
- **Where the CASC init lands.** In C (runs C1, C2) the first extraction, character-select texture 948080, initialized CASC inside the main-thread "Account" step at world entry. In A and B (A1, A2, B1-B4) it ran on the `casc-startup` worker before any client step. Startup frames kept a 0.6-3.8 ms median with a 7-89 ms maximum while it ran.
- **First Flash of Light.** It passed in B1-B4, C1, C2 and A2. Spell visuals took at most 5.1-18.7 ms of any frame, and the precast lasted 1.497-1.620 s for 1.45-1.5 s. A1 failed: a 1018 ms "World objects" frame inside the precast cut it to 0.750 s on the client clock, and that frame spent 52.7 ms on spell visuals.
- asset-resolver `833a70f`: a failure to warm key-aware archive access is logged, and `initialize()` still succeeds. Only encrypted fallback reads need that access, so the client's startup gate no longer exits over it.

**Historical measurements (2026-09-30, first use; before the explicit startup gate; debug build; machine load average 12-35 from other agents' clients and builds).**
- **Before** (base `bba70f53`, `data/diagnostics/firstload-2026-09-30/before-*.log`):
  - The first CASC extraction initializes the resolver on the main thread: TACT keys plus the 1,931,507-entry resolution cache took 1548 ms and 1654 ms. It hit at world entry (an NPC aura's sound 569423 through `sync_casts`). In the earlier `paladin-first-use/` run it hit at the first Flash of Light.
  - Per file, extraction on the main thread took 0.6-40 ms for sounds, 1.4-33 ms for model+skin and 2.5-34 ms per model's textures. Parsing took 0.7-1.6 ms per model (`read_model`). Godot creation took 0.6-17 ms (`build_model`, including BLP decode and listfile lookups), 1.6-18 ms for particle placement, and 0.5-0.8 ms per Ogg stream.
  - Whole calls: `SpellGo` 45-59 ms, per-frame advance 27-86 ms, HoJ `sync_casts` 36 ms. Under load the "Spell visuals" frame step took 117 ms and 283 ms during the first Flash of Light.
- **After** (`data/diagnostics/firstload-2026-09-30/after-*.log`): the prefetch extracted 175 files on the workers after world entry. The first Flash of Light, Judgment and Hammer of Justice sounds all played with `late=0.000`. Across the loaded runs the largest spell-visual frame share during Flash of Light was 30.4 ms, and the precast lasted 1.493-1.573 s on the client clock for a 1.500 s cast. The final run after merging master (`after-final-merged.log`, load average about 20) showed a 45.0 ms pre-press median, a worst frame of 54.8 ms and at most 4.3 ms of spell visuals per frame. Its precast lasted 1.512 s for the 1.450 s left at first sight, and the fixture exited with `SPELLCAST_ANIM_DONE`. The CASC init ran on a worker (its first extraction was a prefetched kit model).
- **Other main-thread stalls** (not spell visuals; reported here, fixed in [world-entry stalls](../investigations/world-entry-stalls.md)). Timings are from the 8 A/B runs, as frame steps of 50 ms or more:
  - **"World objects" step** (ADT doodad/WMO spawning, budget `WORLD_OBJECT_BUDGET` 8 ms): 8-17 frames per run over 50 ms, a median of 73-158 ms and a maximum of 281-1018 ms. Earlier runs at load average 30 had 50-1074 ms, and `world_objects.pending` never reached 0 within 600-900 s in base or branch.
  - **"Account" step at world entry:** one frame of 17.3-86.6 s (A1 18.3 s, A2 30.1 s, B1 17.3 s, B2 24.3 s, C1 23.2 s, C2 86.6 s, B3 81.6 s, B4 27.7 s). This is the longest frame of every run. Other Account frames reached 0.1-1.5 s. It appears with and without the CASC gate.

**First-use fixture.** `scripts/agent/first-use-data.py <worktree> isolate` replaces the worktree's `data/models`, `data/textures` and `data/sounds` links with local directories of per-file links to canonical data. `--models`/`--textures` FDIDs are left out, and `sounds/spells/` is empty. `reset` deletes what a run extracted. The Flash of Light kit assets are models 2467327, 2470733, 1237495-1237497 and textures 2062922, 2447763, 1114588, 942427, 1114590-1114592. `spellcast_anim.gd` with `SPELL_SCENARIO=paladin` then requires the following from the press through the loops stopping:
- no frame spends more than `SPELL_FRAME_SLACK_MS` (50) on spell visuals;
- when the world had settled, no frame is longer than the 120-frame pre-press median plus the slack;
- the precast lasts the replicated cast time within 0.1 s plus two median frames.

Run with `SPELL_WORLD_TIMEOUT_S=0` when doodads cannot finish streaming (only the spell share is then required). Passed on 2026-09-30: private game-server `b85943c` on UDP 5094, account `fb_firstload`/Fbfirstpal (level-10 Human paladin at `-8969.78 -154.5 81.6`).

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
- The damage number showing 13 frames before the impact was the server's same-tick damage at the time of this capture (fixed in game-server `fa5e689`; about 200 ms early remains, see Server timing). Not re-captured.
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
- Fixture API: `unit_display(id)` → `display_id`, `visual`, `animation`, and for an NPC `animation_rate`.
- Follow-up fixes (2026-09-29, branch `polyfix`):
  - Nameplate hostility: a non-friendly unit with the local player on its replicated `UnitThreatList` gets a (1, 0, 0) health fill, as `considerSelectionInCombatAsHostile` (Blizzard_NamePlateFrameOptions.lua:30, :53; CompactUnitFrame.lua:674). The neutral spy's plate turns red on the pull and stays red while it is a sheep (it keeps the mage on its threat list with no target). The list also counts as "fighting the player" for plate visibility (`in_combat_with_player`). The plate name and the TargetFrame reputation colour are not coloured.
  - Level scaling: the TargetFrame shows a tuned creature at its level for the viewer (`level_for_viewer`, as `UnitEffectiveLevel`; TargetFrame.lua:266-282 shows the skull only when the effective level is not positive) and its replicated native health pool times `GetHealthMultiplierForTarget` (ExpectedStat creature health at that level over the native level, `creature_health_scaling_data`, default-expansion rows only). The Blackrock Spy (ContentTuning 73, levels 1-30, native 30) showed "??" and 4379 to the level-10 mage; it now shows 10 and 377 / 377.
  - Clip pacing: an NPC's walk and run clips play at its replicated `MovementSpeed` over the M2 sequence `movespeed` (`movement_animation_data::locomotion_playback_rate`). The Chilled spy runs at 3 yd/s on a Run authored at 7 yd/s (949470.skel), rate 0.43; the sheep walks 2.5 yd/s on a Walk authored at 1.11 yd/s (1377131.m2), rate 2.25. Before, every clip played at 1: the chilled spy's legs moved 2.3× too fast, the sheep's 0.44× (sliding). The rate formula is inferred from the sequence `movespeed` field; no client source was checked.

**Live fixture:** `godot/tests/polymorph_mob.gd` (env `GODOT_TEST_SERVER`, `POLY_ACCOUNT`, `POLY_CHARACTER`, `POLY_SHOTS`, optional `POLY_GRAB`). Private server UDP 5081 (game-server `7742201`), a fresh level-10 Human mage at `-8966.63 -194.0 80.0`, 12 yd south of Blackrock Spy 20279977 (humanoid, level 1-30 scaled). Sequence, all asserted: Tab to the spy; Frostbolt pulls it and it swings (Attack1H 17, the mage's CombatWound 9); Polymorph shows its cast bar, then display 856/857 appears within 2.5 yd of the spy, still selected with its ring; for 10 s the sheep plays only Stand/Walk and never swings; a second Frostbolt breaks it: display 36654 returns and it swings again. Exit 0 at game-engine `8f9a2730`.

Recordings in `data/diagnostics/polymorph-2026-09-29/`:
- `polymorph-1x.mp4`, `polymorph-halfspeed.mp4`, `contact-sheet.png`, `stills-movie/`: Movie Maker (`--write-movie --fixed-fps 30`). The client renders about 8 fps here, and Movie Maker steps 1/30 s of game time per frame while the server runs on wall-clock time, so server events come about 3.6x early in the video: the 10 s sheep phase lasts about 3.7 s and the 1.7 s cast bar only partly fills.
- `polymorph-realtime-1x.mp4`, `polymorph-realtime-halfspeed.mp4`, `contact-sheet-realtime.png`, `stills-realtime-grab/`: the same fixture with `POLY_GRAB`, one JPEG per 100 ms of wall-clock time; timing is true, motion is choppy (about 8 fps).
- `stills-realtime/`: an earlier run with the camera behind trees, which shows the full Polymorph cast bar.

**Recording with audio (2026-10-01, `data/diagnostics/polymorph-2026-10-01/`).** `POLY_GRAB` now grabs the first frame of every 1/30 s wall-clock slot (JPEG encoding on `WorkerThreadPool`), records the Master bus with an `AudioEffectRecord` from the first frame on (`audio.wav`), and prints fixture events and spell sounds with `t=` on that timeline. `scripts/agent/grab-video.py` muxes it into a 30 fps H.264 + AAC MP4 and writes timing, per-event audio RMS and a contact sheet. The client needs a mixing audio driver: run Godot with `--audio-driver PulseAudio` and `PULSE_SINK` set to a `pactl load-module module-null-sink`, so nothing plays on the desktop. On game-server 40241f5, a level-10 mage with no chosen spec gets Arcane 62, which replaces Frostbolt with Arcane Blast. The fixture now sends `set_specialization(64)` (Frost) after entering the world. Findings:
- The grab keeps up with rendering, but in-world the client rendered only 5-7 fps in the headless cage while the machine had a load average of about 20. The main thread was CPU-bound and the network/grab threads were nearly idle. Building the `game-engine-godot` crate at opt-level 2 (an experiment, not committed) raised the rate from 5.2 to 6.7 fps.
- Main-thread stalls of over 10 s during streaming time out netcode, and the client re-enters the world with a new player entity.
- The spy chases along the ridge and then fights, turns into a sheep and wanders several yards up in the air above the mage. The 2026-09-30 video showed it on the ground.
- The breaking Frostbolt restores the spy's display (t=33.28) about 0.5 s before the client's impact sound (t=33.77).

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
- CASC initializes on the `casc-startup` worker before client steps run (see [CASC startup initialization](#casc-startup-initialization-source-audit-2026-09-30) and its live A/B). Terrain, creature and equipment extraction and parsing now run on workers too ([world-entry stalls](../investigations/world-entry-stalls.md)).
- Composite textures (a second texture or overlays on a non-effect batch) are still read and composited on the main thread by `build_model`. Spell effect batches are plain or effect textures.
- Timed casts show no precast kits for observers until `CastState` replicates. Creature casts get kits only through `SpellGo`/`CastState`, like players.

## Sources

- `godot/rust/src/asset_startup.rs`, `godot/rust/src/lib.rs`, `godot/rust/src/startup.rs` — explicit CASC startup worker and readiness/failure gate (`c0165d28`, source audit only)
- `godot/core/src/spell_visual.rs`, `spell_visual_melee.rs`, `godot/rust/src/spell_effects.rs`, `spell_melee.rs`, `godot/rust/src/spell_assets.rs`, `godot/core/src/asset_loader.rs`, `godot/rust/src/world_combat.rs`, `godot/rust/src/animation/action.rs`
- wowdev.wiki DB/WeaponSwingSounds2, DB/WeaponImpactSounds, DB/CreatureSoundData (archived 2025) — swing type, parry material, `s_creatureIpactSounds`
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
