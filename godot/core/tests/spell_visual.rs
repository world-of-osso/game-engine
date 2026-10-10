//! Retail spell visuals resolved from the pinned 12.1.0.69933 local-CASC exports.

use std::path::Path;
use std::sync::OnceLock;

use game_engine_core::spell_visual::{
    CasterContext, KitAnimation, KitTarget, SpellVisualCatalog, UnitSound, VisualEvent,
    VoiceSource, read_animation_fallbacks,
};

const SLAM: u32 = 1464;
const BATTLE_SHOUT: u32 = 6673;
const VICTORY_RUSH: u32 = 34428;
const FROSTBOLT: u32 = 116;
const FIREBALL: u32 = 133;
const FROST_NOVA: u32 = 122;
/// `Item.SubclassID` of a one- and a two-handed sword.
const SWORD_1H: u8 = 7;
const SWORD_2H: u8 = 8;

fn db2_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/db2/12.1.0.69933")
}

fn catalog() -> &'static SpellVisualCatalog {
    static CATALOG: OnceLock<SpellVisualCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| SpellVisualCatalog::build(&db2_dir()).expect("spell visual CSVs load"))
}

fn warrior(main_hand_subclass: Option<u8>) -> CasterContext {
    CasterContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 10,
        spec_order_index: None,
        main_hand_subclass,
        auras: Vec::new(),
    }
}

#[test]
fn toyfx6_spitzy_inverted_aura_condition_shows_without_disabling_aura() {
    // Local PlayerCondition64572: AuraSpellID_0=181943, AuraSpellLogic=65536.
    let visual = catalog().visual_for_spell(261981, &warrior(None));
    assert_eq!(visual, Some(74073));
    let affected = CasterContext {
        auras: vec![(181943, 1)],
        ..warrior(None)
    };
    assert_eq!(catalog().visual_for_spell(261981, &affected), None);
    let unrelated = CasterContext {
        auras: vec![(261981, 1)],
        ..warrior(None)
    };
    assert_eq!(catalog().visual_for_spell(261981, &unrelated), Some(74073));
    // ModifierTree303980 is not locally exported; do not bypass Scoots' condition.
    assert_eq!(catalog().visual_for_spell(1280563, &unrelated), None);
}

#[test]
fn specialization_slam_picks_arms_and_fury_cast_kits_with_the_same_two_handed_sword() {
    let catalog = catalog();
    // ChrSpecialization 71/72 OrderIndex 0/1, not specialization IDs or Lua 1/2.
    let select = |spec_id: Option<u32>| {
        let caster = CasterContext {
            spec_order_index: spec_id.and_then(|id| catalog.specialization_order_index(id)),
            ..warrior(Some(SWORD_2H))
        };
        let visual = catalog.visual_for_spell(SLAM, &caster).unwrap();
        let cast = &catalog.kits(visual, VisualEvent::Cast)[0];
        (visual, cast.kit_id, cast.animation.unwrap().anim_id)
    };
    assert_eq!(catalog.specialization_order_index(71), Some(0));
    assert_eq!(catalog.specialization_order_index(72), Some(1));
    assert_eq!(catalog.specialization_order_index(1446), Some(4));
    assert_eq!(catalog.specialization_order_index(0), None);
    assert_eq!(select(Some(71)), (51946, 62428, 812));
    assert_eq!(select(Some(72)), (97446, 128672, 818));
    // Missing primary specialization skips its check; priority 3 wins over 1.
    assert_eq!(select(None), (51946, 62428, 812));
    // Initial warrior spec (1446, OrderIndex 4) is known, not missing.
    assert_eq!(select(Some(1446)), (51946, 62428, 812));
}

#[test]
fn slam_swings_the_weapon_class_combat_ability_and_bloodies_the_target_chest() {
    let catalog = catalog();
    let one_hand = catalog
        .visual_for_spell(SLAM, &warrior(Some(SWORD_1H)))
        .unwrap();
    let two_hand = catalog
        .visual_for_spell(SLAM, &warrior(Some(SWORD_2H)))
        .unwrap();
    assert_eq!((one_hand, two_hand), (97446, 51946));

    let cast = catalog.kits(one_hand, VisualEvent::Cast);
    assert_eq!(cast.len(), 1);
    assert_eq!(cast[0].target, KitTarget::Caster);
    // AnimKit 8451: CombatAbility1H01, played once.
    assert_eq!(
        cast[0].animation,
        Some(KitAnimation {
            anim_id: 818,
            looping: false
        })
    );
    let two_hand_cast = catalog.kits(two_hand, VisualEvent::Cast);
    assert_eq!(two_hand_cast[0].animation.unwrap().anim_id, 812);

    let impact = catalog.kits(one_hand, VisualEvent::Impact);
    assert_eq!(impact.len(), 1);
    assert_eq!(impact[0].target, KitTarget::PrimaryTarget);
    let model = &impact[0].models[0];
    assert_eq!((model.model_fdid, model.attachment), (1283017, Some(15)));
    // Models lacking CombatAbility1H01 fall back to Special1H.
    let fallbacks = read_animation_fallbacks(&db2_dir()).unwrap();
    assert_eq!(fallbacks.get(&818), Some(&57));
}

