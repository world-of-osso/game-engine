use super::*;

#[test]
fn campsite_tab_is_anchored_to_top_center_without_offsets() {
    use game_engine::ui::anchor::AnchorPoint;
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let bar_id = reg.get_by_name("CampsiteMenuBar").expect("CampsiteMenuBar");
    let tab_id = reg.get_by_name("CampsiteTab").expect("CampsiteTab");
    assert_single_anchor(
        &reg,
        bar_id,
        AnchorPoint::Top,
        AnchorPoint::Top,
        Some(root_id),
        0.0,
        0.0,
    );
    assert_single_anchor(
        &reg,
        tab_id,
        AnchorPoint::TopLeft,
        AnchorPoint::TopLeft,
        Some(bar_id),
        357.0,
        -1.0,
    );
}

#[test]
fn campsite_panel_is_anchored_to_top_center_without_offsets() {
    use game_engine::ui::anchor::AnchorPoint;
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let panel_id = reg.get_by_name("CampsitePanel").expect("CampsitePanel");
    assert_single_anchor(
        &reg,
        panel_id,
        AnchorPoint::Top,
        AnchorPoint::Top,
        Some(root_id),
        0.0,
        -58.0,
    );
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
