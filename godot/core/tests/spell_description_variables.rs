//! Spell description templates rendered to tooltip text: effect points, durations,
//! conditionals, arithmetic and `SpellDescriptionVariables` `$<name>` variables.

use std::path::{Path, PathBuf};

use game_engine_core::spell_catalog::{
    CasterPower, CatalogEffect, CatalogSpell, SpellCatalogData, SpellCatalogPaths,
    SpellTextContext, SpellbookTabIndex, load_spell_catalog,
};

const SPELL: u32 = 900_001;
const CRUSADER_STRIKE: u32 = 35395;
/// Retribution Paladin spec passive and the PvP aura Crusader Strike's variables test.
const RETRIBUTION_AURA: u32 = 137027;
const PVP_AURA: u32 = 134735;

fn effect(index: u8, base_points: f32) -> CatalogEffect {
    CatalogEffect {
        index,
        effect: 2,
        aura: 0,
        base_points,
        aura_period_ms: 0,
        chain_targets: 0,
        radius_yd: 0.0,
        spell_power_coefficient: 0.0,
        attack_power_coefficient: 0.0,
        level_scaled: false,
    }
}

/// One spell with `description`, `variables`, effect points `[5, 3]` and `duration_ms`.
fn render(description: &str, variables: &str, duration_ms: i32, ctx: &SpellTextContext) -> String {
    let spell = CatalogSpell {
        id: SPELL,
        name: "Test Strike".into(),
        description: description.into(),
        description_variables: variables.into(),
        duration_ms,
        effects: vec![effect(0, 5.0), effect(1, 3.0)].into(),
        ..CatalogSpell::default()
    };
    let catalog = SpellCatalogData::from_parts(vec![spell], SpellbookTabIndex::default());
    catalog.render_description(SPELL, ctx).unwrap()
}

fn plain(description: &str) -> String {
    render(description, "", 0, &SpellTextContext::default())
}

#[test]
fn effect_points_replace_s_and_m_tokens() {
    assert_eq!(
        plain("Strike the target for $s1 Physical damage."),
        "Strike the target for 5 Physical damage."
    );
    assert_eq!(plain("Hits $m1 then $s2 times."), "Hits 5 then 3 times.");
}

#[test]
fn a_description_variable_renders_its_definition() {
    let text = render(
        "Strike the target for $<damage> Physical damage.",
        "$damage=${$s1*$s2}",
        0,
        &SpellTextContext::default(),
    );
    assert_eq!(text, "Strike the target for 15 Physical damage.");
}

#[test]
fn variables_nest_through_conditionals_inside_arithmetic() {
    // Crusader Strike's SpellDescriptionVariables 334 shape, its stray `}` included.
    let variables = "$pvp=$?a134735[${1.3}][${1}]}\n\
                     $retribution=$?a137027[${1.00*$<pvp>}][${1}]\n\
                     $damage=${$s1*2*$<retribution>}";
    let text = |auras: Vec<u32>| {
        let ctx = SpellTextContext {
            auras,
            ..SpellTextContext::default()
        };
        render("Deals $<damage> damage.", variables, 0, &ctx)
    };
    assert_eq!(text(vec![]), "Deals 10 damage.");
    assert_eq!(text(vec![RETRIBUTION_AURA]), "Deals 10 damage.");
    assert_eq!(text(vec![RETRIBUTION_AURA, PVP_AURA]), "Deals 13 damage.");
}

#[test]
fn known_spell_conditionals_pick_either_branch() {
    let description = "Deals $?s12345[Holy][Physical] damage.";
    let known = SpellTextContext {
        known_spells: vec![12345],
        ..SpellTextContext::default()
    };
    assert_eq!(plain(description), "Deals Physical damage.");
    assert_eq!(render(description, "", 0, &known), "Deals Holy damage.");
}

#[test]
fn durations_format_in_seconds_and_minutes() {
    let ctx = SpellTextContext::default();
    assert_eq!(render("Lasts $d.", "", 15_000, &ctx), "Lasts 15 sec.");
    assert_eq!(render("Lasts $d.", "", 90_000, &ctx), "Lasts 1.5 min.");
}

#[test]
fn arithmetic_honours_its_decimal_suffix() {
    assert_eq!(plain("Heals ${$s1*1.22}.1% health."), "Heals 6.1% health.");
    assert_eq!(plain("Heals ${$s1*1.22}% health."), "Heals 6% health.");
}

#[test]
fn crusader_strike_reads_its_attack_power_damage_variable() {
    let data_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut paths = SpellCatalogPaths::for_data_dir(&data_dir);
    paths.cache_path = std::env::temp_dir().join(format!(
        "spell_description_variables_{}.bin",
        std::process::id()
    ));
    let catalog = load_spell_catalog(&paths).expect("spell catalog loads");
    let _ = std::fs::remove_file(&paths.cache_path);
    // Effect 0: SCHOOL_DAMAGE, 0 points, BonusCoefficientFromAP 1.4: 1.4 × 100 AP = 140.
    let ctx = SpellTextContext {
        auras: vec![RETRIBUTION_AURA],
        caster_power: Some(CasterPower {
            spell_power: 0.0,
            attack_power: 100.0,
        }),
        ..SpellTextContext::default()
    };
    let text = catalog.render_description(CRUSADER_STRIKE, &ctx).unwrap();
    assert_eq!(
        text,
        "Strike the target for 140 Physical damage.\r\n\r\n|cFFFFFFFFGenerates 1 Holy Power."
    );
}
