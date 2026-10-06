//! Authoritative group categories, recap caps and solo meter behavior.
use game_engine_ui_model::damage_meter_component::damage_meter_screen;
use game_engine_ui_model::damage_meter_data::{DamageMeterWindow, MeterSessionType, MeterType};
use shared::protocol::{
    CombatLogEvent, CombatLogKind, DamageMeterDeathRecap, DamageMeterSession, DamageMeterSnapshot,
    DamageMeterSource,
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn source(unit: u64, name: &str, class_id: u8, values: [u64; 5]) -> DamageMeterSource {
    DamageMeterSource {
        unit,
        name: name.into(),
        class_id,
        is_local_player: unit == 1,
        total_amount: values[0],
        amount_per_second: values[0] as f32 / 10.0,
        spells: vec![],
        healing_done: values[1],
        overhealing: 99,
        absorbs: 77,
        interrupts: values[2],
        interrupt_spells: vec![],
        dispels: values[3],
        dispel_spells: vec![],
        deaths: values[4],
        death_recaps: vec![],
    }
}

fn snapshot() -> DamageMeterSnapshot {
    let current = DamageMeterSession {
        session_id: 2,
        duration_secs: 10.0,
        active: false,
        total_amount: 660,
        sources: vec![
            source(3, "Dps", 8, [330, 60, 6, 3, 1]),
            source(2, "Tank", 1, [220, 120, 4, 2, 2]),
            source(1, "Healer", 5, [110, 240, 2, 1, 3]),
        ],
    };
    let mut overall = current.clone();
    overall.session_id = 0;
    overall.duration_secs = 20.0;
    overall.total_amount *= 2;
    for source in &mut overall.sources {
        source.total_amount *= 2;
        source.healing_done *= 2;
        source.interrupts *= 2;
        source.dispels *= 2;
        source.deaths *= 2;
    }
    DamageMeterSnapshot {
        current: Some(current),
        overall,
    }
}

fn shown(window: &DamageMeterWindow) -> Vec<(String, String)> {
    window
        .rows()
        .into_iter()
        .map(|r| (r.name_text, r.value_text))
        .collect()
}

fn assert_rendered(window: &DamageMeterWindow, skin: ActiveSkin) {
    set_thread_skin(skin);
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    if window.recap.is_some() {
        shared.insert(game_engine_core::ui_layout_data::LayoutSettings {
            damage_meter: game_engine_core::ui_layout_data::FrameSizeSettings {
                width: None,
                height: Some(320),
            },
            ..Default::default()
        });
    }
    shared.insert(window.view(false, 0.0));
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(damage_meter_screen).sync(&shared, &mut registry);
    let texts: Vec<_> = registry
        .frames_iter()
        .filter_map(|frame| match &frame.widget_data {
            Some(WidgetData::FontString(text)) => Some(text.text.as_str()),
            _ => None,
        })
        .collect();
    for (index, row) in window.rows().iter().enumerate() {
        assert!(
            texts.contains(&row.name_text.as_str()),
            "{skin:?}: missing {}",
            row.name_text
        );
        assert!(
            texts.contains(&row.value_text.as_str()),
            "{skin:?}: missing {}",
            row.value_text
        );
        if window.view(false, 0.0).rows_clickable() {
            let button = registry
                .get_by_name(&format!("DamageMeterEntry{}Button", index + 1))
                .unwrap();
            assert_eq!(
                button.onclick.as_deref(),
                Some(format!("damage_meter:row:{index}").as_str())
            );
        }
    }
}

#[test]
fn damage_meter_action_spell_breakdowns_render_every_member_in_both_skins() {
    use shared::protocol::DamageMeterActionSpell;
    let mut data = snapshot();
    for session in data
        .current
        .iter_mut()
        .chain(std::iter::once(&mut data.overall))
    {
        for source in &mut session.sources {
            source.interrupt_spells = vec![
                DamageMeterActionSpell {
                    spell_id: 2139,
                    affected_spell_id: Some(116),
                    total_amount: source.interrupts - 1,
                },
                DamageMeterActionSpell {
                    spell_id: 2139,
                    affected_spell_id: Some(133),
                    total_amount: 1,
                },
            ];
            source.dispel_spells = vec![DamageMeterActionSpell {
                spell_id: 527,
                affected_spell_id: Some(589),
                total_amount: source.dispels,
            }];
        }
    }
    let mut window = DamageMeterWindow {
        snapshot: Some(data),
        ..Default::default()
    };
    window.recap_spell_names = [
        (2139, "Counterspell"),
        (116, "Frostbolt"),
        (133, "Fireball"),
        (527, "Purify"),
        (589, "Shadow Word: Pain"),
    ]
    .into_iter()
    .map(|(id, name)| (id, name.to_owned()))
    .collect();
    for session in ["damage_meter:current", "damage_meter:overall"] {
        window.click(session).unwrap();
        let multiplier = if session == "damage_meter:overall" {
            2
        } else {
            1
        };
        for kind in [MeterType::Interrupts, MeterType::Dispels] {
            window.click(kind.action()).unwrap();
            for (index, unit) in [3, 2, 1].into_iter().enumerate() {
                assert!(window.view(false, 0.0).rows_clickable());
                window.click(&format!("damage_meter:row:{index}")).unwrap();
                let expected = if kind == MeterType::Interrupts {
                    vec![
                        (
                            "Counterspell → Frostbolt".to_owned(),
                            (unit * 2 * multiplier - 1).to_string(),
                        ),
                        ("Counterspell → Fireball".to_owned(), "1".to_owned()),
                    ]
                } else {
                    vec![(
                        "Purify → Shadow Word: Pain".to_owned(),
                        (unit * multiplier).to_string(),
                    )]
                };
                assert_eq!(shown(&window), expected);
                for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
                    assert_rendered(&window, skin);
                }
                window.click("damage_meter:row:0").unwrap();
                assert_eq!(window.rows().len(), 3);
            }
        }
    }
}

