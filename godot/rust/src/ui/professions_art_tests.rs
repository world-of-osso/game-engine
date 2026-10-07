//! Resolved profession geometry through the native production Taffy layout, not authored attrs.
use super::super::layout::compute_layout_with_intrinsics;
use super::preview_view;
use game_engine_ui_model::professions_frame::professions_screen;
use std::collections::HashMap;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn professions_art_native_adjacent_reagent_text_and_footer_geometry() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(preview_view());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(professions_screen).sync(&shared, &mut registry);
        let bounds = compute_layout_with_intrinsics(&registry, &HashMap::new()).unwrap();
        let rect = |name| &bounds[&registry.get_by_name(name).expect(name)];
        let frame = rect("ProfessionsFrame");
        assert_eq!((frame.width, frame.height), (942.0, 658.0));
        let slot = rect("ProfessionReagent0Slot");
        let icon = rect("ProfessionReagentIcon0");
        assert!(registry.get_by_name("ProfessionReagent0Count").is_none());
        let name = rect("ProfessionReagent0");
        assert_eq!((slot.width, slot.height), (39.0, 39.0));
        assert!(icon.x >= slot.x && icon.y >= slot.y);
        assert!(icon.x + icon.width <= slot.x + slot.width);
        assert_eq!((name.width, name.height), (108.0, 36.0));
        assert!(name.x >= slot.x + slot.width);
        let output = rect("ProfessionsOutputSlot");
        assert_eq!((output.width, output.height), (54.0, 54.0));
        let all = rect("ProfessionsCreateAll");
        let quantity = rect("ProfessionsQuantity");
        let create = rect("ProfessionsCreate");
        assert_eq!(quantity.x - all.x - all.width, 30.0);
        assert_eq!(create.x - quantity.x - quantity.width, 30.0);
        assert_eq!(frame.y + frame.height - create.y - create.height, 7.0);
    }
    set_thread_skin(ActiveSkin::Modern);
}
