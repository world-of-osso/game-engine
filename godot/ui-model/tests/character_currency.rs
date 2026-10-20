use game_engine_ui_model::character_frame::{
    CharacterFrameView, CharacterTab, apply_character_frame_postsetup, character_frame_screen,
    currency_rows, currency_tooltip,
};
use shared::protocol::CurrencyEntrySnapshot;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn currency_click_selects_a_native_character_subframe() {
    assert!(
        CharacterTab::from_action("character_tab:currency").is_some(),
        "Retail CharacterFrameTab3 must route to TokenFrame, not an unported action"
    );
}

fn wallet_rows() -> Vec<game_engine_ui_model::character_frame::CurrencyRow> {
    currency_rows(&[
        CurrencyEntrySnapshot {
            id: 1792,
            name: "Honor".into(),
            amount: 0,
        },
        CurrencyEntrySnapshot {
            id: 1602,
            name: "Conquest".into(),
            amount: 0,
        },
    ])
    .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::FontString(value)) = frame.widget_data.as_ref() else {
        panic!("{name} text")
    };
    value.text.clone()
}

#[test]
fn ui_template_blend_discovered_zero_pvp_wallet_opens_source_token_rows_in_both_skins_but_empty_wallet_hides_tab()
 {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = CharacterFrameView {
            visible: true,
            race_id: 1,
            class_id: 1,
            title: "Honorproof".into(),
            ..Default::default()
        };
        let mut shared = SharedContext::new();
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(character_frame_screen);
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("CharacterFrameTab3").is_none());
        view.has_currencies = true;
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        let tab = registry
            .get(
                registry
                    .get_by_name("CharacterFrameTab3")
                    .expect("discovered wallet tab"),
            )
            .unwrap();
        assert_eq!(tab.onclick.as_deref(), Some("character_tab:currency"));
        let window = registry
            .get(registry.get_by_name("CharacterFrame").unwrap())
            .unwrap();
        let ui_toolkit::frame::Dimension::Fixed(height) = window.height else {
            panic!("CharacterFrame must have a fixed source size")
        };
        assert_eq!(
            tab.position.top,
            ui_toolkit::layout_values::Val::Px(height - 2.0),
            "discovered-wallet tabs anchor to the shown subframe BOTTOMLEFT y-2"
        );
        view.tab = CharacterTab::from_action(tab.onclick.as_deref().unwrap()).unwrap();
        view.currencies = wallet_rows();
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        for suffix in ["Left", "Middle", "Right"] {
            let name = format!("TokenHeader2Highlight{suffix}");
            let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("{name} must carry authored texture data");
            };
            assert_eq!(
                texture.blend_mode,
                ui_toolkit::widgets::texture::BlendMode::Additive
            );
        }
        apply_character_frame_postsetup(&mut registry, view.tab);
        let currency_tab = registry
            .get(registry.get_by_name("CharacterFrameTab3").unwrap())
            .unwrap();
        assert_eq!(
            currency_tab.position.top,
            ui_toolkit::layout_values::Val::Px(422.0)
        );
        assert_eq!(text(&registry, "CharacterFrameTitleText"), "Currency");
        assert_eq!(text(&registry, "TokenHeader2Name"), "Player vs. Player");
        assert_eq!(text(&registry, "TokenEntry1792Name"), "Honor");
        assert_eq!(text(&registry, "TokenEntry1602Name"), "Conquest");
        assert_eq!(text(&registry, "TokenEntry1792Count"), "0");
        assert!(registry.get_by_name("TokenFrameScrollBox").is_some());
        assert!(registry.get_by_name("ReputationScrollBox").is_none());
        let quantity = registry
            .get(registry.get_by_name("TokenEntry1792Count").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(quantity)) = quantity.widget_data.as_ref() else {
            panic!("quantity text")
        };
        assert_eq!(
            quantity.color,
            [0.5, 0.5, 0.5, 1.0],
            "TokenEntryMixin quantity0 is gray, not an earned reward"
        );
        let header = registry
            .get(registry.get_by_name("TokenHeader2").unwrap())
            .unwrap();
        assert_eq!(header.onclick.as_deref(), Some("currency:collapse"));
        view.currency_collapsed = true;
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("TokenEntry1792").is_none());
        assert!(registry.get_by_name("TokenHeader2").is_some());
    }
}

#[test]
fn pvp_currency_tooltip_uses_source_cap_only_and_refuses_unknown_metadata() {
    let rows = wallet_rows();
    let honor = currency_tooltip(&rows[0]);
    assert_eq!(rows[0].icon, 1455894);
    assert_eq!(honor.content.title, "Honor");
    assert_eq!(
        honor.content.lines[0].left_text,
        "Used to purchase Unrated PvP equipment."
    );
    assert_eq!(honor.content.lines[1].left_text, "Total Maximum: 0/15,000");
    let conquest = currency_tooltip(&rows[1]);
    assert_eq!(rows[1].icon, 1523630);
    assert_eq!(
        conquest.content.lines[1].left_text, "Total: 0",
        "dynamic31270 is not a static/weekly cap"
    );
    assert!(
        currency_rows(&[CurrencyEntrySnapshot {
            id: 3,
            name: "Legacy unsourced currency".into(),
            amount: 0
        }])
        .is_err()
    );
}
