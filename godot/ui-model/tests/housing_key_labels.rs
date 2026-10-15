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

#[test]
fn housing_mode_tooltips_use_full_keys_source_colors_and_owner_top_both_skins() {
    use game_engine_ui_model::game_tooltip::render::tooltip_size;
    use game_engine_ui_model::game_tooltip::{OwnerSide, TooltipScreen, place};
    use game_engine_ui_model::housing::EDITOR_CONTROL_TOOLTIPS;
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = view(EditorMode::Expert);
        let before = view.authority.clone();
        for (_, action, label) in EDITOR_CONTROL_TOOLTIPS {
            for (binding, suffix) in [
                (None, ""),
                (
                    Some(InputBinding::CtrlKeyboard(BindingKey::F8)),
                    " (Ctrl-F8)",
                ),
                (
                    Some(InputBinding::ShiftKeyboard(BindingKey::F12)),
                    " (Shift-F12)",
                ),
                (None, ""),
            ] {
                // Native tooltip content reads current Options, not the housing snapshot's UI-only copy.
                assert!(view.model.editor_bindings.binding(action).is_none());
                let tooltip = view
                    .model
                    .editor_control_tooltip(action, binding)
                    .unwrap()
                    .owned([600.0, 600.0, 64.0, 64.0], OwnerSide::Top)
                    .for_skin(skin);
                assert_eq!(tooltip.content.title_text(), format!("{label}{suffix}"));
                assert_eq!(tooltip.content.title_color, [1.0; 4]);
                let shown = place(
                    tooltip,
                    TooltipScreen {
                        size: [1280.0, 720.0],
                        cursor: [0.0; 2],
                    },
                );
                let [width, height] = tooltip_size(&shown);
                assert_eq!(shown.y + height, 600.0);
                assert!((shown.x + width / 2.0 - 632.0).abs() < 0.01);
                assert_tooltip_text_runs(shown, label, suffix);
                assert_eq!(view.authority, before);
            }
        }
        assert!(
            view.model
                .editor_control_tooltip(InputAction::HousingRemoveDecor, None)
                .is_none()
        );
        view.model.pending = true;
        assert!(
            view.model
                .editor_control_tooltip(InputAction::HousingBasicMode, None)
                .is_none()
        );
        view.model.pending = false;
        view.model.surface = HousingSurface::Dashboard;
        assert!(
            view.model
                .editor_control_tooltip(InputAction::HousingBasicMode, None)
                .is_none()
        );
    }
}

fn assert_tooltip_text_runs(
    shown: game_engine_ui_model::tooltip_presentation::TooltipPresentation,
    label: &str,
    suffix: &str,
) {
    use game_engine_ui_model::game_tooltip::{GameTooltipView, game_tooltip_screen};
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::thread_skin());
    shared.insert(GameTooltipView {
        main: shown,
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(game_tooltip_screen).sync(&shared, &mut registry);
    let label_frame = registry
        .get(registry.get_by_name("TooltipTitle").unwrap())
        .unwrap();
    let Some(WidgetData::FontString(label_text)) = &label_frame.widget_data else {
        panic!("Tooltip label must render text")
    };
    assert_eq!(label_text.text, label);
    assert_eq!(label_text.color, [1.0; 4]);
    if suffix.is_empty() {
        assert!(registry.get_by_name("TooltipTitleSuffix").is_none());
    } else {
        let suffix_frame = registry
            .get(registry.get_by_name("TooltipTitleSuffix").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(suffix_text)) = &suffix_frame.widget_data else {
            panic!("Binding suffix must render text")
        };
        assert_eq!(suffix_text.text, suffix);
        assert_eq!(suffix_text.color, [1.0, 0.82, 0.0, 1.0]);
        assert_eq!(suffix_text.font_size, label_text.font_size);
        let (Val::Px(suffix_left), Val::Px(label_left)) =
            (suffix_frame.position.left, label_frame.position.left)
        else {
            panic!("Tooltip spans require resolved inline positions")
        };
        assert!(suffix_left > label_left);
    }
}

#[test]
fn housing_control_hover_hides_while_pressed_and_restores_on_release_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for mode in [EditorMode::Basic, EditorMode::Expert, EditorMode::Cleanup] {
            let mut view = view(mode);
            view.authority.state = Some(shared::protocol::HousingState {
                layout: Some(shared::protocol::HouseLayout {
                    house_id: 1,
                    can_edit: true,
                    decor: vec![],
                }),
                ..Default::default()
            });
            let mut shared = SharedContext::new();
            shared.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            Screen::new(housing_screen).sync(&shared, &mut registry);
            assert_editor_hover_states(&view, &mut registry);
        }
    }
}

