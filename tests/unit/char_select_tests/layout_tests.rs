use super::*;

#[test]
fn screen_builds_with_empty_char_list() {
    let reg = build_screen(CharSelectState::default());
    assert!(reg.get_by_name("CharSelectRoot").is_some());
    assert!(reg.get_by_name("EnterWorld").is_some());
    assert!(reg.get_by_name("BackToLogin").is_some());
    let ui = CharSelectUi::resolve(&reg);
    assert_eq!(ui.delete_button, None);
}

#[test]
fn screen_builds_with_characters() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });
    assert!(reg.get_by_name("CharCard_0").is_some());
    assert!(reg.get_by_name("CharCard_0Name").is_some());
}

#[test]
fn char_select_screen_builds_all_critical_frames() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    for frame_name in [
        "CharacterListCards",
        "CharCard_0",
        ENTER_WORLD_BUTTON.0,
        DELETE_CHAR_BUTTON.0,
        BACK_BUTTON.0,
        MENU_BUTTON.0,
    ] {
        assert!(
            reg.get_by_name(frame_name).is_some(),
            "expected {frame_name} to exist in char select screen"
        );
    }
}

#[test]
fn character_card_list_is_anchored_to_top_left() {
    use game_engine::ui::anchor::AnchorPoint;

    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let cards_id = reg
        .get_by_name("CharacterListCards")
        .expect("CharacterListCards");
    let list_panel_id = reg
        .get_by_name("CharacterListPanel")
        .expect("CharacterListPanel");
    assert_single_anchor(
        &reg,
        cards_id,
        AnchorPoint::TopLeft,
        AnchorPoint::TopLeft,
        Some(list_panel_id),
        19.0,
        -94.0,
    );
}

#[test]
fn character_cards_are_vertically_stacked_with_consistent_gap() {
    let reg = build_screen(CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Theron".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
        ],
        selected_index: Some(0),
        ..Default::default()
    });

    let card0 = reg
        .get_by_name("CharCard_0")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("CharCard_0 layout_rect");
    let card1 = reg
        .get_by_name("CharCard_1")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("CharCard_1 layout_rect");

    assert!(
        card0.y < card1.y,
        "expected CharCard_1 below CharCard_0, got {} <= {}",
        card1.y,
        card0.y
    );
    assert_eq!(
        card1.y - card0.y,
        card0.height + 10.0,
        "expected char card vertical spacing to match card height plus 10px gap"
    );
}

#[test]
fn char_select_action_buttons_occupy_expected_screen_regions() {
    let reg = build_screen_with_real_layout(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Theron".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let enter_world = frame_center(&reg, ENTER_WORLD_BUTTON.0);
    let delete_char = frame_center(&reg, DELETE_CHAR_BUTTON.0);
    let back = frame_center(&reg, BACK_BUTTON.0);

    assert!(
        (enter_world.x - 960.0).abs() <= 20.0,
        "expected EnterWorld centered near screen midpoint, got x={}",
        enter_world.x
    );
    assert!(
        enter_world.y > 540.0,
        "expected EnterWorld in lower half, got y={}",
        enter_world.y
    );
    assert!(
        delete_char.x > 960.0 && delete_char.y > 540.0,
        "expected DeleteChar in lower-right region, got ({}, {})",
        delete_char.x,
        delete_char.y
    );
    assert!(
        back.x < 960.0 && back.y > 540.0,
        "expected BackToLogin in lower-left region, got ({}, {})",
        back.x,
        back.y
    );
}

#[test]
fn selected_character_shows_delete_button_with_empty_text_and_icon_child() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let delete_id = reg.get_by_name("DeleteChar").expect("DeleteChar");
    let delete_frame = reg.get(delete_id).expect("delete frame");
    let Some(WidgetData::Button(button)) = delete_frame.widget_data.as_ref() else {
        panic!("DeleteChar should be a button");
    };
    assert_eq!(button.text, "");

    let icon_id = reg.get_by_name("DeleteCharIcon").expect("DeleteCharIcon");
    let icon_frame = reg.get(icon_id).expect("icon frame");
    assert_eq!(icon_frame.parent_id, Some(delete_id));

    let Some(WidgetData::Texture(icon_texture)) = icon_frame.widget_data.as_ref() else {
        panic!("DeleteCharIcon should be a texture");
    };
    assert!(
        matches!(
            &icon_texture.source,
            game_engine::ui::widgets::texture::TextureSource::File(path)
                if path == "data/ui/delete-trash-icon-gold.ktx2"
        ),
        "DeleteCharIcon should point at the generated trash icon texture"
    );
}

