//! Retail melee swing, impact, wound and death sounds of a Human warrior with a Worn
//! Shortsword against a Northshire Kobold Vermin (12.1.0.69933 local-CASC exports).

use std::path::Path;
use std::sync::OnceLock;

use game_engine_core::spell_visual::{
    CasterContext, KitSound, MeleeHand, SpellVisualCatalog, SwingResult, UnitSound, VisualEvent,
    VoiceSource,
};

/// Worn Shortsword: `Item` 25 (Sword 7, Material 1 Metal), `ItemDisplayInfo` 1542.
const WARRIOR: MeleeHand = MeleeHand {
    item_id: Some(25),
    display_info_id: Some(1542),
    chest_item_id: None,
    shield_item_id: None,
    unit: VoiceSource::Player { race: 1, sex: 0 },
};
/// Kobold Vermin (creature 6): display 10913 (model 8379 → CreatureSoundData 5042) with
/// its equipped item 5276 (Staff 10, Material 2 Wood), `ItemDisplayInfo` 5010.
const KOBOLD: MeleeHand = MeleeHand {
    item_id: Some(5276),
    display_info_id: Some(5010),
    chest_item_id: None,
    shield_item_id: None,
    unit: VoiceSource::Creature { display_id: 10913 },
};
const HIT: SwingResult = SwingResult::Hit { critical: false };
const CRIT: SwingResult = SwingResult::Hit { critical: true };
/// The lowest and highest vocal chance rolls: 0 and 100 of 0..=100.
const ROLL_LOW: u32 = 0;
const ROLL_HIGH: u32 = u32::MAX;

fn catalog() -> &'static SpellVisualCatalog {
    static CATALOG: OnceLock<SpellVisualCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/db2/12.1.0.69933");
        SpellVisualCatalog::build(&dir).expect("spell visual CSVs load")
    })
}

/// (sound kit, its files' FDIDs).
fn kit(sound: Option<&KitSound>) -> Option<(u32, Vec<u32>)> {
    sound.map(|sound| {
        (
            sound.sound_kit_id,
            sound.files.iter().map(|file| file.fdid).collect(),
        )
    })
}

fn kit_id(sound: Option<&KitSound>) -> Option<u32> {
    sound.map(|sound| sound.sound_kit_id)
}

/// `WeaponSwingSounds2`: a one-hand sword (WeaponSwingSize 1, Medium) swooshes 235
/// (`fx_whoosh_medium_revamp_01..10`), 236 on a crit; the kobold's staff (size 2,
/// Heavy) 237 (`mwooshlarge1..3`), 238 on a crit.
#[test]
fn swings_swoosh_by_weapon_swing_size() {
    let catalog = catalog();
    assert_eq!(
        kit(catalog.swing_sound(WARRIOR, HIT)),
        Some((235, (1302596..=1302605).rev().collect()))
    );
    assert_eq!(kit_id(catalog.swing_sound(WARRIOR, CRIT)), Some(236));
    assert_eq!(
        kit_id(catalog.swing_sound(WARRIOR, SwingResult::Parry)),
        Some(235)
    );
    assert_eq!(
        kit(catalog.swing_sound(KOBOLD, HIT)),
        Some((237, vec![567936, 567943, 567941]))
    );
    assert_eq!(kit_id(catalog.swing_sound(KOBOLD, CRIT)), Some(238));
}

/// A miss or dodge swings the miss whoosh (benilla `0x624ca0`): the one-hand sword 7080,
/// the two-hand staff 7081, both `fx_misswhoosh_revamp_*`.
#[test]
fn misses_and_dodges_swing_the_miss_whoosh_by_handedness() {
    let catalog = catalog();
    for result in [SwingResult::Miss, SwingResult::Dodge] {
        let one_hand = catalog.swing_sound(WARRIOR, result).unwrap();
        assert_eq!(one_hand.sound_kit_id, 7080);
        assert!(
            one_hand
                .files
                .iter()
                .all(|file| (1455928..=1455936).contains(&file.fdid))
        );
        assert_eq!(kit_id(catalog.swing_sound(KOBOLD, result)), Some(7081));
    }
}

