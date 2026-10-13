use game_engine_ui_model::auction::{AuctionGossipView, auction_gossip_screen};
use ui_toolkit::{
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn selection_lists_gossip_service_choices_are_flat_rows_not_action_buttons() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(AuctionGossipView {
        text: "How can I help?".into(),
        options: vec![shared::protocol::GossipMenuOption {
            option_id: 0,
            icon: 0,
            text: "I want to transmogrify my gear.".into(),
        }],
    });
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(auction_gossip_screen).sync(&shared, &mut registry);
    let id = registry.get_by_name("AuctionGossipOption0").unwrap();
    let Some(WidgetData::Button(button)) = &registry.get(id).unwrap().widget_data else {
        panic!("service choice")
    };
    assert!(
        !button.use_default_skin,
        "gossip service choices are list rows"
    );
    assert_eq!(registry.click_frame(id), Some("auction_gossip:0".into()));
}
