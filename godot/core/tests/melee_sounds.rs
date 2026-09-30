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
    unit: VoiceSource::Player { race: 1, sex: 0 },
};
/// Kobold Vermin (creature 6): display 10913 (model 8379 → CreatureSoundData 5042) with
/// its equipped item 5276 (Staff 10, Material 2 Wood), `ItemDisplayInfo` 5010.
const KOBOLD: MeleeHand = MeleeHand {
    item_id: Some(5276),
    display_info_id: Some(5010),
    unit: VoiceSource::Creature { display_id: 10913 },
};
const HIT: SwingResult = SwingResult::Hit { critical: false };
const CRIT: SwingResult = SwingResult::Hit { critical: true };

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
        kit(catalog.swing_sound(WARRIOR, false)),
        Some((235, (1302596..=1302605).rev().collect()))
    );
    assert_eq!(kit_id(catalog.swing_sound(WARRIOR, true)), Some(236));
    assert_eq!(
        kit(catalog.swing_sound(KOBOLD, false)),
        Some((237, vec![567936, 567943, 567941]))
    );
    assert_eq!(kit_id(catalog.swing_sound(KOBOLD, true)), Some(238));
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
                .impact_sound(attacker, victim, SwingResult::Avoided)
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
        unit: VoiceSource::Creature { display_id: 10913 },
    };
    assert_eq!(kit_id(catalog.impact_sound(fist, WARRIOR, HIT)), Some(1014));
    assert_eq!(kit_id(catalog.swing_sound(fist, false)), Some(235));
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
/// warrior's metal sword plays 1019 (`unarmedparrymetala`).
#[test]
fn blackrock_worg_fights_bare_handed() {
    let catalog = catalog();
    let worg = MeleeHand {
        item_id: None,
        display_info_id: None,
        unit: VoiceSource::Creature { display_id: 40147 },
    };
    assert_eq!(kit_id(catalog.swing_sound(worg, false)), Some(235));
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
