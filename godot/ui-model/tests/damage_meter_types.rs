//! Damage meter types counted from the combat log lines the client receives: healing,
//! interrupts, dispels, deaths and the death recap (docs/specs/damage-meter.md).

use game_engine_ui_model::damage_meter_data::{
    ACTION_DAMAGE_METER_CURRENT, ACTION_DAMAGE_METER_MENU, ACTION_DAMAGE_METER_OVERALL,
    ACTION_DAMAGE_METER_TYPE_MENU, DamageMeterWindow, MeterEvent, MeterSessionType, MeterType,
    MeterUnit,
};
use shared::protocol::{
    CombatLogEvent, CombatLogKind, DamageMeterSession, DamageMeterSnapshot, DamageMeterSource,
};

const SHOT: u64 = 10;
const PRIEST: u64 = 11;
const KOBOLD: u64 = 20;
const GEOMANCER: u64 = 21;

fn unit(id: u64) -> MeterUnit {
    let (name, class_id) = match id {
        SHOT => ("Shot", 2),
        PRIEST => ("Fbpriest", 5),
        KOBOLD => ("Kobold Vermin", 0),
        GEOMANCER => ("Kobold Geomancer", 0),
        other => panic!("no unit {other}"),
    };
    MeterUnit {
        unit: id,
        name: name.into(),
        class_id,
        is_local_player: id == SHOT,
    }
}

/// Spell ids of the test lines: the line's spell and its extra spell.
const SPELL: u32 = 1;
const EXTRA_SPELL: u32 = 2;

/// A combat log line arriving at client time `time`, logged by the server at the same time.
fn receive(
    window: &mut DamageMeterWindow,
    time: f64,
    kind: CombatLogKind,
    (source, target): (u64, u64),
    spell: &str,
    (amount, overflow): (i32, i32),
) {
    let line = CombatLogEvent {
        source: Some(source),
        target: Some(target),
        spell_id: (spell != "Melee").then_some(SPELL),
        school_mask: 1,
        amount,
        overflow,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: (time * 1000.0) as u64,
        kind,
    };
    receive_line(window, time, &line, spell, "");
}

/// `line` arriving at client time `time`, its spell named `spell`, its extra spell `extra`.
fn receive_line(
    window: &mut DamageMeterWindow,
    time: f64,
    line: &CombatLogEvent,
    spell: &str,
    extra: &str,
) {
    let name = |id| match id {
        SPELL => spell.to_string(),
        EXTRA_SPELL => extra.to_string(),
        other => panic!("no spell {other}"),
    };
    let (source, target) = (unit(line.source.unwrap()), unit(line.target.unwrap()));
    if let Some(event) = MeterEvent::from_combat_log(time, line, source, target, name) {
        window.log.push(event);
    }
}

fn session(session_id: u32, duration_secs: f32, active: bool) -> DamageMeterSession {
    DamageMeterSession {
        session_id,
        duration_secs,
        active,
        total_amount: 300,
        sources: vec![DamageMeterSource {
            unit: SHOT,
            name: "Shot".into(),
            class_id: 2,
            is_local_player: true,
            total_amount: 300,
            amount_per_second: 30.0,
            spells: vec![],
        }],
    }
}

/// The server's snapshot arriving at client time `now`.
fn snapshot(window: &mut DamageMeterWindow, now: f64, current: DamageMeterSession, overall: f32) {
    let snapshot = DamageMeterSnapshot {
        current: Some(current),
        overall: session(0, overall, false),
    };
    window.set_snapshot(now, Some(snapshot));
}