#[test]
fn no_selection_hides_delete_button_and_icon() {
    let reg = build_screen(CharSelectState::default());
    assert!(reg.get_by_name("DeleteChar").is_none());
    assert!(reg.get_by_name("DeleteCharIcon").is_none());
}

#[test]
fn delete_confirmation_modal_requires_timer_and_phrase() {
    let reg = build_screen_with_delete_confirm(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        },
        DeleteConfirmUiState {
            visible: true,
            character_name: "Elara".to_string(),
            typed_text: "DEL".to_string(),
            countdown_text: "Delete unlocks in 2s".to_string(),
            confirm_enabled: false,
        },
    );

    assert!(reg.get_by_name("DeleteCharacterDialog").is_some());
    assert!(reg.get_by_name("DeleteCharacterConfirmInput").is_some());

    let confirm = reg
        .get(
            reg.get_by_name("DeleteCharacterConfirmButton")
                .expect("confirm button"),
        )
        .expect("confirm frame");
    let Some(WidgetData::Button(button)) = confirm.widget_data.as_ref() else {
        panic!("DeleteCharacterConfirmButton should be a button");
    };
    assert_eq!(button.state, ButtonState::Disabled);

    let countdown = reg
        .get(
            reg.get_by_name("DeleteCharacterDialogCountdown")
                .expect("countdown"),
        )
        .expect("countdown frame");
    let Some(WidgetData::FontString(text)) = countdown.widget_data.as_ref() else {
        panic!("DeleteCharacterDialogCountdown should be a fontstring");
    };
    assert_eq!(text.text, "Delete unlocks in 2s");
}

#[test]
fn delete_confirmation_modal_enables_confirm_after_phrase_and_timer() {
    let reg = build_screen_with_delete_confirm(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        },
        DeleteConfirmUiState {
            visible: true,
            character_name: "Elara".to_string(),
            typed_text: "DELETE".to_string(),
            countdown_text: "Ready. Press Delete Forever to remove this character.".to_string(),
            confirm_enabled: true,
        },
    );

    let confirm = reg
        .get(
            reg.get_by_name("DeleteCharacterConfirmButton")
                .expect("confirm button"),
        )
        .expect("confirm frame");
    let Some(WidgetData::Button(button)) = confirm.widget_data.as_ref() else {
        panic!("DeleteCharacterConfirmButton should be a button");
    };
    assert!(button.enabled);

    let input = reg
        .get(
            reg.get_by_name("DeleteCharacterConfirmInput")
                .expect("confirm input"),
        )
        .expect("input frame");
    let Some(WidgetData::EditBox(editbox)) = input.widget_data.as_ref() else {
        panic!("DeleteCharacterConfirmInput should be an editbox");
    };
    assert_eq!(editbox.text, "DELETE");
}

