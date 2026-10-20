use game_engine_ui_model::{
    bank_frame_component::{BankFrameState, bank_frame_screen},
    mail_frame_component::{MailFrameState, MailFrameTab, mail_frame_screen},
    quest_art::panel_button,
    quest_frame_component::{QuestFramePage, QuestFrameState, quest_frame_screen},
    raid_info::{RaidInfoState, screen as raid_info_screen},
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
    widgets::texture::TextureSource,
};

fn context<T: 'static>(state: T) -> SharedContext {
    let mut shared = SharedContext::new();
    shared.insert(state);
    shared
}

#[test]
fn shared_panel_art_real_windows_emit_source_state_art_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let cases: [(fn(&SharedContext) -> Element, SharedContext, &str); 4] = [
            (
                quest_frame_screen,
                context(QuestFrameState {
                    visible: true,
                    page: QuestFramePage::Detail {
                        title: "Wine Shop Advert".into(),
                        description: "Visit the winery.".into(),
                        objectives_text: "Bring the ticket.".into(),
                        rewards: Default::default(),
                    },
                    ..Default::default()
                }),
                "QuestFrameAcceptButton",
            ),
            (
                mail_frame_screen,
                context(MailFrameState {
                    visible: true,
                    tab: MailFrameTab::Send,
                    ..Default::default()
                }),
                "SendMailCancelButton",
            ),
            (
                raid_info_screen,
                context(RaidInfoState {
                    visible: true,
                    ..Default::default()
                }),
                "RaidInfoExtendButton",
            ),
            (
                bank_frame_screen,
                context(BankFrameState {
                    visible: true,
                    title: "Bank".into(),
                    deposit_all_label: "Deposit All Reagents".into(),
                    ..Default::default()
                }),
                "BankFrameDepositAllButton",
            ),
        ];
        for (build, shared, name) in cases {
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(build).sync(&shared, &mut registry);
            let id = registry.get_by_name(name).expect(name);
            let Some(WidgetData::Button(button)) = &registry.get(id).unwrap().widget_data else {
                panic!("{name} must be a button");
            };
            assert_eq!(
                button
                    .normal_texture
                    .as_ref()
                    .map(|texture| texture.source.clone()),
                Some(TextureSource::FileDataId(130828)),
                "{skin:?} {name}"
            );
            assert_eq!(
                button
                    .pushed_texture
                    .as_ref()
                    .map(|texture| texture.source.clone()),
                Some(TextureSource::FileDataId(130825)),
                "{skin:?} {name}"
            );
            assert_eq!(
                button
                    .disabled_texture
                    .as_ref()
                    .map(|texture| texture.source.clone()),
                Some(TextureSource::FileDataId(130824)),
                "{skin:?} {name}"
            );
            assert_eq!(
                button
                    .highlight_texture
                    .as_ref()
                    .map(|texture| texture.source.clone()),
                Some(TextureSource::FileDataId(130826)),
                "{skin:?} {name}"
            );
        }
    }
}

#[test]
fn shared_panel_art_keeps_disabled_actions_and_explicit_rectangles() {
    for enabled in [true, false] {
        let mut shared = SharedContext::new();
        shared.insert(enabled);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(|shared| {
            panel_button(
                "PanelAction".into(),
                "Accept",
                "quest_accept",
                *shared.get::<bool>().unwrap(),
                (6.0, 470.0, 120.0, 22.0),
            )
        })
        .sync(&shared, &mut registry);
        let id = registry.get_by_name("PanelAction").unwrap();
        let frame = registry.get(id).unwrap();
        assert_eq!((frame.width.value(), frame.height.value()), (120.0, 22.0));
        assert_eq!(
            registry.click_frame(id).as_deref(),
            Some(if enabled { "quest_accept" } else { "" })
        );
    }
}
