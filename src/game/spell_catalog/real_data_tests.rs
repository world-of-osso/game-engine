//! Tests against the pinned 12.1.0.69933 CSVs; skipped when `data/db2` is absent.

use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use super::*;

fn temp_paths(tag: &str) -> SpellCatalogPaths {
    let mut paths = SpellCatalogPaths::for_data_dir(Path::new("data"));
    paths.cache_path =
        std::env::temp_dir().join(format!("spell_catalog_{tag}_{}.bin", std::process::id()));
    paths
}

fn source_present() -> bool {
    let present = temp_paths("probe")
        .source_dir
        .join("SpellName.csv")
        .exists();
    if !present {
        eprintln!("skipping: spell DB2 CSVs not present");
    }
    present
}

/// Built once per test process: a cold load (CSV build + cache write) followed
/// by a warm load from that cache.
struct Loaded {
    cold: SpellCatalogData,
    warm: SpellCatalogData,
}

fn loaded() -> Option<&'static Loaded> {
    static LOADED: OnceLock<Option<Loaded>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            if !source_present() {
                return None;
            }
            let paths = temp_paths("shared");
            let _ = std::fs::remove_file(&paths.cache_path);
            let cold = load_spell_catalog(&paths).expect("cold spell catalog load");
            let warm = load_spell_catalog(&paths).expect("warm spell catalog load");
            std::fs::remove_file(&paths.cache_path).expect("remove test cache");
            Some(Loaded { cold, warm })
        })
        .as_ref()
}

fn catalog() -> Option<&'static SpellCatalogData> {
    loaded().map(|loaded| &loaded.cold)
}

fn description(id: u32) -> String {
    let data = catalog().unwrap();
    data.render_description(id).unwrap()
}

#[test]
fn fireball_fields_and_description() {
    let Some(data) = catalog() else { return };
    let fireball = data.get(133).unwrap();
    assert_eq!(&*fireball.name, "Fireball");
    assert_eq!(fireball.icon_fdid, 135812);
    assert_eq!(fireball.school_mask, 4);
    assert_eq!(fireball.cast_time_ms, 1750);
    assert_eq!(fireball.range.max_yd, [40.0, 40.0]);
    assert_eq!(fireball.cooldown.gcd_ms, 1500);
    assert_eq!(fireball.charges, None);
    assert_eq!(
        &*fireball.powers,
        &[SpellPowerCost {
            power_type: 0,
            flat: 0,
            pct: 1.0,
            required_aura_spell_id: 0
        }]
    );
    assert_eq!(
        description(133),
        "Throws a fiery ball that causes 0 Fire damage.$?a157642[\r\n\r\nEach time your Fireball \
         fails to critically strike a target, it gains a stacking 20% increased critical strike \
         chance. Effect ends when Fireball critically strikes.][]"
    );
}

#[test]
fn crusader_strike_charges_and_description() {
    let Some(data) = catalog() else { return };
    let strike = data.get(35395).unwrap();
    assert_eq!(
        strike.charges,
        Some(SpellCharges {
            max_charges: 2,
            recovery_ms: 6000
        })
    );
    assert_eq!(strike.range.max_yd, [5.0, 5.0]);
    assert_eq!(strike.powers.len(), 4);
    assert_eq!(
        description(35395),
        "Strike the target for $<damage> $?s403664 [Holystrike][Physical] damage.$?a196926[\r\n\
         \r\nReduces the cooldown of Judgment by ${$196926m1/-1000}.1 sec.][]\r\n\r\n\
         |cFFFFFFFFGenerates 1 Holy Power."
    );
}

#[test]
fn shadow_word_pain_description_and_aura() {
    let Some(data) = catalog() else { return };
    assert_eq!(
        description(589),
        "A word of darkness that causes $?a390707[${$s1*(1+$390707s1/100)}][0] Shadow damage \
         instantly, and an additional $?a390707[${$o2*(1+$390707s1/100)}][0] Shadow damage over \
         16 sec.$?s137033[\r\n\r\n|cFFFFFFFFGenerates ${$m3/100} Insanity.|r][]"
    );
    assert_eq!(
        data.render_aura_description(589).unwrap(),
        "Suffering 0 Shadow damage every 2 sec."
    );
}

