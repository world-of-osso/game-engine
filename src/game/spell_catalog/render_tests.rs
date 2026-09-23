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
    SpellCatalogData::from_sorted(vec![dot, buff, no_data])
}

fn render(id: u32, text: &str) -> String {
    let catalog = fixture_catalog();
    render::render_spell_text(text, catalog.get(id).unwrap(), &catalog)
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
fn unsupported_constructs_stay_verbatim_but_branch_text_renders() {
    let text = "${$s1*2}.1 $?a137033[yes $s1][no] $<mult> $@spellname100 $/1000;s1 $lsec:secs;";
    assert_eq!(
        render(100, text),
        "${$s1*2}.1 $?a137033[yes 120][no] $<mult> $@spellname100 $/1000;s1 $lsec:secs;"
    );
}

#[test]
fn tokens_without_data_stay_verbatim() {
    assert_eq!(
        render(300, "$s1 $d $t1 $a1 $o1 $x1 $u $n $h $999s1 $s $"),
        "$s1 $d $t1 $a1 $o1 $x1 $u $n $h $999s1 $s $"
    );
}

#[test]
fn durations_humanize_by_unit() {
    let catalog = |duration_ms| {
        SpellCatalogData::from_sorted(vec![CatalogSpell {
            id: 1,
            duration_ms,
            ..Default::default()
        }])
    };
    for (duration_ms, expected) in [
        (1_500, "1.5 sec"),
        (60_000, "1 min"),
        (3_600_000, "1 hour"),
        (7_200_000, "2 hours"),
        (-1, "$d"),
    ] {
        let data = catalog(duration_ms);
        let spell = data.get(1).unwrap();
        assert_eq!(render::render_spell_text("$d", spell, &data), expected);
    }
}