#[test]
fn damage_meter_action_detail_tracks_snapshot_and_missing_affected_identity() {
    use shared::protocol::DamageMeterActionSpell;
    let mut data = snapshot();
    data.overall.sources[1].interrupt_spells = vec![DamageMeterActionSpell {
        spell_id: 2139,
        affected_spell_id: None,
        total_amount: 4,
    }];
    let mut window = DamageMeterWindow {
        snapshot: Some(data.clone()),
        meter_type: MeterType::Interrupts,
        ..Default::default()
    };
    window.recap_spell_names.insert(2139, "Counterspell".into());
    window.click("damage_meter:row:1").unwrap();
    assert_eq!(shown(&window), vec![("Counterspell".into(), "4".into())]);
    data.overall.sources[1].interrupt_spells[0].total_amount = 9;
    data.overall.sources.reverse(); // Selection follows the member, not its old row index.
    window.set_snapshot(Some(data.clone()));
    assert_eq!(shown(&window), vec![("Counterspell".into(), "9".into())]);
    data.overall.sources.retain(|source| source.unit != 2);
    window.set_snapshot(Some(data));
    assert_eq!(window.rows().len(), 2);
    assert!(window.rows()[0].name_text.contains("Dps"));
    window.set_snapshot(None);
    assert!(window.rows().is_empty());
}

#[test]
fn damage_meter_group_snapshot_renders_exact_categories_in_both_skins() {
    let mut window = DamageMeterWindow {
        snapshot: Some(snapshot()),
        session: MeterSessionType::Current,
        ..Default::default()
    };
    let cases = [
        (
            MeterType::DamageDone,
            [
                ("1. Dps", "330 (33)"),
                ("2. Tank", "220 (22)"),
                ("3. Healer", "110 (11)"),
            ],
        ),
        (
            MeterType::HealingDone,
            [
                ("1. Healer", "240 (24)"),
                ("2. Tank", "120 (12)"),
                ("3. Dps", "60 (6)"),
            ],
        ),
        (
            MeterType::Interrupts,
            [("1. Dps", "6"), ("2. Tank", "4"), ("3. Healer", "2")],
        ),
        (
            MeterType::Dispels,
            [("1. Dps", "3"), ("2. Tank", "2"), ("3. Healer", "1")],
        ),
        (
            MeterType::Deaths,
            [("Healer", "3"), ("Tank", "2"), ("Dps", "1")],
        ),
    ];
    for (kind, expected) in cases {
        window.click(kind.action()).unwrap();
        assert_eq!(
            shown(&window),
            expected.map(|(n, v)| (n.to_owned(), v.to_owned()))
        );
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            assert_rendered(&window, skin);
        }
        let rows = window.rows();
        assert_eq!(rows.iter().filter(|r| r.is_local_player).count(), 1);
        assert!(rows.iter().all(|r| r.fraction.is_finite()));
    }
}

