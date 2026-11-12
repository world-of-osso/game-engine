use game_engine_ui_model::{
    friends_frame::{FriendsTab, WHO_QUERY_EDIT_BOX, WHO_SCROLL_BOX},
    friends_frame_component::{FriendsFrameView, friends_frame_screen},
};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, Frame},
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let frame = frame(registry, name);
    let (Val::Px(left), Val::Px(top), Dimension::Fixed(width), Dimension::Fixed(height)) = (
        frame.position.left,
        frame.position.top,
        frame.width,
        frame.height,
    ) else {
        panic!("{name} must use fixed pixel geometry")
    };
    (left, top, width, height)
}

#[test]
fn who_feedback_sits_below_results_and_above_the_search_box_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();

    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(FriendsFrameView {
            visible: true,
            position: [16.0, 104.0],
            tab: FriendsTab::Who,
            friends: Vec::new(),
            selected_friend: None,
            who: Vec::new(),
            selected_who: None,
            raid_members: Vec::new(),
            is_raid: false,
            can_convert_to_raid: false,
            has_raid_info: false,
            add_friend_open: false,
            feedback: Some("who: 1 result(s)".into()),
            error: None,
            context_menu_open: false,
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(friends_frame_screen).sync(&shared, &mut registry);

        let (_, header_top, _, header_height) = rect(&registry, "WhoFrameNameHeaderLeft");
        let (_, list_top, _, list_height) = rect(&registry, WHO_SCROLL_BOX);
        let (_, feedback_top, _, feedback_height) = rect(&registry, "FriendsFrameFeedbackText");
        let (_, search_top, _, _) = rect(&registry, WHO_QUERY_EDIT_BOX);

        assert!(
            feedback_top >= header_top + header_height,
            "{skin:?}: Who feedback overlaps the column headers"
        );
        assert!(
            feedback_top >= list_top + list_height,
            "{skin:?}: Who feedback overlaps the result list"
        );
        assert!(
            feedback_top + feedback_height <= search_top,
            "{skin:?}: Who feedback overlaps the search box"
        );
    }
}