#[test]
fn delete_confirmation_modal_keeps_confirm_disabled_until_timer_and_phrase_are_both_satisfied() {
    let delete_target = Some(DeleteCharacterTarget {
        character_id: 1,
        name: "Elara".to_string(),
    });

    let timer_locked = super::build_delete_confirm_ui_state(
        &DeleteCharacterConfirmationState {
            target: delete_target.clone(),
            typed_text: "DELETE".to_string(),
            elapsed_secs: super::DELETE_CONFIRM_DELAY_SECS - 0.1,
        },
        &CharSelectFocus(None),
    );
    assert!(
        !timer_locked.confirm_enabled,
        "matching the phrase alone must not enable delete before the timer elapses"
    );
    let timer_locked_reg = build_screen_with_delete_confirm(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        },
        timer_locked,
    );
    let timer_locked_confirm = timer_locked_reg
        .get(
            timer_locked_reg
                .get_by_name("DeleteCharacterConfirmButton")
                .expect("confirm button"),
        )
        .expect("confirm frame");
    let Some(WidgetData::Button(timer_locked_button)) = timer_locked_confirm.widget_data.as_ref()
    else {
        panic!("DeleteCharacterConfirmButton should be a button");
    };
    assert_eq!(timer_locked_button.state, ButtonState::Disabled);

    let phrase_locked = super::build_delete_confirm_ui_state(
        &DeleteCharacterConfirmationState {
            target: delete_target,
            typed_text: "DEL".to_string(),
            elapsed_secs: super::DELETE_CONFIRM_DELAY_SECS,
        },
        &CharSelectFocus(None),
    );
    assert!(
        !phrase_locked.confirm_enabled,
        "elapsed timer alone must not enable delete without the full phrase"
    );
    let phrase_locked_reg = build_screen_with_delete_confirm(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        },
        phrase_locked,
    );
    let phrase_locked_confirm = phrase_locked_reg
        .get(
            phrase_locked_reg
                .get_by_name("DeleteCharacterConfirmButton")
                .expect("confirm button"),
        )
        .expect("confirm frame");
    let Some(WidgetData::Button(phrase_locked_button)) = phrase_locked_confirm.widget_data.as_ref()
    else {
        panic!("DeleteCharacterConfirmButton should be a button");
    };
    assert_eq!(phrase_locked_button.state, ButtonState::Disabled);
}

#[test]
fn character_cards_use_tinted_atlas_textures_without_css_border() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let card_id = reg.get_by_name("CharCard_0").expect("CharCard_0");
    let card = reg.get(card_id).expect("card frame");
    assert!(
        card.border.is_none(),
        "card should rely on atlas art, not CSS border"
    );

    let backdrop_id = reg
        .get_by_name("CharCard_0Backdrop")
        .expect("CharCard_0Backdrop");
    let backdrop = reg.get(backdrop_id).expect("backdrop frame");
    let Some(WidgetData::Texture(backdrop_tex)) = backdrop.widget_data.as_ref() else {
        panic!("backdrop should be a texture");
    };
    assert_eq!(backdrop_tex.vertex_color, [0.76, 0.70, 0.57, 0.96]);

    let selected_id = reg
        .get_by_name("CharCard_0Selected")
        .expect("CharCard_0Selected");
    let selected_frame = reg.get(selected_id).expect("selected frame");
    let Some(WidgetData::Texture(selected_tex)) = selected_frame.widget_data.as_ref() else {
        panic!("selected highlight should be a texture");
    };
    assert_eq!(selected_tex.vertex_color, [0.82, 0.74, 0.46, 0.9]);
}

#[test]
fn screen_does_not_include_inline_create_panel() {
    let reg = build_screen(CharSelectState::default());
    assert!(reg.get_by_name("CreatePanel").is_none());
}

#[test]
fn character_list_backdrop_uses_atlas_slice_metadata() {
    let ns = atlas_nine_slice("glues-characterselect-card-all-bg", 386.0, 520.0)
        .expect("atlas-backed nine-slice");
    assert_eq!(ns.uv_edge_sizes, Some([14.0, 11.0, 14.0, 17.0]));
    let display = ns.edge_sizes.expect("display edge sizes");
    assert_eq!(display, [14.0, 11.0, 14.0, 17.0]);
}

#[test]
fn delete_confirmation_modal_is_centered_on_screen_when_visible() {
    let reg = build_screen_with_delete_confirm_real_layout(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            selected_name: "Elara".to_string(),
            ..Default::default()
        },
        DeleteConfirmUiState {
            visible: true,
            character_name: "Elara".to_string(),
            typed_text: "DEL".to_string(),
            countdown_text: "Delete unlocks in 2s".to_string(),
            confirm_enabled: false,
        },
    );

    let dialog = reg
        .get_by_name("DeleteCharacterDialog")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("DeleteCharacterDialog layout_rect");
    let center_x = dialog.x + dialog.width * 0.5;
    let center_y = dialog.y + dialog.height * 0.5;

    assert!(
        (center_x - 960.0).abs() <= 10.0,
        "expected delete dialog centered near x=960, got {center_x}"
    );
    assert!(
        (center_y - 540.0).abs() <= 20.0,
        "expected delete dialog centered near y=540, got {center_y}"
    );
}
