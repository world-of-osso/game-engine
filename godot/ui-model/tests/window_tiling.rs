use game_engine_ui_model::{
    bank_frame_component::{BankFrameState, bank_frame_screen},
    cooldown_assist::CooldownViewerView,
    cooldown_viewer_component::cooldown_viewer_screen,
    mail_frame_component::{MailFrameState, MailFrameTab, mail_frame_screen},
    quest_frame_component::{QuestFramePage, QuestFrameState, quest_frame_screen},
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    widget_def::Element,
};

fn context<T: 'static>(state: T) -> SharedContext {
    let mut shared = SharedContext::new();
    shared.insert(state);
    shared
}

#[test]
fn source_window_background_tiles_from_one_xml_origin_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut settings = CooldownViewerView::default();
        settings.settings.open = true;
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
                "QuestFrame",
            ),
            (
                mail_frame_screen,
                context(MailFrameState {
                    visible: true,
                    tab: MailFrameTab::Send,
                    ..Default::default()
                }),
                "MailFrame",
            ),
            (
                bank_frame_screen,
                context(BankFrameState {
                    visible: true,
                    title: "Bank".into(),
                    deposit_all_label: "Deposit All Reagents".into(),
                    ..Default::default()
                }),
                "BankFrame",
            ),
            (
                cooldown_viewer_screen,
                context(settings),
                "CooldownViewerSettings",
            ),
        ];
        for (build, shared, prefix) in cases {
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(build).sync(&shared, &mut registry);
            let root = registry.get(registry.get_by_name(prefix).unwrap()).unwrap();
            let width = root.width.value();
            let height = root.height.value();
            for (suffix, rect, axes) in [
                ("Bg", (2.0, 21.0, width - 4.0, height - 23.0), (true, true)),
                (
                    "TopTileStreaks",
                    (6.0, 21.0, width - 8.0, 43.0),
                    (true, false),
                ),
            ] {
                let name = format!("{prefix}{suffix}");
                let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
                assert_eq!(
                    (frame.position.left, frame.position.top),
                    (Val::Px(rect.0), Val::Px(rect.1)),
                    "{skin:?} {name}"
                );
                assert_eq!(
                    (frame.width.value(), frame.height.value()),
                    (rect.2, rect.3),
                    "{skin:?} {name}"
                );
                let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                    panic!("{name} must supply texture render input");
                };
                assert_eq!(
                    (texture.horiz_tile, texture.vert_tile),
                    axes,
                    "{skin:?} {name}"
                );
            }
        }
    }
}