#[test]
fn token_coverage_on_real_spells() {
    if catalog().is_none() {
        return;
    }
    assert_eq!(
        description(606),
        "Drains 34.29 mana from the target over 30 sec."
    );
    assert_eq!(
        description(6673),
        "Increases the attack power of all raid and party members within 100 yards by 5% for \
         1 hour."
    );
    assert!(
        description(6254).ends_with("affects up to 3 targets, causing 5.19 Nature damage to each.")
    );
    assert_eq!(
        description(15572),
        "Hacks at an enemy's armor, increasing Physical damage taken by 5% for 20 sec, stacking \
         up to 5 times."
    );
    assert!(description(8788).starts_with(
        "Surrounds an ally with 3 balls of lightning that have 50% chance of striking melee or \
         ranged attackers for 6.06 damage."
    ));
    assert!(description(8788).ends_with("expires after 10 min. or after it has struck 3 times."));
    assert!(description(2825).ends_with("Bloodlust or Time Warp again for 10 min."));
    assert_eq!(
        description(3409),
        "Coats a weapon with poison that lasts for 1 hour.\r\nEach strike has a 30% chance of \
         poisoning the enemy, slowing their movement speed by 50% for 12 sec."
    );
}

#[test]
fn warm_cache_load_matches_cold_build() {
    let Some(loaded) = loaded() else { return };
    assert!(loaded.cold.len() > 400_000);
    assert_eq!(loaded.warm.spells, loaded.cold.spells);
}

#[test]
fn changed_source_key_invalidates_cache() {
    let Some(data) = catalog() else { return };
    let paths = temp_paths("stale");
    let key = cache::cache_key(&paths.source_dir).unwrap();
    cache::write_cache(&paths.cache_path, &key, &data.spells[..1]).unwrap();
    let cached = cache::read_cache(&paths.cache_path, &key).unwrap();
    assert_eq!(cached.as_deref(), Some(&data.spells[..1]));

    let other_source =
        std::env::temp_dir().join(format!("spell_catalog_src_{}", std::process::id()));
    std::fs::create_dir_all(&other_source).unwrap();
    for table in build::SOURCE_TABLES {
        std::fs::write(other_source.join(format!("{table}.csv")), "ID\n").unwrap();
    }
    let changed = cache::cache_key(&other_source).unwrap();
    let stale = cache::read_cache(&paths.cache_path, &changed).unwrap();
    std::fs::remove_dir_all(other_source).unwrap();
    std::fs::remove_file(paths.cache_path).unwrap();
    assert!(stale.is_none());
}

/// `cargo test --lib spell_catalog_load_probe -- --ignored --nocapture`
#[test]
#[ignore]
fn spell_catalog_load_probe() {
    if !source_present() {
        return;
    }
    let paths = temp_paths("probe");
    let _ = std::fs::remove_file(&paths.cache_path);
    let started = Instant::now();
    let cold = load_spell_catalog(&paths).unwrap();
    let cold_time = started.elapsed();
    drop(cold);
    let started = Instant::now();
    let warm = load_spell_catalog(&paths).unwrap();
    let warm_time = started.elapsed();
    let cache_bytes = std::fs::metadata(&paths.cache_path).unwrap().len();
    std::fs::remove_file(&paths.cache_path).unwrap();
    eprintln!(
        "spells={} cold={cold_time:?} warm={warm_time:?} cache={cache_bytes}B heap={}B",
        warm.len(),
        heap_bytes(&warm)
    );
}

fn heap_bytes(data: &SpellCatalogData) -> usize {
    data.spells.capacity() * size_of::<CatalogSpell>()
        + data
            .spells
            .iter()
            .map(|spell| {
                spell.name.len()
                    + spell.subtext.len()
                    + spell.description.len()
                    + spell.aura_description.len()
                    + size_of_val(&*spell.effects)
                    + size_of_val(&*spell.powers)
            })
            .sum::<usize>()
}

#[test]
fn passive_flag_and_spellbook_tabs() {
    let Some(data) = catalog() else { return };
    assert!(data.get(76671).unwrap().passive, "Mastery: Divine Bulwark");
    assert!(!data.get(35395).unwrap().passive, "Crusader Strike");
    let tabs = &data.tabs;
    let protection = Some(66);
    assert_eq!(tabs.classify(35395, protection), SpellbookTabKind::Class);
    assert_eq!(tabs.classify(275779, protection), SpellbookTabKind::Spec);
    assert_eq!(tabs.classify(76671, protection), SpellbookTabKind::Spec);
    assert_eq!(tabs.classify(6603, protection), SpellbookTabKind::General);
    assert_eq!(tabs.class_name(protection, &[]), Some("Paladin"));
    assert_eq!(tabs.spec_name(protection), Some("Protection"));
    assert_eq!(
        tabs.spec_name(Some(1451)),
        None,
        "Initial Paladin has no tab"
    );
    assert_eq!(tabs.class_name(None, &[6603, 35395]), Some("Paladin"));
}
