//! Materialized close declarations must survive both skins and screen rebuilds.
use game_engine_ui_model::{character_frame, raid_info};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::lifecycle::{CloseGroup, EscapePriority};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn set_skin(skin: ActiveSkin) {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
}

fn assert_close(registry: &FrameRegistry, name: &str, hook: &str) {
    let id = registry.get_by_name(name).expect(name);
    let binding = registry.get(id).unwrap().lifecycle.as_ref().expect(name);
    assert_eq!(binding.close_hook.as_deref(), Some(hook), "{name}");
    assert_eq!(binding.close_group, Some(CloseGroup::Panels), "{name}");
    assert_eq!(
        binding.escape_priority,
        Some(EscapePriority::AddOnPost),
        "{name}"
    );
}

#[test]
fn ui_template_lifecycle_root_character_keeps_registration_across_hide_and_rebuild() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_skin(skin);
        let mut shared = SharedContext::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(character_frame::character_frame_screen);
        for visible in [true, false, true] {
            shared.insert(character_frame::CharacterFrameView {
                visible,
                ..Default::default()
            });
            screen.sync(&shared, &mut registry);
            assert_close(&registry, "CharacterFrame", "character");
        }
    }
}

#[test]
fn ui_template_lifecycle_root_raid_info_keeps_registration_across_hide_and_rebuild() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_skin(skin);
        let mut shared = SharedContext::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(raid_info::screen);
        for visible in [true, false, true] {
            shared.insert(raid_info::RaidInfoState {
                visible,
                ..Default::default()
            });
            screen.sync(&shared, &mut registry);
            assert_close(&registry, raid_info::FRAME, "raid_info");
        }
    }
}
