//! Regression boundary: the same model, native Taffy layout and scroll input as RegistryUi.
use std::collections::HashMap;

use game_engine_ui_model::CharacterSelectModel;
use game_engine_ui_model::char_select_component::{CharDisplayEntry, CharSelectState};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout::LayoutRect;

use super::{RegistryModel, ScreenPostsetup, layout::compute_layout_with_intrinsics};

const LIST: &str = "CharacterListCards";

fn roster(count: usize, selected: usize) -> CharSelectState {
    CharSelectState {
        characters: (0..count)
            .map(|i| CharDisplayEntry {
                name: format!("Skyborne{}", i + 1),
                info: "Level 1 Windshaper Skyborne Druid".into(),
                status: "Ready to enter world".into(),
            })
            .collect(),
        selected_index: Some(selected),
        selected_name: format!("Skyborne{}", selected + 1),
        status_text: String::new(),
    }
}

fn model(count: usize, skin: ActiveSkin) -> RegistryModel {
    set_thread_skin(skin);
    let CharacterSelectModel {
        screen,
        mut shared,
        registry,
    } = CharacterSelectModel::new(1920.0, 1080.0);
    shared.insert(roster(count, 0));
    let mut model = RegistryModel {
        screen,
        shared,
        registry,
        postsetup: ScreenPostsetup::CharacterSelect,
        icon_masks: Default::default(),
    };
    rebuild(&mut model);
    model
}

fn rebuild(model: &mut RegistryModel) {
    model.sync();
    let bounds = compute_layout_with_intrinsics(&model.registry, &HashMap::new()).unwrap();
    for (id, rect) in bounds {
        model.registry.set_computed_layout(id, rect).unwrap();
    }
}

fn rect(model: &RegistryModel, name: &str) -> LayoutRect {
    let id = model.registry.get_by_name(name).unwrap();
    model.registry.get(id).unwrap().layout_rect.clone().unwrap()
}

fn intersects(a: &LayoutRect, b: &LayoutRect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn check_height(count: usize) {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let model = model(count, skin);
        for i in 0..count {
            let card = rect(&model, &format!("CharCard_{i}"));
            assert_eq!(card.height, 95.0, "{skin:?} card{i}: {card:?}");
            if i > 0 {
                let previous = rect(&model, &format!("CharCard_{}", i - 1));
                assert!(card.y >= previous.y + previous.height);
            }
        }
    }
}

fn check_viewport_and_footer(count: usize) {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let model = model(count, skin);
        let list = rect(&model, LIST);
        let panel = rect(&model, "CharacterListPanel");
        assert!(list.y >= panel.y && list.y + list.height <= panel.y + panel.height);
        for footer in ["CreateChar", "DeleteChar"] {
            let footer = rect(&model, footer);
            assert!(
                !intersects(&list, &footer),
                "{skin:?}: viewport {list:?} overlaps footer {footer:?}"
            );
            for i in 0..count {
                let card = rect(&model, &format!("CharCard_{i}"));
                // Native ScrollBox clips partial rows. Only their intersection is visible.
                if intersects(&card, &list) {
                    let visible = LayoutRect {
                        y: card.y.max(list.y),
                        height: (card.y + card.height).min(list.y + list.height)
                            - card.y.max(list.y),
                        ..card
                    };
                    assert!(
                        visible.y >= list.y && visible.y + visible.height <= list.y + list.height
                    );
                    assert!(!intersects(&visible, &footer));
                }
            }
        }
    }
}

fn check_selection(count: usize) {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut model = model(count, skin);
        // Production Down-key handler updates the selected index through step_selection.
        let mut selected = Some(0);
        for _ in 0..9 {
            selected =
                game_engine_ui_model::char_select_component::step_selection(selected, count, true);
            model.shared.insert(roster(count, selected.unwrap()));
            rebuild(&mut model);
        }
        let list = rect(&model, LIST);
        let tenth = rect(&model, "CharCard_9");
        let offset = model
            .registry
            .scroll_lists
            .get(LIST)
            .expect("scrollable roster")
            .first_row;
        assert!(offset > 0);
        assert!(
            tenth.y >= list.y && tenth.y + tenth.height <= list.y + list.height,
            "{skin:?}: card10 {tenth:?} viewport {list:?}"
        );
        model.shared.insert(roster(count, 0));
        rebuild(&mut model);
        assert_eq!(model.registry.scroll_lists.get(LIST).unwrap().first_row, 0);
    }
}