#[test]
fn housing_disabled_controls_use_source_default_atlas_and_darkgray_tint_both_skins() {
    use game_engine_ui_model::housing_editor::PrecisionSubmode::{Rotate, Scale, Translate};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let cases = [
        (
            EditorMode::Basic,
            Translate,
            "HouseEditorBasicDecorModeButton",
            "move",
        ),
        (
            EditorMode::Expert,
            Translate,
            "HouseEditorExpertDecorModeButton",
            "expertmode",
        ),
        (
            EditorMode::Cleanup,
            Translate,
            "HouseEditorCleanupModeButton",
            "cleanup",
        ),
        (
            EditorMode::Expert,
            Translate,
            "TranslateSubmodeButton",
            "expert-horzmove",
        ),
        (
            EditorMode::Expert,
            Rotate,
            "RotateSubmodeButton",
            "expert-rotate",
        ),
        (
            EditorMode::Expert,
            Scale,
            "ScaleSubmodeButton",
            "expert-scale",
        ),
    ];
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for case in cases {
            assert_disabled_control_case(skin, case);
        }
    }
}

fn assert_disabled_control_case(
    skin: ActiveSkin,
    case: (
        EditorMode,
        game_engine_ui_model::housing_editor::PrecisionSubmode,
        &str,
        &str,
    ),
) {
    use game_engine_ui_model::housing_component::apply_housing_postsetup;
    set_thread_skin(skin);
    let (mode, submode, name, atlas) = case;
    let mut view = view(mode);
    view.model.editor.set_submode(submode);
    view.authority.state = Some(shared::protocol::HousingState {
        layout: Some(shared::protocol::HouseLayout {
            house_id: 1,
            can_edit: true,
            decor: vec![],
        }),
        ..Default::default()
    });
    for pending in [true, false] {
        view.model.pending = pending;
        let mut shared = SharedContext::new();
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(housing_screen).sync(&shared, &mut registry);
        let id = registry.get_by_name(name).unwrap();
        let Some(WidgetData::Button(button)) = registry.get_mut(id).unwrap().widget_data.as_mut()
        else {
            panic!("Mode controls must accept button input")
        };
        button.hovered = true;
        apply_housing_postsetup(&view, &mut registry);
        let expected = if pending {
            ("default", [0.4, 0.4, 0.4, 1.0], 0.0)
        } else {
            ("active", [1.0; 4], 0.5)
        };
        assert_source_icon_visual(
            &registry,
            name,
            (
                &format!("decor-ability-{atlas}-{}", expected.0),
                expected.1,
                expected.2,
            ),
        );
    }
}

fn assert_source_icon_visual(
    registry: &FrameRegistry,
    name: &str,
    expected: (&str, [f32; 4], f32),
) {
    use ui_toolkit::widgets::texture::TextureSource;
    let (atlas, color, hover_alpha) = expected;
    let icon = registry
        .get(registry.get_by_name(&format!("{name}Icon")).unwrap())
        .unwrap();
    assert_eq!(
        icon.alpha, 1.0,
        "{name} source tint is not frame transparency"
    );
    let Some(WidgetData::Texture(texture)) = &icon.widget_data else {
        panic!("Mode icon must render a texture")
    };
    assert_eq!(texture.source, TextureSource::Atlas(atlas.into()));
    assert_eq!(texture.vertex_color, color);
    assert!(
        !texture.desaturated,
        "Source disabled mode color does not desaturate the artwork"
    );
    let hover = registry
        .get(registry.get_by_name(&format!("{name}Hover")).unwrap())
        .unwrap();
    assert_eq!(hover.alpha, hover_alpha);
}

fn assert_editor_hover_states(view: &HousingView, registry: &mut FrameRegistry) {
    use game_engine_ui_model::housing_component::apply_housing_postsetup;
    use ui_toolkit::widgets::button::ButtonState;
    for name in [
        "HouseEditorBasicDecorModeButton",
        "HouseEditorExpertDecorModeButton",
        "HouseEditorCleanupModeButton",
        "TranslateSubmodeButton",
        "RotateSubmodeButton",
        "ScaleSubmodeButton",
    ] {
        let Some(id) = registry.get_by_name(name) else {
            continue;
        };
        for (state, hovered, expected_alpha) in [
            (ButtonState::Normal, true, 0.5),
            (ButtonState::Pushed, true, 0.0),
            (ButtonState::Normal, true, 0.5),
            (ButtonState::Normal, false, 0.0),
            (ButtonState::Disabled, true, 0.0),
        ] {
            let Some(WidgetData::Button(button)) =
                registry.get_mut(id).unwrap().widget_data.as_mut()
            else {
                panic!("Editor controls require real button state")
            };
            button.state = state;
            button.hovered = hovered;
            apply_housing_postsetup(view, registry);
            let hover = registry
                .get(registry.get_by_name(&format!("{name}Hover")).unwrap())
                .unwrap();
            assert_eq!(
                hover.alpha, expected_alpha,
                "{name} {state:?} hover={hovered}"
            );
        }
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
        for mode in [EditorMode::Basic, EditorMode::Expert, EditorMode::Cleanup] {
            let mut view = view(mode);
            let original_authority = view.authority.clone();
            let mut shared = SharedContext::new();
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(housing_screen);
            for (name, action, binding, text, submode) in controls {
                if submode && mode != EditorMode::Expert {
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
