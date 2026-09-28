use super::*;

#[test]
fn top_nav_keeps_top_center_position_with_campsites_tab_last() {
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let nav_id = reg
        .get_by_name("CharSelectTopNav")
        .expect("CharSelectTopNav");
    assert_top_edge_centered(&reg, nav_id, Some(root_id), 10.0);
    let right_edge = |name: &str| {
        let rect = reg
            .get(reg.get_by_name(name).expect(name))
            .and_then(|frame| frame.layout_rect.clone())
            .expect("tab layout");
        rect.x + rect.width
    };
    assert!(right_edge("CharSelectRealmsTab") < right_edge("CharSelectCampsitesTab"));
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

    let panel = reg
        .get(reg.get_by_name("CampsitePanel").expect("CampsitePanel"))
        .expect("panel");
    let card = reg
        .get(reg.get_by_name("CampsiteScene_1").expect("CampsiteScene_1"))
        .expect("card");

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

fn campsites_tab_look(reg: &FrameRegistry) -> (bool, [f32; 4]) {
    let boxed = !reg
        .get(
            reg.get_by_name("CharSelectCampsitesTabBox")
                .expect("tab box"),
        )
        .expect("tab box frame")
        .hidden;
    let label = reg
        .get(
            reg.get_by_name("CharSelectCampsitesTabLabel")
                .expect("tab label"),
        )
        .expect("tab label frame");
    let Some(WidgetData::FontString(font)) = label.widget_data.as_ref() else {
        panic!("CharSelectCampsitesTabLabel should be a fontstring");
    };
    (boxed, font.color)
}

#[test]
fn open_campsite_panel_boxes_campsites_tab_with_white_label() {
    let reg = build_screen_with_campsites_real_layout(
        CharSelectState::default(),
        one_scene_campsite_state(),
    );
    assert_eq!(campsites_tab_look(&reg), (true, [1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn closed_campsite_panel_leaves_campsites_tab_gold_and_unboxed() {
    let mut campsite = one_scene_campsite_state();
    campsite.panel_visible = false;
    let reg = build_screen_with_campsites_real_layout(CharSelectState::default(), campsite);
    assert_eq!(campsites_tab_look(&reg), (false, [1.0, 0.82, 0.0, 1.0]));
}
