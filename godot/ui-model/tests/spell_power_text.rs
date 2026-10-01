//! Spell tooltips with the player's replicated spell and attack power, and the spec
//! primary stat the CharacterFrame reads, against the pinned 12.1.0.69933 CSVs.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use game_engine_core::spell_catalog::{
    CasterPower, PrimaryStat, SpellCatalogData, SpellCatalogPaths, SpellTextContext,
    load_spell_catalog,
};
use game_engine_ui_model::game_tooltip::spell::{SpellTooltipInput, spell_tooltip};

const ARCANE_BLAST: u32 = 30451;
/// Necrotic Strike: "absorbs the next ${$m1/100*$AP} healing", effect 0 points 100.
const NECROTIC_STRIKE: u32 = 73975;

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn catalog() -> &'static SpellCatalogData {
    static CATALOG: OnceLock<SpellCatalogData> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut paths = SpellCatalogPaths::for_data_dir(&data_dir());
        paths.cache_path = std::env::temp_dir().join(format!(
            "ui_model_spell_power_text_{}.bin",
            std::process::id()
        ));
        let catalog = load_spell_catalog(&paths).expect("spell catalog loads");
        let _ = std::fs::remove_file(&paths.cache_path);
        catalog
    })
}

/// The server's `DerivedStats` powers of a level 20 human Arcane mage in three caster
/// cloth pieces (game-server `level_20_arcane_mage_in_caster_cloth_derives_sheet_stats`).
fn arcane_mage() -> SpellTextContext {
    SpellTextContext {
        caster_power: Some(CasterPower {
            spell_power: 66.0,
            attack_power: 0.0,
        }),
        ..SpellTextContext::default()
    }
}

#[test]
fn arcane_blast_tooltip_shows_spell_power_damage_instead_of_the_token() {
    // The tooltip wraps lines by measured FrizQuadrata width.
    game_engine_ui_model::paths::set_data_root(data_dir()).unwrap();
    let data = catalog();
    // Effect 0: SCHOOL_DAMAGE, 0 points, EffectBonusCoefficient 1.487;
    // 1.487 × 66 spell power = 98.14, truncated as Unit::SpellDamageBonusDone adds it.
    let description = data
        .render_description(ARCANE_BLAST, &arcane_mage())
        .unwrap();
    assert_eq!(
        description,
        "Blasts the target with energy, dealing 98 Arcane damage.\r\n\r\nEach Arcane \
         Charge increases damage by 60% and mana cost by 100%, and reduces cast time by \
         8%.\r\n\r\n|cFFFFFFFFGenerates 1 Arcane Charge.|r"
    );
    let spell = data.get(ARCANE_BLAST).unwrap();
    let tooltip = spell_tooltip(
        spell,
        &SpellTooltipInput {
            description,
            ..SpellTooltipInput::default()
        },
    );
    let lines: Vec<&str> = tooltip
        .content
        .lines
        .iter()
        .map(|line| line.left_text.as_str())
        .collect();
    assert!(
        lines.join(" ").contains("dealing 98 Arcane damage."),
        "{lines:?}"
    );
    assert!(lines.iter().all(|line| !line.contains("{?")), "{lines:?}");
}

#[test]
fn power_scaled_points_stay_marked_until_the_powers_arrive() {
    let description = catalog()
        .render_description(ARCANE_BLAST, &SpellTextContext::default())
        .unwrap();
    assert!(
        description.starts_with("Blasts the target with energy, dealing {?$30451s1} Arcane"),
        "{description}"
    );
}

#[test]
fn attack_power_token_reads_the_replicated_attack_power() {
    let ctx = SpellTextContext {
        caster_power: Some(CasterPower {
            spell_power: 0.0,
            attack_power: 250.0,
        }),
        ..SpellTextContext::default()
    };
    let description = catalog().render_description(NECROTIC_STRIKE, &ctx).unwrap();
    assert!(
        description.contains("absorbs the next 250 healing"),
        "{description}"
    );
}

#[test]
fn specializations_carry_their_chr_specialization_primary_stat() {
    let specs = &catalog().tabs.specs;
    let primary = |spec_id: u32| specs[&spec_id].primary_stat();
    // PrimaryStatPriority: Arcane 0, Arms 5, Feral 3, Brewmaster 2, Holy paladin 1,
    // Initial paladin 5.
    assert_eq!(primary(62), PrimaryStat::Intellect);
    assert_eq!(primary(71), PrimaryStat::Strength);
    assert_eq!(primary(103), PrimaryStat::Agility);
    assert_eq!(primary(268), PrimaryStat::Agility);
    assert_eq!(primary(65), PrimaryStat::Intellect);
    assert_eq!(primary(1451), PrimaryStat::Strength);
}
