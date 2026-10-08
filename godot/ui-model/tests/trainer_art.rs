//! Concrete production trainer row content, coins and state presentation in both skins.
use game_engine_ui_model::{
    trainer::{TrainerBook, TrainerDisplay},
    trainer_frame::{TrainerView, apply_trainer_art, trainer_screen},
};
use shared::protocol::{TrainerList, TrainerService, TrainerServiceState};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn render(skin: ActiveSkin, money: u64) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let services = [
        (100, TrainerServiceState::Available, 12345),
        (200, TrainerServiceState::Unavailable, 500),
        (300, TrainerServiceState::Known, 10),
        (400, TrainerServiceState::Available, 0),
    ]
    .into_iter()
    .map(|(spell_id, state, cost)| TrainerService {
        spell_id,
        state,
        cost,
        req_level: 5,
        req_skill_line: 0,
        req_skill_rank: 0,
        req_abilities: vec![],
        profession: spell_id == 400,
    })
    .collect();
    let mut book = TrainerBook {
        money,
        filter_menu: true,
        ..Default::default()
    };
    book.receive_list(TrainerList {
        npc: 1,
        trainer_id: 7,
        greeting: "Not a Retail text pane".into(),
        services,
    });
    let mut context = SharedContext::new();
    context.insert(skin);
    context.insert(TrainerView {
        book,
        title: "Georgio Bolero".into(),
        display: TrainerDisplay {
            names: [
                (100, "Bolt of Linen Cloth".into()),
                (200, "Bolt of Woolen Cloth".into()),
                (300, "Linen Bandage".into()),
                (400, "Tailoring".into()),
            ]
            .into(),
            icons: [(100, 132890), (200, 132894), (300, 133681), (400, 136249)].into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(trainer_screen).sync(&context, &mut registry);
    apply_trainer_art(context.get::<TrainerView>().unwrap(), &mut registry);
    registry
}
fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}
fn text(registry: &FrameRegistry, name: &str) -> (String, [f32; 4]) {
    match frame(registry, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(font) => (font.text.clone(), font.color),
        other => panic!("Expected label {name}: {other:?}"),
    }
}
#[test]
fn trainer_art_names_are_retail_normal_gold_even_when_unavailable_or_used() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, 10000);
        for (id, name) in [
            (100, "Bolt of Linen Cloth"),
            (200, "Bolt of Woolen Cloth"),
            (300, "Linen Bandage"),
            (400, "Tailoring"),
        ] {
            assert_eq!(
                text(&registry, &format!("ClassTrainerService{id}Name")),
                (name.into(), [1.0, 210.0 / 255.0, 0.0, 1.0])
            );
        }
        assert_eq!(
            text(&registry, "ClassTrainerService300Requirements").0,
            "Already known"
        );
        assert!(registry.get_by_name("ClassTrainerGreeting").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn trainer_art_costs_use_denomination_coins_and_red_only_when_unaffordable() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (money, color) in [
            (10000, [1.0, 32.0 / 255.0, 32.0 / 255.0, 1.0]),
            (12345, [1.0; 4]),
        ] {
            let registry = render(skin, money);
            for (index, amount) in [(0, "1"), (1, "23"), (2, "45")] {
                assert_eq!(
                    text(
                        &registry,
                        &format!("ClassTrainerService100CostAmount{index}")
                    ),
                    (amount.into(), color)
                );
                assert!(matches!(
                    frame(&registry, &format!("ClassTrainerService100CostCoin{index}")).widget_data,
                    Some(WidgetData::Texture(_))
                ));
            }
            assert!(
                registry
                    .get_by_name("ClassTrainerService300CostAmount0")
                    .is_none()
            );
            assert!(
                registry
                    .get_by_name("ClassTrainerService400CostAmount0")
                    .is_none()
            );
        }
        let zero = render(skin, 0);
        assert_eq!(
            text(&zero, "ClassTrainerMoneyFrameAmount0"),
            ("0".into(), [1.0; 4])
        );
        assert!(zero.get_by_name("ClassTrainerMoneyFrameCoin0").is_some());
        assert!(zero.get_by_name("ClassTrainerMoneyFrameCoin1").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
#[test]
fn trainer_art_unavailable_icons_desaturate_without_greying_known_icons_and_selection_adds() {
    use ui_toolkit::widgets::texture::BlendMode;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, 10000);
        for (id, desaturated) in [(100, false), (200, true), (300, false), (400, false)] {
            let icon = frame(&registry, &format!("ClassTrainerService{id}Icon"));
            assert!(
                matches!(&icon.widget_data, Some(WidgetData::Texture(texture)) if texture.desaturated == desaturated)
            );
        }
        let selected = frame(&registry, "ClassTrainerService100Selected");
        assert!(
            matches!(&selected.widget_data, Some(WidgetData::Texture(texture)) if texture.blend_mode == BlendMode::Additive)
        );
        assert!(
            registry
                .get_by_name("ClassTrainerService200DisabledBG")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("ClassTrainerService300DisabledBG")
                .is_none()
        );
        assert!(!frame(&registry, "ClassTrainerService100Highlight").visible);
        assert!(registry.get_by_name("ClassTrainerFramePortrait").is_some());
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn trainer_art_requirement_numbers_use_global_string_red_not_a_red_whole_line() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, 10000);
        let requirements: Vec<_> = registry
            .frames_iter()
            .filter(|frame| {
                frame
                    .name
                    .as_deref()
                    .is_some_and(|name| name.starts_with("ClassTrainerService200Requirements"))
            })
            .filter_map(|frame| match &frame.widget_data {
                Some(WidgetData::FontString(font)) => Some((font.text.clone(), font.color)),
                _ => None,
            })
            .collect();
        assert!(
            requirements
                .iter()
                .any(|(text, color)| text.contains("Requires:") && *color == [1.0; 4])
        );
        assert!(requirements.contains(&("5".into(), [1.0, 32.0 / 255.0, 32.0 / 255.0, 1.0])));
        assert!(
            !requirements
                .iter()
                .any(|(text, color)| text.contains("Level") && *color != [1.0; 4])
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn trainer_art_wallet_keeps_retail_player_zero_lower_denominations() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (money, amounts) in [(10000, vec!["1", "0", "0"]), (5000, vec!["50", "0"])] {
            let registry = render(skin, money);
            for (index, amount) in amounts.into_iter().enumerate() {
                assert_eq!(
                    text(&registry, &format!("ClassTrainerMoneyFrameAmount{index}")),
                    (amount.into(), [1.0; 4])
                );
                assert!(
                    registry
                        .get_by_name(&format!("ClassTrainerMoneyFrameCoin{index}"))
                        .is_some()
                );
            }
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn trainer_art_filter_labels_use_retail_state_colors() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin, 0);
        for (index, value, color) in [
            (0, "Available", [25.0 / 255.0, 1.0, 25.0 / 255.0, 1.0]),
            (1, "Unavailable", [1.0, 32.0 / 255.0, 32.0 / 255.0, 1.0]),
            (
                2,
                "Used",
                [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0],
            ),
        ] {
            assert_eq!(
                text(&registry, &format!("ClassTrainerFilter{index}Label")),
                (value.into(), color)
            );
            assert!(
                registry
                    .get_by_name(&format!("ClassTrainerFilter{index}Check"))
                    .is_some()
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