#[test]
fn battle_shout_roars_once_with_a_base_effect_and_buffs_each_hit_unit() {
    let catalog = catalog();
    let visual = catalog
        .visual_for_spell(BATTLE_SHOUT, &warrior(None))
        .unwrap();
    let cast = catalog.kits(visual, VisualEvent::Cast);
    assert_eq!(
        cast[0].animation,
        Some(KitAnimation {
            anim_id: 55,
            looping: false
        })
    );
    assert_eq!(cast[0].models[0].model_fdid, 1138011);
    assert_eq!(cast[0].models[0].attachment, None);
    let impact = catalog.kits(visual, VisualEvent::Impact);
    assert_eq!(impact[0].target, KitTarget::HitUnits);
    assert_eq!(impact[0].models[0].model_fdid, 6194303);
}

#[test]
fn victory_rush_picks_the_one_hand_visual_for_a_sword_and_board_warrior() {
    let catalog = catalog();
    let visual = catalog
        .visual_for_spell(VICTORY_RUSH, &warrior(Some(SWORD_1H)))
        .unwrap();
    assert_eq!(visual, 56744);
    let cast = catalog.kits(visual, VisualEvent::Cast);
    assert_eq!(cast[0].animation.unwrap().anim_id, 824);
    // No main hand satisfies neither weapon condition.
    assert_eq!(catalog.visual_for_spell(VICTORY_RUSH, &warrior(None)), None);
}

#[test]
fn frostbolt_holds_a_directed_ready_loop_with_hand_effects_then_launches_a_missile() {
    let catalog = catalog();
    let mage = CasterContext {
        class: 8,
        ..warrior(None)
    };
    let visual = catalog.visual_for_spell(FROSTBOLT, &mage).unwrap();
    let precast = catalog.kits(visual, VisualEvent::PrecastStart);
    assert_eq!(precast[0].end, VisualEvent::PrecastEnd);
    assert_eq!(
        precast[0].animation,
        Some(KitAnimation {
            anim_id: 51,
            looping: true
        })
    );
    let hands: Vec<_> = precast[0]
        .models
        .iter()
        .map(|model| (model.model_fdid, model.attachment))
        .collect();
    assert_eq!(hands, vec![(1598571, Some(22)), (1598571, Some(21))]);
    let cast = catalog.kits(visual, VisualEvent::Cast);
    assert_eq!(
        cast[0].animation,
        Some(KitAnimation {
            anim_id: 53,
            looping: false
        })
    );
    let missile = catalog.missile(visual).unwrap();
    assert_eq!(missile.model_fdid, 1598570);
    assert_eq!(
        (missile.cast_attachment, missile.impact_attachment),
        (Some(56), Some(34))
    );
}

/// (SoundKit, looping, file FDIDs) of the kits `visual` starts at `event`.
fn sounds(visual: u32, event: VisualEvent) -> Vec<(u32, bool, Vec<u32>)> {
    catalog()
        .kits(visual, event)
        .iter()
        .flat_map(|kit| &kit.sounds)
        .map(|sound| {
            let files = sound.files.iter().map(|file| file.fdid).collect();
            (sound.sound_kit_id, sound.looping, files)
        })
        .collect()
}