#[test]
fn rosterfix_ten_fixed_height() {
    check_height(10);
}
#[test]
fn rosterfix_twelve_fixed_height() {
    check_height(12);
}
#[test]
fn rosterfix_ten_viewport_footer() {
    check_viewport_and_footer(10);
}
#[test]
fn rosterfix_twelve_viewport_footer() {
    check_viewport_and_footer(12);
}
#[test]
fn rosterfix_ten_selection_visible() {
    check_selection(10);
}
#[test]
fn rosterfix_twelve_selection_visible() {
    check_selection(12);
}

#[test]
fn rosterfix_protocol_names_render_as_text() {
    use shared::protocol::CharacterListEntry;
    let characters = [CharacterListEntry {
        character_id: 30,
        name: "Skywinddru".into(),
        level: 1,
        race: 96,
        class: 11,
        appearance: Default::default(),
        equipment_appearance: Default::default(),
    }];
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let names = game_engine_ui_model::char_select_data::CharSelectNames::load(&root).unwrap();
    let state =
        game_engine_ui_model::char_select_state_from_roster(&characters, Some(0), &names).unwrap();
    let mut model = model(10, ActiveSkin::Modern);
    model.shared.insert(state);
    rebuild(&mut model);
    let id = model.registry.get_by_name("CharCard_0Info").unwrap();
    let Some(WidgetData::FontString(label)) = &model.registry.get(id).unwrap().widget_data else {
        panic!("missing info text")
    };
    assert_eq!(label.text, "Level 1 Windshaper Skyborne Druid");
}

#[test]
fn rosterfix_wheel_stepper_and_thumb_use_retail_pan_without_reselecting() {
    use super::scroll_lists::{drag_thumbs, press_stepper, press_thumb, release_thumbs, wheel};
    use game_engine_ui_model::minimal_scroll_bar::forward_stepper_name;
    use ui_toolkit::widgets::scroll_list::{thumb_name, track_name};
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut model = model(12, skin);
        assert!(wheel(&mut model, LIST, false));
        rebuild(&mut model);
        assert_eq!(
            model.registry.scroll_lists.get(LIST).unwrap().first_row,
            194
        );
        let forward = model
            .registry
            .get_by_name(&forward_stepper_name(LIST))
            .unwrap();
        assert!(press_stepper(&mut model, forward));
        rebuild(&mut model);
        assert_eq!(
            model.registry.scroll_lists.get(LIST).unwrap().first_row,
            291
        );
        let thumb = model.registry.get_by_name(&thumb_name(LIST)).unwrap();
        let thumb_rect = rect(&model, &thumb_name(LIST));
        assert!(press_thumb(&mut model.registry, thumb, thumb_rect.y));
        let track = rect(&model, &track_name(LIST));
        assert!(drag_thumbs(&mut model.registry, track.y + track.height));
        rebuild(&mut model);
        let state = model.registry.scroll_lists.get(LIST).unwrap();
        assert_eq!(state.first_row, state.geometry.max_first_row());
        assert!(release_thumbs(&mut model.registry));
        let last = rect(&model, "CharCard_11");
        let list = rect(&model, LIST);
        assert_eq!(last.y + last.height, list.y + list.height);
        assert_eq!(
            model
                .shared
                .get::<CharSelectState>()
                .unwrap()
                .selected_index,
            Some(0)
        );
        while wheel(&mut model, LIST, true) {
            rebuild(&mut model);
        }
        assert_eq!(model.registry.scroll_lists.get(LIST).unwrap().first_row, 0);
    }
}
