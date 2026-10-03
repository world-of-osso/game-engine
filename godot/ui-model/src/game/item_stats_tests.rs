use super::*;
use crate::item_catalog::wait_for_item_catalog;

fn entry(item_id: u32) -> &'static ItemCatalogEntry {
    crate::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .expect("item stat test data root");
    wait_for_item_catalog()
        .get(item_id)
        .unwrap_or_else(|| panic!("item {item_id} in the catalog"))
}

#[test]
fn armor_follows_the_quality_total_and_location_tables() {
    // Thin Cloth Shoes: common cloth feet, item level 2.
    assert_eq!(item_armor(entry(2117)), 1);
    // Large Round Shield: shields read ItemArmorShield.
    assert_eq!(item_armor(entry(2129)), 24);
    // Pioneer Tunic: uncommon leather chest, item level 6.
    assert_eq!(item_armor(entry(6268)), 5);
    // Linen Cloth and Worn Shortsword have none.
    assert_eq!(item_armor(entry(2589)), 0);
    assert_eq!(item_armor(entry(25)), 0);
}

#[test]
fn weapon_damage_spreads_the_table_dps_by_variance_and_speed() {
    // Worn Shortsword: common one-hander, item level 1, 2.6 s, variance 0.5.
    let sword = weapon_damage(entry(25)).expect("weapon");
    assert!((sword.min - 0.7596).abs() < 1e-3, "{sword:?}");
    assert_eq!((sword.max, sword.speed), (1.0, 2.6));
    assert!((sword.dps - 0.3895).abs() < 1e-3);
    // Arced War Axe: rare two-hander, item level 13, 3.6 s, variance 0.7.
    let axe = weapon_damage(entry(3191)).expect("weapon");
    assert!((axe.min - 5.845).abs() < 1e-2, "{axe:?}");
    assert_eq!((axe.max, axe.speed), (12.0, 3.6));
    // Lesser Magic Wand reads the one-hand caster table.
    let wand = weapon_damage(entry(11287)).expect("wand");
    assert!((wand.dps - 0.3364).abs() < 1e-3, "{wand:?}");
    assert_eq!(weapon_damage(entry(2117)), None);
}

#[test]
fn stats_scale_the_percent_editor_by_the_random_property_points() {
    let stat = |stat, value| ItemStat { stat, value };
    // Defias Rapier: rare one-hander, item level 12: Agility, Critical Strike.
    assert_eq!(item_stats(entry(1925)), [stat(3, 2), stat(32, 2)]);
    // Arced War Axe: Strength, Stamina, Haste, Critical Strike.
    assert_eq!(
        item_stats(entry(3191)),
        [stat(4, 3), stat(7, 5), stat(36, 3), stat(32, 2)]
    );
    // Pioneer Tunic: uncommon, Agility or Intellect and Stamina.
    assert_eq!(item_stats(entry(6268)), [stat(73, 1), stat(7, 2)]);
    // Common items have no random property points.
    assert!(item_stats(entry(25)).is_empty());
}
