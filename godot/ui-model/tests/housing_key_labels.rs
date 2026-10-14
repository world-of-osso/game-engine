use game_engine_core::housing::HousingCatalog;
use game_engine_core::input_bindings_data::{BindingKey, InputAction, InputBinding};
use game_engine_ui_model::housing::{HousingAuthority, HousingDashboard, HousingSurface};
use game_engine_ui_model::housing_component::{HousingView, housing_screen};
use game_engine_ui_model::housing_editor::EditorMode;
use std::sync::Arc;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::font_string::GameFont;

fn view(mode: EditorMode) -> HousingView {
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog::default()));
    model.surface = HousingSurface::Editor;
    model.editor.set_mode(mode);
    HousingView {
        model,
        authority: HousingAuthority::default(),
        viewport: [1280.0, 720.0],
        money: None,
        missing_thumbnails: Default::default(),
        vendor_names: Default::default(),
    }
}

fn assert_key_label(registry: &FrameRegistry, name: &str, expected: &str, submode: bool) {
    let id = registry
        .get_by_name(name)
        .expect("Assigned housing key label exists");
    let frame = registry.get(id).unwrap();
    assert!(!frame.hidden, "Assigned housing key label is visible");
    let Some(WidgetData::FontString(font)) = &frame.widget_data else {
        panic!("Housing key labels use source font strings");
    };
    assert_eq!(font.text, expected);
    assert_eq!(font.font, GameFont::ArialNarrow);
    assert_eq!(font.font_size, if submode { 10.0 } else { 14.0 });
    assert_eq!(
        frame.position.left,
        Val::Px(if submode { 12.0 } else { 8.0 })
    );
    assert_eq!(
        frame.position.top,
        Val::Px(if submode { 12.0 } else { 10.0 })
    );
    assert_eq!(frame.height, Dimension::Fixed(10.0));
    if submode {
        assert_eq!(frame.width, Dimension::Fixed(40.0));
        assert_eq!(font.shadow_offset, [1.0, -1.0]);
    }
}

#[test]
fn housing_mode_key_labels_follow_assignment_rebind_unbind_in_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let controls = [
        (
            "HouseEditorBasicDecorModeButtonControlText",
            InputAction::HousingBasicMode,
            InputBinding::ShiftKeyboard(BindingKey::F2),
            "s-F2",
            false,
        ),
        (
            "HouseEditorExpertDecorModeButtonControlText",
            InputAction::HousingExpertMode,
            InputBinding::CtrlKeyboard(BindingKey::F3),
            "c-F3",
            false,
        ),
        (
            "HouseEditorCleanupModeButtonControlText",
            InputAction::HousingCleanupMode,
            InputBinding::Keyboard(BindingKey::F4),
            "F4",
            false,
        ),
        (
            "TranslateSubmodeButtonControlText",
            InputAction::HousingExpertTranslate,
            InputBinding::Keyboard(BindingKey::F8),
            "F8",
            true,
        ),
        (
            "RotateSubmodeButtonControlText",
            InputAction::HousingExpertRotate,
            InputBinding::Keyboard(BindingKey::F9),
            "F9",
            true,
        ),
        (
            "ScaleSubmodeButtonControlText",
            InputAction::HousingExpertScale,
            InputBinding::Keyboard(BindingKey::F10),
            "F10",
            true,
        ),
    ];
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for mode in [EditorMode::Basic, EditorMode::Expert] {
            let mut view = view(mode);
            let original_authority = view.authority.clone();
            let mut shared = SharedContext::new();
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(housing_screen);
            for (name, action, binding, text, submode) in controls {
                if submode && mode == EditorMode::Basic {
                    continue;
                }
                view.model.editor_bindings.assign(action, binding);
                shared.insert(view.clone());
                screen.sync(&shared, &mut registry);
                assert_key_label(&registry, name, text, submode);
                view.model
                    .editor_bindings
                    .assign(action, InputBinding::Keyboard(BindingKey::F12));
                shared.insert(view.clone());
                screen.sync(&shared, &mut registry);
                assert_key_label(&registry, name, "F12", submode);
                view.model.editor_bindings.clear(action);
                shared.insert(view.clone());
                screen.sync(&shared, &mut registry);
                assert!(
                    registry
                        .get_by_name(name)
                        .is_none_or(|id| registry.get(id).unwrap().hidden),
                    "Unbound labels are hidden, not fabricated defaults"
                );
                assert_eq!(
                    view.authority, original_authority,
                    "Binding display cannot change housing authority"
                );
            }
        }
    }
}
