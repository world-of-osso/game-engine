//! Retail spell visuals resolved from the pinned 12.1.0.69933 local-CASC exports.

use std::path::Path;
use std::sync::OnceLock;

use game_engine_core::spell_visual::{
    CasterContext, KitAnimation, KitTarget, SpellVisualCatalog, VisualEvent,
};

const SLAM: u32 = 1464;
const BATTLE_SHOUT: u32 = 6673;
const VICTORY_RUSH: u32 = 34428;
const FROSTBOLT: u32 = 116;
/// `Item.SubclassID` of a one- and a two-handed sword.
const SWORD_1H: u8 = 7;
const SWORD_2H: u8 = 8;

fn catalog() -> &'static SpellVisualCatalog {
    static CATALOG: OnceLock<SpellVisualCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/db2/12.1.0.69933");
        SpellVisualCatalog::build(&dir).expect("spell visual CSVs load")
    })
}

fn warrior(main_hand_subclass: Option<u8>) -> CasterContext {
    CasterContext {
        race: 1,
        class: 1,
        gender: 0,
        level: 10,
        spec_order_index: None,
        main_hand_subclass,
    }
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
    assert_eq!(catalog.anim_fallback(818), Some(57));
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