#[test]
fn damage_meter_current_and_overall_use_distinct_server_totals() {
    let mut window = DamageMeterWindow {
        snapshot: Some(snapshot()),
        ..Default::default()
    };
    for (kind, overall, current) in [
        (MeterType::HealingDone, "480 (24)", "240 (24)"),
        (MeterType::Interrupts, "12", "6"),
        (MeterType::Dispels, "6", "3"),
        (MeterType::Deaths, "6", "3"),
    ] {
        window.click(kind.action()).unwrap();
        window.click("damage_meter:overall").unwrap();
        assert_eq!(window.rows()[0].value_text, overall);
        window.click("damage_meter:current").unwrap();
        assert_eq!(window.rows()[0].value_text, current);
    }
}

fn recap(timestamp: u64) -> DamageMeterDeathRecap {
    DamageMeterDeathRecap {
        timestamp_unix_ms: timestamp,
        events: (1..=8)
            .map(|i| CombatLogEvent {
                source: Some(2),
                target: Some(1),
                spell_id: None,
                school_mask: 1,
                amount: i * 10,
                overflow: 0,
                absorbed: 0,
                resisted: 0,
                blocked: 0,
                crit: false,
                glancing: false,
                periodic: false,
                extra_spell_id: None,
                timestamp_unix_ms: timestamp - (9 - i) as u64 * 1000,
                kind: if i == 7 {
                    CombatLogKind::Heal
                } else {
                    CombatLogKind::Damage
                },
            })
            .collect(),
    }
}

#[test]
fn damage_meter_death_recap_shows_server_capped_events_newest_first() {
    let mut data = snapshot();
    let mut old = recap(10_000);
    old.events.last_mut().unwrap().amount = 999;
    data.overall.sources[2].death_recaps = vec![old, recap(20_000)];
    for (index, amount) in [(0, 82), (1, 81)] {
        let mut remote_recap = recap(30_000);
        for event in &mut remote_recap.events {
            event.target = Some(data.overall.sources[index].unit);
        }
        remote_recap.events.last_mut().unwrap().amount = amount;
        data.overall.sources[index].death_recaps = vec![remote_recap];
    }
    let mut window = DamageMeterWindow {
        snapshot: Some(data),
        meter_type: MeterType::Deaths,
        ..Default::default()
    };
    assert_eq!(window.rows()[0].value_text, "6"); // Total is not reduced to two exported recaps.
    window.click("damage_meter:row:0").unwrap();
    assert!(window.view(false, 0.0).recap_open);
    let rows = window.rows();
    assert_eq!(rows.len(), 8);
    assert_eq!(rows[0].name_text, "Melee by Tank");
    assert_eq!(rows[0].value_text, "-80 (1.0s)");
    assert_eq!(rows[1].value_text, "+70 (2.0s)");
    assert_eq!(rows[7].value_text, "-10 (8.0s)");
    assert_ne!(rows[0].color, rows[1].color);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assert_rendered(&window, skin);
    }
    window.click("damage_meter:row:0").unwrap();
    assert!(!window.view(false, 0.0).recap_open);
    for (index, expected) in [(1, "-81 (1.0s)"), (2, "-82 (1.0s)")] {
        window.click(&format!("damage_meter:row:{index}")).unwrap();
        assert_eq!(window.rows().len(), 8);
        assert_eq!(window.rows()[0].value_text, expected);
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            assert_rendered(&window, skin);
        }
        window.click("damage_meter:row:0").unwrap();
    }
}

#[test]
fn damage_meter_solo_damage_healing_and_empty_current_are_unchanged() {
    let mut data = snapshot();
    data.current
        .as_mut()
        .unwrap()
        .sources
        .retain(|s| s.unit == 1);
    data.overall.sources.retain(|s| s.unit == 1);
    let mut window = DamageMeterWindow {
        snapshot: Some(data),
        session: MeterSessionType::Current,
        ..Default::default()
    };
    assert_eq!(shown(&window), [("1. Healer".into(), "110 (11)".into())]);
    assert_eq!(window.timer_text(true), "[00:10] ");
    window.click(MeterType::HealingDone.action()).unwrap();
    assert_eq!(shown(&window), [("1. Healer".into(), "240 (24)".into())]);
    window.snapshot.as_mut().unwrap().current = None;
    assert!(window.rows().is_empty());
    assert_eq!(window.timer_text(true), "");
}

#[test]
fn damage_meter_zero_members_remain_present_without_nan_bars() {
    let mut data = snapshot();
    for source in &mut data.overall.sources {
        source.healing_done = 0;
    }
    let window = DamageMeterWindow {
        snapshot: Some(data),
        meter_type: MeterType::HealingDone,
        ..Default::default()
    };
    assert_eq!(window.rows().len(), 3);
    assert!(
        window
            .rows()
            .iter()
            .all(|r| r.value_text == "0 (0)" && r.fraction == 0.0)
    );
}
