use super::*;

#[test]
fn campsite_tab_keeps_top_center_position() {
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let bar_id = reg.get_by_name("CampsiteMenuBar").expect("CampsiteMenuBar");
    let tab_id = reg.get_by_name("CampsiteTab").expect("CampsiteTab");
    assert_top_edge_centered(&reg, bar_id, Some(root_id), 0.0);
    assert_bounds_offset_from_top_left(&reg, tab_id, Some(bar_id), 357.0, 1.0);
}

#[test]
fn campsite_panel_keeps_top_center_position() {
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let panel_id = reg.get_by_name("CampsitePanel").expect("CampsitePanel");
    assert_top_edge_centered(&reg, panel_id, Some(root_id), 58.0);
}

#[test]
fn campsite_overlay_renders_in_dialog_strata() {
    let reg = build_screen_with_campsites(
        CharSelectState {
            selected_index: Some(0),
            selected_name: "Elara".to_string(),
            ..Default::default()
        },
        one_scene_campsite_state(),
    );

    let menu_bar = reg
        .get(reg.get_by_name("CampsiteMenuBar").expect("CampsiteMenuBar"))
        .expect("menu bar");
    let panel = reg
        .get(reg.get_by_name("CampsitePanel").expect("CampsitePanel"))
        .expect("panel");
    let card = reg
        .get(reg.get_by_name("CampsiteScene_1").expect("CampsiteScene_1"))
        .expect("card");

    assert_eq!(menu_bar.strata, FrameStrata::Dialog);
    assert_eq!(panel.strata, FrameStrata::Dialog);
    assert_eq!(card.strata, FrameStrata::Dialog);
}

#[test]
fn campsite_panel_does_not_overlap_character_cards() {
    let reg = build_screen_with_campsites_real_layout(
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
        one_scene_campsite_state(),
    );

    let panel = reg
        .get_by_name("CampsitePanel")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("CampsitePanel layout_rect");
    let character_card = reg
        .get_by_name("CharCard_0")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("CharCard_0 layout_rect");

    let panel_right = panel.x + panel.width;
    let card_left = character_card.x;

    assert!(
        panel_right <= card_left,
        "expected CampsitePanel to stay left of CharCard_0, got panel_right={} card_left={}",
        panel_right,
        card_left
    );
}

#[test]
fn campsite_tab_selected_uses_gold_label_and_underline() {
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());

    assert!(reg.get_by_name("CampsiteTabUnderline").is_some());
    let label = reg
        .get(
            reg.get_by_name("CampsiteTabLabel")
                .expect("CampsiteTabLabel"),
        )
        .expect("CampsiteTabLabel frame");
    let Some(WidgetData::FontString(font)) = label.widget_data.as_ref() else {
        panic!("CampsiteTabLabel should be a fontstring");
    };
    assert_eq!(font.color, [1.0, 0.82, 0.0, 1.0]);
}

#[test]
fn campsite_tab_unselected_uses_subtitle_label_and_hides_underline() {
    let mut campsite = one_scene_campsite_state();
    campsite.panel_visible = false;
    let reg = build_screen_with_campsites(CharSelectState::default(), campsite);

    assert!(reg.get_by_name("CampsiteTabUnderline").is_none());
    let label = reg
        .get(
            reg.get_by_name("CampsiteTabLabel")
                .expect("CampsiteTabLabel"),
        )
        .expect("CampsiteTabLabel frame");
    let Some(WidgetData::FontString(font)) = label.widget_data.as_ref() else {
        panic!("CampsiteTabLabel should be a fontstring");
    };
    assert_eq!(font.color, [0.92, 0.88, 0.74, 1.0]);
}