fn shown(window: &DamageMeterWindow) -> Vec<(String, String)> {
    window
        .rows()
        .into_iter()
        .map(|row| (row.name_text, row.value_text))
        .collect()
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

#[test]
fn healing_rows_rank_players_by_effective_healing_with_per_second_of_the_session() {
    let mut window = DamageMeterWindow {
        session: MeterSessionType::Current,
        meter_type: MeterType::HealingDone,
        ..Default::default()
    };
    // Combat 1 runs from client time 100 to 110.
    snapshot(&mut window, 100.0, session(1, 0.0, true), 0.0);
    let heal = CombatLogKind::Heal;
    receive(
        &mut window,
        101.0,
        heal,
        (SHOT, SHOT),
        "Flash of Light",
        (120, 20),
    );
    receive(
        &mut window,
        103.0,
        heal,
        (PRIEST, SHOT),
        "Flash Heal",
        (60, 0),
    );
    receive(
        &mut window,
        105.0,
        heal,
        (SHOT, PRIEST),
        "Holy Light",
        (250, 0),
    );
    // Pure overheal and a creature's heal are not listed.
    receive(
        &mut window,
        106.0,
        heal,
        (SHOT, SHOT),
        "Holy Light",
        (50, 50),
    );
    receive(
        &mut window,
        107.0,
        heal,
        (KOBOLD, KOBOLD),
        "Bandage",
        (40, 0),
    );
    snapshot(&mut window, 110.0, session(1, 10.0, false), 10.0);
    // Healing after the combat is not in the Current session.
    receive(
        &mut window,
        120.0,
        heal,
        (SHOT, SHOT),
        "Holy Light",
        (500, 0),
    );

    assert_eq!(
        shown(&window),
        pairs(&[("1. Shot", "350 (35)"), ("2. Fbpriest", "60 (6)")])
    );
    let rows = window.rows();
    assert_eq!(rows[0].fraction, 1.0);
    assert!((rows[1].fraction - 60.0 / 350.0).abs() < 1e-6);
    assert_eq!(rows[0].color, [0.96, 0.55, 0.73]);
    assert_eq!((rows[0].class_id, rows[1].class_id), (2, 5));
    assert!(rows[0].is_local_player && !rows[1].is_local_player);

    // Combat 2 runs from 200 to 205; Overall is every heal since world entry over the
    // 15 seconds of both combats.
    snapshot(&mut window, 200.0, session(2, 0.0, true), 10.0);
    receive(
        &mut window,
        202.0,
        heal,
        (SHOT, SHOT),
        "Holy Light",
        (30, 0),
    );
    snapshot(&mut window, 205.0, session(2, 5.0, false), 15.0);
    assert_eq!(shown(&window), pairs(&[("1. Shot", "30 (6)")]));
    window.click(ACTION_DAMAGE_METER_OVERALL).unwrap();
    assert_eq!(
        shown(&window),
        pairs(&[("1. Shot", "880 (58)"), ("2. Fbpriest", "60 (4)")])
    );
}

#[test]
fn interrupts_and_dispels_count_per_player_without_a_per_second_value() {
    let mut window = DamageMeterWindow::default();
    snapshot(&mut window, 50.0, session(1, 0.0, true), 0.0);
    let (interrupt, dispel) = (CombatLogKind::Interrupt, CombatLogKind::Dispel);
    receive(
        &mut window,
        51.0,
        interrupt,
        (SHOT, GEOMANCER),
        "Rebuke",
        (0, 0),
    );
    receive(
        &mut window,
        52.0,
        dispel,
        (PRIEST, SHOT),
        "Curse of Weakness",
        (1, 0),
    );
    receive(
        &mut window,
        53.0,
        interrupt,
        (SHOT, GEOMANCER),
        "Rebuke",
        (0, 0),
    );
    receive(
        &mut window,
        54.0,
        interrupt,
        (GEOMANCER, SHOT),
        "Counterspell",
        (0, 0),
    );
    receive(&mut window, 55.0, dispel, (SHOT, SHOT), "Poison", (3, 0));
    receive(&mut window, 56.0, dispel, (SHOT, PRIEST), "Slow", (1, 0));
    snapshot(&mut window, 60.0, session(1, 10.0, false), 10.0);

    window.click(MeterType::Interrupts.action()).unwrap();
    assert_eq!(shown(&window), pairs(&[("1. Shot", "2")]));
    window.click(MeterType::Dispels.action()).unwrap();
    assert_eq!(
        shown(&window),
        pairs(&[("1. Shot", "2"), ("2. Fbpriest", "1")])
    );
    assert_eq!(window.rows()[1].fraction, 0.5);
}

/// Shot dies twice in one combat that starts at client time 100.
fn two_deaths() -> DamageMeterWindow {
    let mut window = DamageMeterWindow {
        session: MeterSessionType::Current,
        ..Default::default()
    };
    let (damage, heal, death) = (
        CombatLogKind::Damage,
        CombatLogKind::Heal,
        CombatLogKind::Death,
    );
    // The opening swing arrives just before the snapshot announcing the session.
    receive(&mut window, 99.8, damage, (KOBOLD, SHOT), "Melee", (5, 0));
    snapshot(&mut window, 100.0, session(1, 0.0, true), 0.0);
    receive(&mut window, 104.0, damage, (KOBOLD, SHOT), "Melee", (30, 0));
    receive(
        &mut window,
        106.0,
        heal,
        (PRIEST, SHOT),
        "Flash Heal",
        (80, 0),
    );
    receive(
        &mut window,
        108.0,
        damage,
        (GEOMANCER, SHOT),
        "Fireball",
        (1_234, 0),
    );
    receive(&mut window, 109.0, damage, (KOBOLD, SHOT), "Melee", (20, 0));
    receive(
        &mut window,
        110.0,
        heal,
        (SHOT, SHOT),
        "Holy Light",
        (40, 0),
    );
    // Shot's own damage and the kobold's death are not part of anybody's recap.
    receive(
        &mut window,
        111.0,
        damage,
        (SHOT, KOBOLD),
        "Crusader Strike",
        (70, 0),
    );
    receive(
        &mut window,
        111.5,
        damage,
        (GEOMANCER, SHOT),
        "Melee",
        (55, 0),
    );
    receive(
        &mut window,
        112.0,
        death,
        (GEOMANCER, SHOT),
        "Melee",
        (0, 0),
    );
    receive(
        &mut window,
        113.0,
        heal,
        (PRIEST, SHOT),
        "Resurrection",
        (10, 0),
    );
    receive(&mut window, 114.0, death, (SHOT, KOBOLD), "Melee", (0, 0));
    // Second death: one hit 11 s before it, one 5 s before it.
    receive(
        &mut window,
        119.0,
        damage,
        (GEOMANCER, SHOT),
        "Fireball",
        (900, 0),
    );
    receive(
        &mut window,
        125.0,
        damage,
        (GEOMANCER, SHOT),
        "Fireball",
        (950, 0),
    );
    receive(
        &mut window,
        130.0,
        death,
        (GEOMANCER, SHOT),
        "Melee",
        (0, 0),
    );
    snapshot(&mut window, 131.0, session(1, 31.0, false), 31.0);
    window.click(MeterType::Deaths.action()).unwrap();
    window
}

#[test]
fn deaths_list_player_deaths_newest_first_with_the_time_into_the_current_session() {
    let mut window = two_deaths();
    assert_eq!(shown(&window), pairs(&[("Shot", "30s"), ("Shot", "12s")]));
    let rows = window.rows();
    assert_eq!(rows[0].fraction, 1.0);
    assert_eq!(rows[0].color, [0.96, 0.55, 0.73]);
    assert!(window.view(false, 0.0).rows_clickable());
    // Overall has no death times.
    window.click(ACTION_DAMAGE_METER_OVERALL).unwrap();
    assert_eq!(shown(&window), pairs(&[("Shot", ""), ("Shot", "")]));
    // A death after the combat is listed in Overall only.
    let death = CombatLogKind::Death;
    receive(&mut window, 500.0, death, (SHOT, SHOT), "Melee", (0, 0));
    assert_eq!(shown(&window).len(), 3);
    window.click(ACTION_DAMAGE_METER_CURRENT).unwrap();
    assert_eq!(shown(&window), pairs(&[("Shot", "30s"), ("Shot", "12s")]));
}

#[test]
fn a_death_recap_lists_the_last_damage_and_healing_before_the_death_newest_first() {
    let mut window = two_deaths();
    // The older death: the five newest of its six lines, nothing after the death and
    // nothing of the opening swing 12.2 s before it.
    window.click("damage_meter:row:1").unwrap();
    let view = window.view(false, 0.0);
    assert!(view.recap_open && view.rows_clickable());
    assert_eq!(view.type_label(), "Death Recap");
    assert_eq!(
        shown(&window),
        pairs(&[
            ("Melee by Kobold Geomancer", "-55 (0.5s)"),
            ("Holy Light by Shot", "+40 (2.0s)"),
            ("Melee by Kobold Vermin", "-20 (3.0s)"),
            ("Fireball by Kobold Geomancer", "-1,234 (4.0s)"),
            ("Flash Heal by Fbpriest", "+80 (6.0s)"),
        ])
    );
    let rows = window.rows();
    assert_eq!(rows[3].fraction, 1.0);
    assert_ne!(rows[0].color, rows[1].color);

    // Clicking a recap row closes it; the newer death has only the hit inside its
    // 10 second window.
    window.click("damage_meter:row:0").unwrap();
    assert_eq!(window.view(false, 0.0).type_label(), "Deaths");
    window.click("damage_meter:row:0").unwrap();
    assert_eq!(
        shown(&window),
        pairs(&[("Fireball by Kobold Geomancer", "-950 (5.0s)")])
    );
    // Choosing a type leaves the recap.
    window.click(MeterType::Deaths.action()).unwrap();
    assert_eq!(shown(&window).len(), 2);
}

#[test]
fn the_type_menu_switches_the_rows_and_types_without_lines_are_empty() {
    let mut window = DamageMeterWindow::default();
    snapshot(&mut window, 10.0, session(1, 10.0, false), 10.0);
    let view = window.view(false, 0.0);
    assert_eq!(view.type_label(), "Damage Done");
    assert_eq!(shown(&window), pairs(&[("1. Shot", "300 (30)")]));
    assert!(!view.rows_clickable());

    // The two menus exclude each other; choosing a type closes its menu.
    window.click(ACTION_DAMAGE_METER_MENU).unwrap();
    window.click(ACTION_DAMAGE_METER_TYPE_MENU).unwrap();
    assert!(window.type_menu_open && !window.menu_open);
    for meter_type in [
        MeterType::HealingDone,
        MeterType::Interrupts,
        MeterType::Dispels,
        MeterType::Deaths,
    ] {
        window.click(meter_type.action()).unwrap();
        assert!(!window.type_menu_open);
        assert_eq!(window.view(false, 0.0).type_label(), meter_type.label());
        assert!(window.rows().is_empty(), "{meter_type:?}");
    }
    assert!(window.click("damage_meter:row:0").is_err());
    window.click(MeterType::DamageDone.action()).unwrap();
    assert_eq!(shown(&window), pairs(&[("1. Shot", "300 (30)")]));
    assert!(window.click("damage_meter:row:0").is_err());
    assert!(window.click("damage_meter:threat").is_err());
}

/// An interrupt or dispel line naming its extra spell (`SPELL_INTERRUPT` / `SPELL_DISPEL`
/// `extraSpellId`), arriving and logged at `time`.
fn receive_extra(
    window: &mut DamageMeterWindow,
    time: f64,
    kind: CombatLogKind,
    (source, target): (u64, u64),
    (spell, extra): (&str, &str),
) {
    let line = CombatLogEvent {
        source: Some(source),
        target: Some(target),
        spell_id: Some(SPELL),
        school_mask: 1,
        amount: 0,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: Some(EXTRA_SPELL),
        timestamp_unix_ms: (time * 1000.0) as u64,
        kind,
    };
    receive_line(window, time, &line, spell, extra);
}

#[test]
fn an_interrupts_row_opens_its_breakdown_by_interrupted_spell() {
    let mut window = DamageMeterWindow::default();
    snapshot(&mut window, 50.0, session(1, 0.0, true), 0.0);
    let interrupt = CombatLogKind::Interrupt;
    let kicks = [
        (51.0, (SHOT, GEOMANCER), ("Rebuke", "Fireball")),
        (52.0, (SHOT, GEOMANCER), ("Rebuke", "Frost Nova")),
        (53.0, (SHOT, GEOMANCER), ("Rebuke", "Fireball")),
        (54.0, (GEOMANCER, SHOT), ("Counterspell", "Flash of Light")),
        (55.0, (PRIEST, GEOMANCER), ("Silence", "Fireball")),
    ];
    for (time, units, spells) in kicks {
        receive_extra(&mut window, time, interrupt, units, spells);
    }
    snapshot(&mut window, 60.0, session(1, 10.0, false), 10.0);
    window.click(MeterType::Interrupts.action()).unwrap();
    assert_eq!(
        shown(&window),
        pairs(&[("1. Shot", "3"), ("2. Fbpriest", "1")])
    );
    assert!(window.view(false, 0.0).rows_clickable());

    // Shot's row: the spells Shot interrupted, most first, in Shot's class colour.
    window.click("damage_meter:row:0").unwrap();
    let view = window.view(false, 0.0);
    assert!(view.breakdown_open && view.rows_clickable());
    assert_eq!(view.type_label(), "Interrupts");
    assert_eq!(
        shown(&window),
        pairs(&[("Fireball", "2"), ("Frost Nova", "1")])
    );
    let rows = window.rows();
    assert_eq!((rows[0].fraction, rows[1].fraction), (1.0, 0.5));
    assert_eq!(rows[0].color, [0.96, 0.55, 0.73]);

    // A breakdown row closes it; Fbpriest's breakdown is its own.
    window.click("damage_meter:row:0").unwrap();
    window.click("damage_meter:row:1").unwrap();
    assert_eq!(shown(&window), pairs(&[("Fireball", "1")]));
    // Choosing a type leaves it.
    window.click(MeterType::Interrupts.action()).unwrap();
    assert_eq!(shown(&window).len(), 2);
}

#[test]
fn a_dispels_breakdown_lists_the_removed_auras() {
    let mut window = DamageMeterWindow::default();
    snapshot(&mut window, 50.0, session(1, 0.0, true), 0.0);
    let dispel = CombatLogKind::Dispel;
    receive_extra(
        &mut window,
        51.0,
        dispel,
        (SHOT, SHOT),
        ("Cleanse", "Poison"),
    );
    receive_extra(
        &mut window,
        52.0,
        dispel,
        (SHOT, PRIEST),
        ("Cleanse", "Slow"),
    );
    receive_extra(
        &mut window,
        53.0,
        dispel,
        (SHOT, PRIEST),
        ("Cleanse", "Slow"),
    );
    window.click(MeterType::Dispels.action()).unwrap();
    window.click("damage_meter:row:0").unwrap();
    assert_eq!(shown(&window), pairs(&[("Slow", "2"), ("Poison", "1")]));
}

/// The recap's seconds come from the server's timestamps: lines that reach the client in
/// one burst keep the spacing the server logged them with.
#[test]
fn a_death_recap_times_lines_by_the_server_timestamp() {
    let mut window = DamageMeterWindow {
        session: MeterSessionType::Current,
        ..Default::default()
    };
    snapshot(&mut window, 100.0, session(1, 0.0, true), 0.0);
    let line = |kind, amount, logged_at: f64| CombatLogEvent {
        source: Some(GEOMANCER),
        target: Some(SHOT),
        spell_id: Some(SPELL),
        school_mask: 4,
        amount,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: (logged_at * 1000.0) as u64,
        kind,
    };
    // Logged 3.5 s and 1.2 s before the death; all three arrive at client time 105.
    let lines = [
        line(CombatLogKind::Damage, 400, 1_791_100_796.5),
        line(CombatLogKind::Damage, 600, 1_791_100_798.8),
        line(CombatLogKind::Death, 0, 1_791_100_800.0),
    ];
    for line in &lines {
        receive_line(&mut window, 105.0, line, "Fireball", "");
    }
    window.click(MeterType::Deaths.action()).unwrap();
    window.click("damage_meter:row:0").unwrap();
    assert_eq!(
        shown(&window),
        pairs(&[
            ("Fireball by Kobold Geomancer", "-600 (1.2s)"),
            ("Fireball by Kobold Geomancer", "-400 (3.5s)"),
        ])
    );
}
