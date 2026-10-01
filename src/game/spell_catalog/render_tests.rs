use super::*;

fn effect(index: u8, base_points: f32) -> CatalogEffect {
    CatalogEffect {
        index,
        effect: 6,
        aura: 3,
        base_points,
        aura_period_ms: 0,
        chain_targets: 0,
        radius_yd: 0.0,
        spell_power_coefficient: 0.0,
        attack_power_coefficient: 0.0,
        level_scaled: false,
    }
}

fn fixture_catalog() -> SpellCatalogData {
    let dot = CatalogSpell {
        id: 100,
        name: "Fixture Dot".into(),
        duration_ms: 15_000,
        max_stacks: 5,
        proc_charges: 3,
        proc_chance: 20,
        effects: vec![
            CatalogEffect {
                aura_period_ms: 3_000,
                chain_targets: 3,
                radius_yd: 8.0,
                ..effect(0, 120.0)
            },
            effect(1, -30.0),
        ]
        .into(),
        ..Default::default()
    };
    let buff = CatalogSpell {
        id: 200,
        duration_ms: 90_000,
        effects: vec![effect(0, 7.5)].into(),
        ..Default::default()
    };
    let no_data = CatalogSpell {
        id: 300,
        ..Default::default()
    };
    let nuke = CatalogSpell {
        id: 400,
        name: "Fixture Nuke".into(),
        description: "Blasts for $s1 and heals for $s2.".into(),
        aura_description: "Burning for $200s1%.".into(),
        effects: vec![
            CatalogEffect {
                spell_power_coefficient: 1.5,
                ..effect(0, 0.0)
            },
            effect(1, 25.0),
        ]
        .into(),
        ..Default::default()
    };
    let echo = CatalogSpell {
        id: 500,
        description: "Echo: $@spelldesc500".into(),
        ..Default::default()
    };
    SpellCatalogData::from_parts(vec![dot, buff, no_data, nuke, echo], Default::default())
}

fn render_with(id: u32, text: &str, ctx: &SpellTextContext) -> String {
    let catalog = fixture_catalog();
    render::render_spell_text(text, catalog.get(id).unwrap(), &catalog, ctx)
}

fn render(id: u32, text: &str) -> String {
    render_with(id, text, &SpellTextContext::default())
}

#[test]
fn effect_points_render_abs_raw_and_current() {
    assert_eq!(
        render(100, "Deals $s1 damage, $s2 abs, $m2 raw, $w1 now."),
        "Deals 120 damage, 30 abs, -30 raw, 120 now."
    );
}

#[test]
fn periodic_duration_radius_and_chain_tokens() {
    assert_eq!(
        render(100, "$o1 over $d every $t1 sec within $a1 yd, $x1 targets."),
        "600 over 15 sec every 3 sec within 8 yd, 3 targets."
    );
}

#[test]
fn aura_option_tokens() {
    assert_eq!(
        render(100, "$u stacks, $n charges, $h% chance."),
        "5 stacks, 3 charges, 20% chance."
    );
}

#[test]
fn cross_spell_refs_and_uppercase_tokens() {
    assert_eq!(
        render(100, "$200s1 for $200d, $S1 for $D."),
        "7.5 for 1.5 min, 120 for 15 sec."
    );
}

#[test]
fn conditionals_test_known_spells_auras_and_values_with_else_if_chains() {
    let text = "$?s137033[known][unknown] $?a100[aura][no aura] $?$m2<0[neg][pos] \
                $?s1[one]?a100[two][three] $?(s137033&!a100)[both][not both]$?s9[x]";
    assert_eq!(render(100, text), "unknown no aura neg three not both");
    let ctx = SpellTextContext {
        known_spells: vec![137033],
        auras: vec![100],
        spec_id: None,
        caster_power: None,
    };
    assert_eq!(render_with(100, text, &ctx), "known aura neg two not both");
    assert_eq!(
        render(
            100,
            "$?s403664 [Holystrike][Physical], $?$w1>100\n[\n$s1 big][]"
        ),
        "Physical, \n120 big"
    );
}

#[test]
fn expressions_scaling_and_plural_forms() {
    assert_eq!(
        render(
            100,
            "${$s1*2}.1 ${$m2/-10} ${($s1+$200s1*2)/3}.2 ${$gte($s1,100)+$abs($m2)}"
        ),
        "240.0 3 45.00 31"
    );
    assert_eq!(render(100, "$/1000;s1 $*2;s1 $/10;200s1"), "0.12 240 0.75");
    assert_eq!(
        render(
            100,
            "$u $lcharge:charges; / $200s1 $lsec:secs; / ${1} $Lstack:stacks;"
        ),
        "5 charges / 7.5 secs / 1 stack"
    );
}

#[test]
fn references_render_other_spells_and_stop_at_cycles() {
    assert_eq!(
        render(100, "$@spellname400: $@spelldesc400 / $@spellaura400"),
        "Fixture Nuke: Blasts for {?$s1} and heals for 25. / Burning for 7.5%."
    );
    assert_eq!(
        render(100, "$spelldesc400"),
        "Blasts for {?$s1} and heals for 25."
    );
    assert_eq!(
        render(500, "$@spelldesc500"),
        "Echo: Echo: Echo: Echo: {?$@spelldesc500}"
    );
}

#[test]
fn unresolvable_tokens_render_visible_markers() {
    assert_eq!(
        render(
            400,
            "$s1 $o1 ${$s1*2} $<mult> $AP $gHe:She; $@spellicon100 $@versadmg $?j1g[a][b]"
        ),
        "{?$s1} {?$o1} {?${$s1*2}} {?$<mult>} {?$AP} {?$gHe:She;} {?$@spellicon100} \
         {?$@versadmg} {?$?j1g}"
    );
}

#[test]
fn durations_humanize_by_unit() {
    let catalog = |duration_ms| {
        SpellCatalogData::from_parts(
            vec![CatalogSpell {
                id: 1,
                duration_ms,
                ..Default::default()
            }],
            Default::default(),
        )
    };
    for (duration_ms, expected) in [
        (1_500, "1.5 sec"),
        (60_000, "1 min"),
        (3_600_000, "1 hour"),
        (7_200_000, "2 hours"),
        (-1, "{?$d}"),
    ] {
        let data = catalog(duration_ms);
        let spell = data.get(1).unwrap();
        let ctx = SpellTextContext::default();
        assert_eq!(
            render::render_spell_text("$d", spell, &data, &ctx),
            expected
        );
    }
}