/// Worn Dagger (`Item` 2092, Dagger 15): WeaponSwingSize 8 is no `SwingType`, so a
/// connecting swing has no swoosh; its miss still whooshes. Bare hands swing Light (233,
/// `fx_whoosh_small_revamp_*`), whatever their `UnarmedWeaponType`.
#[test]
fn daggers_swoosh_only_when_they_miss_and_bare_hands_swing_light() {
    let catalog = catalog();
    let dagger = MeleeHand {
        item_id: Some(2092),
        display_info_id: None,
        ..KOBOLD
    };
    assert!(catalog.swing_sound(dagger, HIT).is_none());
    assert!(catalog.swing_sound(dagger, CRIT).is_none());
    assert_eq!(
        kit_id(catalog.swing_sound(dagger, SwingResult::Miss)),
        Some(7080)
    );
    let bare = MeleeHand {
        item_id: None,
        display_info_id: None,
        ..KOBOLD
    };
    assert_eq!(
        kit(catalog.swing_sound(bare, HIT)),
        Some((
            233,
            vec![
                1302929, 1302928, 1302927, 1302926, 1302925, 1302924, 1302923, 1302932, 1302931,
                1302930
            ]
        ))
    );
    assert_eq!(kit_id(catalog.swing_sound(bare, CRIT)), Some(234));
}

/// `WeaponImpactSounds`: the warrior's sword (subclass 7, metal, player source) is row 8;
/// the kobold's CreatureImpactType 0 is flesh, so a hit plays 53248
/// (`1h_sword_hit_flesh_01..10`) and a crit 53249. The kobold's staff (subclass 10,
/// wood, creature source) is row 10: a hit on the Human (impact type 0) plays 61562
/// (`staff_wood_npc_hit_flesh_01..06`).
#[test]
fn hits_play_the_attacker_weapon_on_the_victim_material() {
    let catalog = catalog();
    assert_eq!(
        kit(catalog.impact_sound(WARRIOR, KOBOLD, HIT)),
        Some((53248, (1247339..=1247348).collect()))
    );
    assert_eq!(
        kit_id(catalog.impact_sound(WARRIOR, KOBOLD, CRIT)),
        Some(53249)
    );
    assert_eq!(
        kit(catalog.impact_sound(KOBOLD, WARRIOR, HIT)),
        Some((61562, (1394207..=1394212).collect()))
    );
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, WARRIOR, CRIT)),
        Some(61563)
    );
}

/// A parry strikes the parrying weapon: the kobold's wooden staff takes the sword's
/// wood-parry 53263, the warrior's metal sword the staff's parry 61557; a miss or dodge
/// strikes nothing.
#[test]
fn parries_strike_the_parrying_weapon_and_misses_nothing() {
    let catalog = catalog();
    let parry = SwingResult::Parry;
    assert_eq!(
        kit_id(catalog.impact_sound(WARRIOR, KOBOLD, parry)),
        Some(53263)
    );
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, WARRIOR, parry)),
        Some(61557)
    );
    for (attacker, victim) in [(WARRIOR, KOBOLD), (KOBOLD, WARRIOR)] {
        assert!(
            catalog
                .impact_sound(attacker, victim, SwingResult::Miss)
                .is_none()
        );
        assert!(
            catalog
                .impact_sound(attacker, victim, SwingResult::Dodge)
                .is_none()
        );
    }
}

/// Bare hands swing the display's `UnarmedWeaponType`, -1 being Fist Weapon (13): a
/// creature's fist is row 13 (`unarmedattacksmalla`), a Cat Claws display (12) row 12
/// (`unarmed_impacts_clawed_cat_01`), the only Cat Claws row (display 892).
#[test]
fn bare_hands_swing_their_unarmed_weapon_type() {
    let catalog = catalog();
    let fist = MeleeHand {
        item_id: None,
        display_info_id: None,
        chest_item_id: None,
        shield_item_id: None,
        unit: VoiceSource::Creature { display_id: 10913 },
    };
    assert_eq!(kit_id(catalog.impact_sound(fist, WARRIOR, HIT)), Some(1014));
    let cat = MeleeHand {
        unit: VoiceSource::Creature { display_id: 892 },
        ..fist
    };
    assert_eq!(kit_id(catalog.impact_sound(cat, WARRIOR, HIT)), Some(57787));
}

/// CreatureSoundData wounds and deaths: the kobold's row 5042 wounds 53725
/// (`mon_kobold_v2_wound_01..08`), 53726 on a crit, and dies 53727; the Human male's row
/// 49 leaves its crit wound 0, so a crit plays its wound kit 2942, whose files include
/// `humanmalemainwoundcrita`; it dies 2944.
#[test]
fn victims_voice_wounds_and_deaths() {
    let catalog = catalog();
    let (kobold, human) = (KOBOLD.unit, WARRIOR.unit);
    assert_eq!(
        kit(catalog.wound_sound(kobold, false)),
        Some((53725, (1255506..=1255513).rev().collect()))
    );
    assert_eq!(kit_id(catalog.wound_sound(kobold, true)), Some(53726));
    assert_eq!(
        kit_id(catalog.unit_sound(kobold, UnitSound::Death)),
        Some(53727)
    );
    assert_eq!(kit_id(catalog.wound_sound(human, false)), Some(2942));
    let crit = catalog.wound_sound(human, true).unwrap();
    assert_eq!(crit.sound_kit_id, 2942);
    assert!(crit.files.iter().any(|file| file.fdid == 542371));
    assert_eq!(
        kit_id(catalog.unit_sound(human, UnitSound::Death)),
        Some(2944)
    );
}

