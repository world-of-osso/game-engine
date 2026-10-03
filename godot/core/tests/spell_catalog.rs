//! Shared spell catalog and spellbook tabs against the pinned 12.1.0.69933 CSVs.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use game_engine_core::spell_catalog::{
    SpellCatalogData, SpellCatalogPaths, SpellTextContext, load_spell_catalog,
};
use game_engine_core::spellbook_data::{SpellbookPlayer, build_spellbook_tabs};

const SLAM: u32 = 1464;
const ATTACK: u32 = 88163;
const BATTLE_SHOUT: u32 = 6673;
const CHARGE: u32 = 100;
const PARRY: u32 = 3127;
const WARRIOR_INITIAL: u32 = 1446;
const ARMS: u32 = 71;

fn human_warrior(level: u32) -> Option<SpellbookPlayer> {
    Some(SpellbookPlayer {
        class_id: 1,
        race_id: 1,
        level,
    })
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn catalog() -> &'static SpellCatalogData {
    static CATALOG: OnceLock<SpellCatalogData> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut paths = SpellCatalogPaths::for_data_dir(&data_dir());
        paths.cache_path = std::env::temp_dir().join(format!(
            "godot_spell_catalog_test_{}.bin",
            std::process::id()
        ));
        let catalog = load_spell_catalog(&paths).expect("spell catalog loads");
        let _ = std::fs::remove_file(&paths.cache_path);
        catalog
    })
}

#[test]
fn warrior_spells_carry_retail_names_icons_and_rendered_descriptions() {
    let data = catalog();
    let slam = data.get(SLAM).expect("Slam");
    assert_eq!(&*slam.name, "Slam");
    assert_eq!(slam.icon_fdid, 132340);
    assert!(!slam.passive);
    assert_eq!(data.get(BATTLE_SHOUT).unwrap().icon_fdid, 132333);

    let ctx = SpellTextContext::default();
    let shout = data.render_description(BATTLE_SHOUT, &ctx).unwrap();
    assert!(
        shout.starts_with("Increases the attack power of all raid and party members within "),
        "{shout}"
    );
    assert!(shout.ends_with(" for 1 hour."), "{shout}");
    assert!(!shout.contains('$') && !shout.contains("{?"), "{shout}");
    // Slam's $s1 scales with attack power, which the client does not know: the shared
    // renderer marks it instead of showing base points.
    let slam_text = data.render_description(SLAM, &ctx).unwrap();
    assert_eq!(
        slam_text,
        "Slams an opponent, causing {?$s1} Physical damage."
    );
}

#[test]
fn level_one_warrior_spellbook_lists_known_spells_then_later_levels() {
    // Server KnownSpellsSnapshot of a new level 1 warrior (Initial spec 1446).
    let known = [137047, SLAM, PARRY, ATTACK, 123829, 325446];
    let tabs = build_spellbook_tabs(
        &known,
        Some(WARRIOR_INITIAL),
        Some(catalog()),
        human_warrior(1),
    );
    let names: Vec<_> = tabs.iter().map(|tab| tab.name.as_str()).collect();
    // Retail shows no empty category; the server grants no General spells yet.
    assert_eq!(names, ["Warrior"]);
    // SPELL_ATTR0_DO_NOT_DISPLAY hides Warrior 137047, Block 123829 and Initial Warrior 325446.
    let entries: Vec<_> = tabs[0]
        .spells
        .iter()
        .map(|spell| (spell.id, spell.available_at))
        .collect();
    assert_eq!(
        entries,
        [
            (SLAM, None),
            (ATTACK, None),
            (PARRY, None),
            (CHARGE, Some(2)),
            (23922, Some(3)),
            (1715, Some(4)),
            (34428, Some(5)),
            (2565, Some(6)),
            (6552, Some(7)),
            (355, Some(8)),
            (1680, Some(9)),
            (BATTLE_SHOUT, Some(10)),
            (57755, Some(10)),
            (163201, Some(10)),
            (319158, Some(11)),
            (18499, Some(12)),
        ]
    );
}

#[test]
fn level_ten_arms_warrior_knows_battle_shout_and_waits_for_level_eleven() {
    // Server KnownSpellsSnapshot of a level 10 warrior with the Arms spec (game-server
    // `compute_known_spells`: class line to level 10, Arms spells to level 10).
    let known = [
        137047,
        SLAM,
        PARRY,
        ATTACK,
        123829,
        CHARGE,
        23922,
        1715,
        34428,
        2565,
        6552,
        355,
        1680,
        BATTLE_SHOUT,
        57755,
        163201,
        137049,
        162698,
        462115,
        462116,
        1256919,
        260708,
        1229376,
        1258398,
    ];
    let tabs = build_spellbook_tabs(&known, Some(ARMS), Some(catalog()), human_warrior(10));
    let names: Vec<_> = tabs.iter().map(|tab| tab.name.as_str()).collect();
    assert_eq!(names, ["Warrior", "Arms"]);
    let class = &tabs[0].spells;
    assert!(
        class
            .iter()
            .any(|spell| spell.id == BATTLE_SHOUT && spell.available_at.is_none())
    );
    let future: Vec<_> = class
        .iter()
        .filter_map(|spell| Some((spell.id, spell.available_at?)))
        .collect();
    assert_eq!(future, [(319158, 11), (18499, 12)]);
    let arms_future: Vec<_> = tabs[1]
        .spells
        .iter()
        .filter_map(|spell| Some((spell.id, spell.available_at?)))
        .collect();
    // Plate Specialization 86101 (27) is SPELL_ATTR0_DO_NOT_DISPLAY.
    assert_eq!(arms_future, [(279423, 11)]);
}

/// `SpellShapeshiftForm.BonusActionBar` of the form a spell's MOD_SHAPESHIFT (36)
/// effect applies: Cat Form 1, Bear Form 3, Moonkin Form 4. Retail's Battle Stance
/// (386164) and Defensive Stance (386208) apply no form, so they never page the bar;
/// the legacy Battle Stance 7165 (form 17) still does.
#[test]
fn shapeshift_forms_carry_their_bonus_action_bar() {
    let data = catalog();
    let bonus_bar = |id: u32| data.get(id).expect("spell").bonus_bar;
    assert_eq!(bonus_bar(768), 1, "Cat Form");
    assert_eq!(bonus_bar(5487), 3, "Bear Form");
    assert_eq!(bonus_bar(24858), 4, "Moonkin Form");
    assert_eq!(bonus_bar(7165), 1, "legacy Battle Stance");
    assert_eq!(bonus_bar(386164), 0, "Battle Stance");
    assert_eq!(bonus_bar(386208), 0, "Defensive Stance");
    assert_eq!(bonus_bar(SLAM), 0);
}
