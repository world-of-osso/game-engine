//! Source pairs: data/skyborn-handoff/2026-10-07-c1cdf6e/engine/db2/1.60.1.70205/
//! CharBaseInfo.csv rows 888,889,898,899,900 / 891,892,893,901,903.
//! Retail Blizzard_CharacterCreate.lua:951-965 gates class buttons by classData.enabled;
//! :696-709 submits the selected name through C_CharacterCreation.CreateCharacter.

use game_engine_ui_model::CharacterCreateModel;
use game_engine_ui_model::char_create_component::{CREATE_BUTTON, CREATE_NAME_INPUT, NEXT_BUTTON};
use shared::protocol::CreateCharacter;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;

use super::super::{CharCreateEffect, CharCreateState, build_ui_state, reduce};
use super::{assert_required_choices_met, data_root, db};
use game_engine_ui_model::char_create_component::CharCreateAction;

fn sync_model(model: &mut CharacterCreateModel, state: &CharCreateState) {
    model.shared.insert(build_ui_state(state, db()));
    model.sync();
}

fn button_action(model: &CharacterCreateModel, name: &str) -> Option<CharCreateAction> {
    let id = model.registry.get_by_name(name).expect(name);
    let frame = model.registry.get(id).unwrap();
    frame.onclick.as_deref().and_then(CharCreateAction::parse)
}

fn press_button(
    model: &mut CharacterCreateModel,
    state: &mut CharCreateState,
    name: &str,
) -> Vec<CharCreateEffect> {
    let action = button_action(model, name).expect("enabled authored button");
    let name = state.name.clone();
    let effects = reduce(state, action, db(), Err("unused names"), &name, 17);
    sync_model(model, state);
    effects
}

fn assert_class_gates(model: &CharacterCreateModel, race: u8) {
    let allowed: &[u8] = match race {
        95 => &[1, 3, 4, 8, 11],
        96 => &[1, 3, 4, 7, 11],
        _ => panic!("not a Forever fixture race"),
    };
    for class in 1..=13 {
        let action = button_action(model, &format!("Class_{class}"));
        let expected = allowed
            .contains(&class)
            .then_some(CharCreateAction::SelectClass(class));
        assert_eq!(action, expected, "race {race} class {class}");
    }
}

fn assert_request_choices(request: &CreateCharacter) {
    let selected = CharCreateState {
        selected_race: request.race,
        selected_class: request.class,
        selected_sex: request.appearance.sex,
        appearance: request.appearance.clone(),
        ..Default::default()
    };
    let options = db()
        .options_for(request.race, request.appearance.sex)
        .unwrap();
    assert_eq!(options.len(), 18 + usize::from(request.appearance.sex));
    let offered_options = super::super::customization_view::offered_options(&selected, db());
    assert_eq!(offered_options.len(), options.len() - 1);
    for option in offered_options {
        let choice = crate::appearance_options::selected_choice(
            db(),
            request.race,
            request.appearance.sex,
            request.class,
            &request.appearance,
            option,
        )
        .expect("offered option has a selected core or additional choice");
        if !crate::appearance_options::is_core_option(
            db(),
            request.race,
            request.appearance.sex,
            option,
        ) {
            assert!(
                request
                    .appearance
                    .customization_choices
                    .iter()
                    .any(|selection| selection.option_id == option.id
                        && selection.choice_id == choice.id),
                "offered additional option is explicit in the request"
            );
        }
        let offered = db().offered_choices(
            request.race,
            request.appearance.sex,
            request.class,
            option.id,
        );
        assert!(
            offered.iter().any(|candidate| candidate.id == choice.id),
            "race {} class {} sex {} option {} choice {}",
            request.race,
            request.class,
            request.appearance.sex,
            option.id,
            choice.id
        );
    }
    for choice in &request.appearance.customization_choices {
        let option = options
            .iter()
            .find(|option| option.id == choice.option_id)
            .expect("extra option belongs to selected race/body type");
        let offered = db().offered_choices(
            request.race,
            request.appearance.sex,
            request.class,
            option.id,
        );
        assert!(
            offered
                .iter()
                .any(|candidate| candidate.id == choice.choice_id)
        );
    }
    assert_required_choices_met(&selected, "outgoing Forever request");
    assert!(request.appearance.visage.is_none());
}

fn prove_pair(race: u8, class: u8) {
    game_engine_ui_model::paths::set_data_root(data_root()).unwrap();
    set_thread_skin(ActiveSkin::Forever);
    for sex in [0, 1] {
        let mut model = CharacterCreateModel::new(1920.0, 1080.0);
        model.shared.insert(ActiveSkin::Forever);
        let mut state = CharCreateState::default();
        sync_model(&mut model, &state);
        press_button(&mut model, &mut state, &format!("Race_{race}"));
        assert_eq!(state.selected_race, race);
        assert_eq!(state.selected_class, if race == 95 { 8 } else { 7 });
        assert_class_gates(&model, race);
        press_button(&mut model, &mut state, &format!("Class_{class}"));
        press_button(&mut model, &mut state, &format!("CharCreateSex_{sex}"));
        assert_eq!((state.selected_class, state.selected_sex), (class, sex));
        // Disabled UI actions cannot mutate even if supplied directly to the reducer.
        for invalid in [2, 5, 6, 9, 10, 12, 13, if race == 95 { 7 } else { 8 }] {
            let before = state.appearance.clone();
            let effects = reduce(
                &mut state,
                CharCreateAction::SelectClass(invalid),
                db(),
                Err("unused names"),
                "",
                29,
            );
            assert!(effects.is_empty());
            assert_eq!(state.selected_class, class);
            assert_eq!(state.appearance, before);
        }
        press_button(&mut model, &mut state, NEXT_BUTTON.0);
        state.name = "Skyproof".into();
        sync_model(&mut model, &state);
        let id = model.registry.get_by_name(CREATE_NAME_INPUT.0).unwrap();
        let Some(WidgetData::EditBox(edit)) = &model.registry.get(id).unwrap().widget_data else {
            panic!("creation name input");
        };
        assert_eq!(edit.text, "Skyproof");
        let effects = press_button(&mut model, &mut state, CREATE_BUTTON.0);
        let requests: Vec<_> = effects
            .into_iter()
            .filter_map(|effect| match effect {
                CharCreateEffect::SendCreate(request) => Some(request),
                _ => None,
            })
            .collect();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(
            (request.race, request.class, request.appearance.sex),
            (race, class, sex)
        );
        assert_eq!(request.name, "Skyproof");
        assert_eq!(request.appearance, state.appearance);
        assert_request_choices(request);
        println!("FOREVER CREATE PASS race={race} class={class} sex={sex}");
    }
    set_thread_skin(ActiveSkin::Modern);
}

macro_rules! pair_test {
    ($name:ident, $race:literal, $class:literal) => {
        #[test]
        fn $name() {
            prove_pair($race, $class);
        }
    };
}
pair_test!(high_order_warrior, 95, 1);
pair_test!(high_order_hunter, 95, 3);
pair_test!(high_order_rogue, 95, 4);
pair_test!(high_order_mage, 95, 8);
pair_test!(high_order_druid, 95, 11);
pair_test!(windshaper_warrior, 96, 1);
pair_test!(windshaper_hunter, 96, 3);
pair_test!(windshaper_rogue, 96, 4);
pair_test!(windshaper_shaman, 96, 7);
pair_test!(windshaper_druid, 96, 11);