/// An interrupt sounds through the interrupting spell's own kits: Pummel (6552) visual
/// 47968's impact kit 59620 plays SoundKit 53711 on its target.
#[test]
fn pummel_interrupts_with_its_impact_kit_sound() {
    let catalog = catalog();
    let warrior = CasterContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 10,
        spec_order_index: None,
        main_hand_subclass: Some(7),
    };
    let visual = catalog.visual_for_spell(6552, &warrior).unwrap();
    assert_eq!(visual, 47968);
    let impact: Vec<(u32, Vec<u32>)> = catalog
        .kits(visual, VisualEvent::Impact)
        .into_iter()
        .map(|kit| {
            (
                kit.kit_id,
                kit.sounds.iter().map(|sound| sound.sound_kit_id).collect(),
            )
        })
        .collect();
    assert_eq!(impact, vec![(59620, vec![53711])]);
}

/// The live Northshire's Blackrock Worg (creature 49871, display 40147 → CreatureSoundData
/// 2482, no item): its bare hands swoosh 235 and land 1014 (row 13) on the warrior, the
/// warrior's sword lands 53248 on it, and it wounds 11908 and dies 11910; a parry by the
/// warrior's metal sword plays 1019 (`unarmedparrymetala`). Its bare hands swing Light.
#[test]
fn blackrock_worg_fights_bare_handed() {
    let catalog = catalog();
    let worg = MeleeHand {
        item_id: None,
        display_info_id: None,
        chest_item_id: None,
        shield_item_id: None,
        unit: VoiceSource::Creature { display_id: 40147 },
    };
    assert_eq!(kit_id(catalog.swing_sound(worg, HIT)), Some(233));
    assert_eq!(kit_id(catalog.impact_sound(worg, WARRIOR, HIT)), Some(1014));
    assert_eq!(
        kit_id(catalog.impact_sound(worg, WARRIOR, SwingResult::Parry)),
        Some(1019)
    );
    assert_eq!(
        kit_id(catalog.impact_sound(WARRIOR, worg, HIT)),
        Some(53248)
    );
    assert_eq!(kit_id(catalog.wound_sound(worg.unit, false)), Some(11908));
    assert_eq!(
        kit_id(catalog.unit_sound(worg.unit, UnitSound::Death)),
        Some(11910)
    );
}

/// A weapon is metal by `Material.Flags & 1` (benilla `material.rs:30-36`): a leather
/// dagger (`Item` 111415, Material 8) strikes with the creature wood-dagger row 22
/// (1151), not the metal row 21 (63987).
#[test]
fn leather_weapons_strike_as_wood() {
    let catalog = catalog();
    let leather = MeleeHand {
        item_id: Some(111415),
        display_info_id: None,
        ..KOBOLD
    };
    assert_eq!(
        kit_id(catalog.impact_sound(leather, WARRIOR, HIT)),
        Some(1151)
    );
    let metal = MeleeHand {
        item_id: Some(2092),
        ..leather
    };
    assert_eq!(
        kit_id(catalog.impact_sound(metal, WARRIOR, HIT)),
        Some(63987)
    );
}

/// A player victim presents its chest's `Material.Flags` (benilla `0x62fb70`): plate
/// (`Item` 3242, Material 6) slot 2, chain (285, Material 5) slot 1, leather (60) and
/// cloth (56) flesh. The kobold's staff row 10: 61561, 61567, 61562.
#[test]
fn player_victims_present_their_chest_armour() {
    let catalog = catalog();
    let wearing = |chest| MeleeHand {
        chest_item_id: Some(chest),
        ..WARRIOR
    };
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, wearing(3242), HIT)),
        Some(61561)
    );
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, wearing(285), HIT)),
        Some(61567)
    );
    for cloth_or_leather in [60, 56] {
        assert_eq!(
            kit_id(catalog.impact_sound(KOBOLD, wearing(cloth_or_leather), HIT)),
            Some(61562)
        );
    }
    // A creature's chest item plays no part: the kobold is flesh by its impact type.
    let armoured_kobold = MeleeHand {
        chest_item_id: Some(3242),
        ..KOBOLD
    };
    assert_eq!(
        kit_id(catalog.impact_sound(WARRIOR, armoured_kobold, HIT)),
        Some(53248)
    );
}

