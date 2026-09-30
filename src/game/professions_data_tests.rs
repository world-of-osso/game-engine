//! Tests against the pinned 12.1.0.69933 CSVs; fail when `data/db2` lacks them.

use std::sync::OnceLock;

use super::*;
#[path = "../../tests/unit/required_asset.rs"]
mod required_asset;
use required_asset::require_asset;

/// Built once per test process from the CSVs, cached in the temp dir.
fn catalog() -> &'static ProfessionCatalog {
    static CATALOG: OnceLock<ProfessionCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let source = db2_dir(Path::new("data"));
        require_asset(source.join("SpellReagents.csv"));
        let cache =
            std::env::temp_dir().join(format!("profession_catalog_{}.bin", std::process::id()));
        load_profession_catalog(&source, &cache).unwrap()
    })
}

#[test]
fn bolt_of_linen_cloth_takes_two_linen_and_makes_one_bolt() {
    let catalog = catalog();
    let bolt = catalog.recipe(2963).unwrap();
    assert_eq!(bolt.name, "Bolt of Linen Cloth");
    assert_eq!(
        (
            bolt.skill_line,
            bolt.skillup_line,
            bolt.trivial_low,
            bolt.trivial_high
        ),
        (197, 2540, 25, 50)
    );
    assert_eq!(
        bolt.reagents,
        vec![Reagent {
            item_id: 2589,
            count: 2
        }]
    );
    assert_eq!(bolt.output, Some((2996, 1)));
    assert_eq!(catalog.item(2589).unwrap().name, "Linen Cloth");
    assert_eq!(catalog.item(2996).unwrap().name, "Bolt of Linen Cloth");
}

#[test]
fn bolt_of_linen_cloth_is_in_materials_under_tailoring() {
    let catalog = catalog();
    let category = catalog
        .category(catalog.recipe(2963).unwrap().category)
        .unwrap();
    assert_eq!(category.name, "Materials");
    assert_eq!(category.parent, 362);
}

#[test]
fn tailoring_is_a_primary_profession_with_a_classic_tier() {
    let catalog = catalog();
    assert!(catalog.is_primary(197));
    assert!(catalog.is_profession(185), "Cooking");
    assert!(!catalog.is_primary(185));
    assert_eq!(catalog.line(197).unwrap().spell_book_spell, 3908);
    let classic = catalog.line(2540).unwrap();
    assert_eq!(
        (
            classic.name.as_str(),
            classic.parent,
            classic.parent_tier_index
        ),
        ("Classic Tailoring", 197, 4)
    );
}