/// Frostbolt's kits play `spell_ma_revamp_frostbolt_*`: the precast kit 81575 starts
/// SoundKit 85501 (precast_start_01-04) and loops 85500 (precast_loop_01-04, Flags
/// 0x200); the cast kit 81337 plays 85502 (cast_01-04) and the impact kit 80718 plays
/// 85503 (impact_01-04), audible out to 55 yards.
#[test]
fn frostbolt_kits_play_the_frostbolt_precast_cast_and_impact_sound_kits() {
    let mage = CasterContext {
        class: 8,
        ..warrior(None)
    };
    let visual = catalog().visual_for_spell(FROSTBOLT, &mage).unwrap();
    assert_eq!(
        sounds(visual, VisualEvent::PrecastStart),
        vec![
            (85500, true, vec![1631387, 1631388, 1631389, 1631390]),
            (85501, false, vec![1631391, 1631392, 1631393, 1631394]),
        ]
    );
    assert_eq!(
        sounds(visual, VisualEvent::Cast),
        vec![(85502, false, vec![1631379, 1631380, 1631381, 1631382])]
    );
    assert_eq!(
        sounds(visual, VisualEvent::Impact),
        vec![(85503, false, vec![1631383, 1631384, 1631385, 1631386])]
    );
    let impact = &catalog().kits(visual, VisualEvent::Impact)[0].sounds[0];
    assert!((impact.volume - 0.8).abs() < 1e-6);
    assert_eq!((impact.min_distance, impact.distance_cutoff), (25.0, 55.0));
}

fn mage() -> CasterContext {
    CasterContext {
        class: 8,
        ..warrior(None)
    }
}

/// Battle Shout's cast kit 43995 plays SoundKit 114049 and the warrior's own battle shout
/// (unit sound 38): Human male CreatureSoundData 49 → 58088, female 50 → 58100.
#[test]
fn battle_shout_plays_its_kit_sound_and_the_caster_voice() {
    let catalog = catalog();
    let visual = catalog
        .visual_for_spell(BATTLE_SHOUT, &warrior(None))
        .unwrap();
    let cast = &catalog.kits(visual, VisualEvent::Cast)[0];
    let kit_sounds: Vec<u32> = cast.sounds.iter().map(|sound| sound.sound_kit_id).collect();
    assert_eq!(kit_sounds, vec![114049]);
    assert_eq!(cast.unit_sounds, vec![UnitSound::BattleShout]);
    let voice = |sex| {
        let source = VoiceSource::Player { race: 1, sex };
        catalog
            .unit_sound(source, UnitSound::BattleShout)
            .map(|sound| (sound.sound_kit_id, sound.files.len()))
    };
    assert_eq!(voice(0), Some((58088, 7)));
    assert_eq!(voice(1), Some((58100, 8)));
    // Human voices have no spell-cast-directed sound in 12.1.0.69933.
    let source = VoiceSource::Player { race: 1, sex: 0 };
    assert!(
        catalog
            .unit_sound(source, UnitSound::SpellCastDirected)
            .is_none()
    );
}

/// Fireball's missile plays its looping travel sound 349558 (SoundEntriesID).
#[test]
fn fireball_missile_plays_its_travel_sound() {
    let catalog = catalog();
    let visual = catalog.visual_for_spell(FIREBALL, &mage()).unwrap();
    let sound = catalog.missile(visual).unwrap().sound.unwrap();
    let files: Vec<u32> = sound.files.iter().map(|file| file.fdid).collect();
    assert_eq!((sound.sound_kit_id, sound.looping), (349558, true));
    assert_eq!(files, vec![2066594, 2066595]);
    let frostbolt = catalog.visual_for_spell(FROSTBOLT, &mage()).unwrap();
    assert_eq!(catalog.missile(frostbolt).unwrap().sound, None);
}

/// Frost Nova's aura kit 266131 loops 350097 and plays 350098 on each rooted unit while
/// the aura lasts; its AuraEnd kit 85719 plays 85938 when it breaks.
#[test]
fn frost_nova_aura_kits_sound_while_rooted_and_when_it_ends() {
    let catalog = catalog();
    let visual = catalog.visual_for_spell(FROST_NOVA, &mage()).unwrap();
    let kit_sounds = |event| {
        catalog
            .kits(visual, event)
            .into_iter()
            .map(|kit| {
                let sounds: Vec<_> = kit
                    .sounds
                    .iter()
                    .map(|sound| (sound.sound_kit_id, sound.looping))
                    .collect();
                (kit.kit_id, kit.target, kit.end, sounds)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kit_sounds(VisualEvent::AuraStart),
        vec![(
            266131,
            KitTarget::HitUnits,
            VisualEvent::AuraEnd,
            vec![(350097, true), (350098, false)]
        )]
    );
    assert_eq!(
        kit_sounds(VisualEvent::AuraEnd),
        vec![(
            85719,
            KitTarget::HitUnits,
            VisualEvent::OneShot,
            vec![(85938, false)]
        )]
    );
}
