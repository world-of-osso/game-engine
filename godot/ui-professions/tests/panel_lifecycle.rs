//! Existing Professions lifecycle case, retaining its original module/test name.
use game_engine_ui_professions::professions_frame;
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
fn ui_template_lifecycle_root_professions_unmounts_without_losing_registration_on_reopen() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_skin(skin);
        let mut shared = SharedContext::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(professions_frame::professions_screen);
        for visible in [true, false, true] {
            let mut view = professions_frame::ProfessionView::default();
            view.book.visible = visible;
            shared.insert(view);
            screen.sync(&shared, &mut registry);
            if visible {
                assert_close(&registry, "ProfessionsFrame", "professions");
            } else {
                assert!(registry.get_by_name("ProfessionsFrame").is_none());
            }
        }
    }
}