/// `CreatureImpactType` 4 is past 1.12's four-entry table (benilla `0x6238f0`) and lands on
/// flesh: display 19162 (CreatureSoundData 2431, type 4) takes the sword's 53248.
#[test]
fn undocumented_creature_impact_types_land_on_flesh() {
    let catalog = catalog();
    let construct = MeleeHand {
        item_id: None,
        display_info_id: None,
        chest_item_id: None,
        shield_item_id: None,
        unit: VoiceSource::Creature { display_id: 19162 },
    };
    assert_eq!(
        kit_id(catalog.impact_sound(WARRIOR, construct, HIT)),
        Some(53248)
    );
}

/// Exertion (benilla `0x62476a`, `kit.rs:164-166`): the kobold (row 5042) voices 53723
/// on a hit, parry or dodge whose roll is within 70 of 0..=100, 53724 on any crit, and
/// nothing on a miss. The Human (row 49) passes within 35 and has no crit exertion.
#[test]
fn attackers_exert_by_chance_unless_they_miss() {
    let catalog = catalog();
    let (kobold, human) = (KOBOLD.unit, WARRIOR.unit);
    // MulHi32(101, roll): the first roll of 71 and of 36.
    let roll_of = |value: u64| ((value << 32) / 101 + 1) as u32;
    for result in [HIT, SwingResult::Parry, SwingResult::Dodge] {
        assert_eq!(
            kit_id(catalog.exertion_sound(kobold, result, ROLL_LOW)),
            Some(53723)
        );
        assert_eq!(
            kit_id(catalog.exertion_sound(kobold, result, roll_of(71) - 1)),
            Some(53723)
        );
        assert!(
            catalog
                .exertion_sound(kobold, result, roll_of(71))
                .is_none()
        );
        assert_eq!(
            kit_id(catalog.exertion_sound(human, result, roll_of(36) - 1)),
            Some(2941)
        );
        assert!(catalog.exertion_sound(human, result, roll_of(36)).is_none());
    }
    assert_eq!(
        kit_id(catalog.exertion_sound(kobold, CRIT, ROLL_HIGH)),
        Some(53724)
    );
    assert!(catalog.exertion_sound(human, CRIT, ROLL_HIGH).is_none());
    assert!(
        catalog
            .exertion_sound(kobold, SwingResult::Miss, ROLL_LOW)
            .is_none()
    );
}

/// Injury (benilla `combat.rs:586-620`, `kit.rs:168-174`): a hit wounds a creature within
/// 60 of 0..=100 and a player within 30, a crit always; a parry, dodge or miss never.
#[test]
fn victims_voice_injuries_by_chance_on_hits() {
    let catalog = catalog();
    let (kobold, human) = (KOBOLD.unit, WARRIOR.unit);
    let roll_of = |value: u64| ((value << 32) / 101 + 1) as u32;
    assert_eq!(
        kit_id(catalog.injury_sound(kobold, HIT, roll_of(61) - 1)),
        Some(53725)
    );
    assert!(catalog.injury_sound(kobold, HIT, roll_of(61)).is_none());
    assert_eq!(
        kit_id(catalog.injury_sound(human, HIT, roll_of(31) - 1)),
        Some(2942)
    );
    assert!(catalog.injury_sound(human, HIT, roll_of(31)).is_none());
    assert_eq!(
        kit_id(catalog.injury_sound(kobold, CRIT, ROLL_HIGH)),
        Some(53726)
    );
    assert_eq!(
        kit_id(catalog.injury_sound(human, CRIT, ROLL_HIGH)),
        Some(2942)
    );
    for result in [SwingResult::Parry, SwingResult::Dodge, SwingResult::Miss] {
        assert!(catalog.injury_sound(kobold, result, ROLL_LOW).is_none());
    }
}

/// A blocked swing strikes the victim's shield (slot 3 metal, 4 wood, by the shield's
/// `Material.Flags & 1`): the kobold's staff row 10 plays 61570 on item 143 (Material 1)
/// and 61568 on the Deathguard Buckler 3276 (Material 2 Wood); without a shield it
/// strikes the body as a hit (61562). The victim still voices its injury.
#[test]
fn a_block_strikes_the_victims_shield() {
    let catalog = catalog();
    let block = SwingResult::Block;
    let with_shield = |shield_item_id| MeleeHand {
        shield_item_id: Some(shield_item_id),
        ..WARRIOR
    };
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, with_shield(143), block)),
        Some(61570)
    );
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, with_shield(3276), block)),
        Some(61568)
    );
    assert_eq!(
        kit_id(catalog.impact_sound(KOBOLD, WARRIOR, block)),
        Some(61562)
    );
    assert!(
        catalog
            .injury_sound(WARRIOR.unit, block, ROLL_LOW)
            .is_some()
    );
}
