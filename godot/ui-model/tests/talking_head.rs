//! Actual TDB1210+70448 source payload projection; entity IDs are fixture identities.
use game_engine_ui_model::talking_head::TalkingHeadState;
use shared::protocol::ConversationUpdate;

fn source() -> ConversationUpdate {
    serde_json::from_str(include_str!("fixtures/conversation-orator.json")).unwrap()
}

#[test]
#[ignore = "RED: native private conversation has no TalkingHead consumer"]
fn talking_head_native_payload_keeps_source_identity_text_sound_and_elapsed_lifetime() {
    let update = source();
    let mut state = TalkingHeadState::default();
    state.receive(update.clone(), 400).unwrap();
    let view = state
        .current()
        .expect("source private TalkingHead must be visible");
    assert_eq!(
        (
            view.conversation,
            view.line,
            view.display_id,
            view.camera_id
        ),
        (700, 67085, 116692, 1751)
    );
    assert_eq!(view.name, "Orator Krix'vizk");
    assert_eq!(
        view.text,
        update.conversations[0].lines[0]
            .text
            .as_ref()
            .unwrap()
            .male_text
    );
    assert_eq!(view.sound_kit, 250774);
    state.advance(9.0);
    assert!(
        state.current().is_some(),
        "source10s object includes elapsed750ms from receipt"
    );
    state.advance(0.3);
    assert!(
        state.current().is_none(),
        "receipt elapsed is not restarted at zero"
    );
}

#[test]
#[ignore = "RED: native private conversation has no TalkingHead dismissal state"]
fn talking_head_native_dismissed_identity_does_not_replay_and_foreign_owner_is_refused() {
    let update = source();
    let mut state = TalkingHeadState::default();
    assert!(
        state.receive(update.clone(), 401).is_err(),
        "private owner validation, not public broadcast"
    );
    state.receive(update.clone(), 400).unwrap();
    assert!(state.current().is_some());
    state.dismiss();
    assert!(state.current().is_none());
    state.receive(update.clone(), 400).unwrap();
    assert!(
        state.current().is_none(),
        "same source object stays ignored after local close"
    );
    let mut next = update;
    next.conversations[0].id = 701;
    state.receive(next.clone(), 400).unwrap();
    assert_eq!(state.current().unwrap().conversation, 701);
    next.conversations.clear();
    state.receive(next, 400).unwrap();
    assert!(
        state.current().is_none(),
        "authoritative clear removes visible source object"
    );
}

#[test]
#[ignore = "RED: authored TalkingHead native frame is absent"]
fn talking_head_source_template_geometry_name_text_and_model_in_both_skins() {
    use ui_toolkit::{
        atlas::ActiveSkin,
        frame::{Dimension, WidgetData},
        layout_values::Val,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let update = source();
        let mut state = TalkingHeadState::default();
        state.receive(update.clone(), 400).unwrap();
        let mut context = SharedContext::new();
        context.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(game_engine_ui_model::talking_head_component::talking_head_screen)
            .sync(&context, &mut registry);
        let frame = |name| {
            registry
                .get(registry.get_by_name(name).expect(name))
                .unwrap()
        };
        assert_eq!(
            (
                frame("TalkingHeadFrame").width,
                frame("TalkingHeadFrame").height
            ),
            (Dimension::Fixed(570.0), Dimension::Fixed(155.0))
        );
        assert_eq!(frame("TalkingHeadFrame").position.bottom, Val::Px(96.0));
        let model = frame("TalkingHeadModel");
        assert_eq!(
            (model.width, model.height),
            (Dimension::Fixed(115.0), Dimension::Fixed(115.0))
        );
        assert_eq!(
            (model.position.left, model.position.top),
            (Val::Px(21.0), Val::Px(21.0))
        );
        let Some(WidgetData::FontString(name)) = &frame("TalkingHeadName").widget_data else {
            panic!("source name")
        };
        assert_eq!(name.text, "Orator Krix'vizk");
        let Some(WidgetData::FontString(text)) = &frame("TalkingHeadText").widget_data else {
            panic!("source text")
        };
        assert_eq!(
            text.text,
            update.conversations[0].lines[0]
                .text
                .as_ref()
                .unwrap()
                .male_text
        );
        assert_eq!(
            (
                frame("TalkingHeadCloseButton").width,
                frame("TalkingHeadCloseButton").height
            ),
            (Dimension::Fixed(24.0), Dimension::Fixed(24.0))
        );
        assert_eq!(
            (
                frame("TalkingHeadCloseButton").position.right,
                frame("TalkingHeadCloseButton").position.top
            ),
            (Val::Px(12.0), Val::Px(12.0))
        );
    }
}
