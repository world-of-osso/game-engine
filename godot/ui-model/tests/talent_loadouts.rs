use game_engine_ui_model::{
    spellbook_frame_component::{PlayerSpellsTab, spellbook_frame_screen},
    spellbook_preview::{load_preview_state, stage_talent_preview},
    talents::{LoadoutDialog, TalentEditor},
};
use shared::protocol::{TraitLoadoutInfo, TraitLoadoutOperation, TraitLoadoutsSnapshot};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};
fn state() -> game_engine_ui_model::spellbook_frame_component::SpellbookFrameState {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(data.clone()).unwrap();
    let mut state = load_preview_state(&data, PlayerSpellsTab::Talents).unwrap();
    let spec_id = state
        .talents
        .as_ref()
        .unwrap()
        .editor
        .snapshot
        .as_ref()
        .unwrap()
        .spec_id;
    state
        .talents
        .as_mut()
        .unwrap()
        .editor
        .receive_loadouts(list(spec_id, 1));
    state
}
fn list(spec_id: u32, selected_id: u32) -> TraitLoadoutsSnapshot {
    TraitLoadoutsSnapshot {
        spec_id,
        selected_id,
        configs: vec![
            TraitLoadoutInfo {
                id: 1,
                name: "Raid".into(),
            },
            TraitLoadoutInfo {
                id: 2,
                name: "Dungeons".into(),
            },
            TraitLoadoutInfo {
                id: 3,
                name: "Solo".into(),
            },
        ],
    }
}
#[test]
fn talent_loadouts_create_copies_pending_allocation_and_waits_for_server_selection() {
    let mut state = state();
    stage_talent_preview(&mut state).unwrap();
    let view = state.talents.as_ref().unwrap();
    let mut editor = view.editor.clone();
    let spec_id = view.editor.snapshot.as_ref().unwrap().spec_id;
    let entries = editor.apply().unwrap().entries;
    editor.action(view, 80, "talent:loadout_new").unwrap();
    editor.loadout_name = "My Build".into();
    editor.action(view, 80, "talent:loadout_save").unwrap();
    let request = editor.take_loadout_request().unwrap();
    assert_eq!(request.spec_id, spec_id);
    assert_eq!(
        request.operation,
        TraitLoadoutOperation::Create {
            name: "My Build".into(),
            entries
        }
    );
    assert_eq!(editor.loadout_caption(), "Raid");
    assert!(editor.loadout_busy);
    assert!(editor.take_loadout_request().is_none());
    editor.receive_loadouts(TraitLoadoutsSnapshot {
        spec_id,
        selected_id: 4,
        configs: vec![TraitLoadoutInfo {
            id: 4,
            name: "My Build".into(),
        }],
    });
    assert_eq!(editor.loadout_caption(), "My Build");
    assert!(!editor.loadout_busy);
}
#[test]
fn talent_loadouts_rename_delete_and_dirty_switch_require_correct_confirmations() {
    let mut state = state();
    stage_talent_preview(&mut state).unwrap();
    let view = state.talents.as_ref().unwrap();
    let mut editor = view.editor.clone();
    editor.action(view, 80, "talent:loadout_select:2").unwrap();
    assert_eq!(editor.loadout_dialog, Some(LoadoutDialog::Switch(2)));
    assert!(editor.take_loadout_request().is_none());
    editor.action(view, 80, "talent:loadout_cancel").unwrap();
    assert!(editor.dirty());
    editor.action(view, 80, "talent:loadout_edit:3").unwrap();
    assert_eq!(editor.loadout_name, "Solo");
    editor.loadout_name = "".into();
    editor.action(view, 80, "talent:loadout_save").unwrap();
    assert!(editor.error_text.is_some());
    assert!(editor.take_loadout_request().is_none());
    editor.loadout_name = "Questing".into();
    editor.action(view, 80, "talent:loadout_save").unwrap();
    assert_eq!(
        editor.take_loadout_request().unwrap().operation,
        TraitLoadoutOperation::Rename {
            id: 3,
            name: "Questing".into()
        }
    );
    editor.receive_loadouts(list(view.editor.snapshot.as_ref().unwrap().spec_id, 1));
    assert!(
        editor.dirty(),
        "Renaming metadata must preserve unapplied talent edits"
    );
    editor.action(view, 80, "talent:loadout_edit:3").unwrap();
    editor.action(view, 80, "talent:loadout_delete").unwrap();
    assert_eq!(editor.loadout_dialog, Some(LoadoutDialog::Delete(3)));
    assert!(editor.take_loadout_request().is_none());
    editor.action(view, 80, "talent:loadout_confirm").unwrap();
    assert_eq!(
        editor.take_loadout_request().unwrap().operation,
        TraitLoadoutOperation::Delete { id: 3 }
    );
}
#[test]
fn talent_loadouts_both_skins_render_three_server_rows_and_selected_caption() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = state();
        state.talents.as_mut().unwrap().editor.loadout_menu = true;
        let mut shared = SharedContext::new();
        shared.insert(state);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
        let Some(WidgetData::FontString(text)) = &registry
            .get(registry.get_by_name("TalentLoadoutName").unwrap())
            .unwrap()
            .widget_data
        else {
            panic!("caption")
        };
        assert_eq!(text.text, "Raid");
        for (id, name) in [(1, "Raid"), (2, "Dungeons"), (3, "Solo")] {
            let Some(WidgetData::FontString(text)) = &registry
                .get(
                    registry
                        .get_by_name(&format!("TalentLoadoutRow{id}Text"))
                        .unwrap(),
                )
                .unwrap()
                .widget_data
            else {
                panic!("row")
            };
            assert_eq!(text.text, name);
            assert!(
                registry
                    .get_by_name(&format!("TalentLoadoutEdit{id}"))
                    .is_some()
            );
        }
        assert!(registry.get_by_name("TalentLoadoutRow1Check").is_some());
        assert!(registry.get_by_name("TalentLoadoutRow2Check").is_none());
        let Some(WidgetData::Button(button)) = &registry
            .get(registry.get_by_name("TalentNewLoadout").unwrap())
            .unwrap()
            .widget_data
        else {
            panic!("new")
        };
        assert_ne!(
            button.state,
            ui_toolkit::widgets::button::ButtonState::Disabled
        );
        assert_eq!(
            button.normal_texture, None,
            "Retail dropdown sentinels draw flat menu text, not panel button art"
        );
        assert!(registry.get_by_name("TalentStarterBuild").is_none());
    }
}
#[test]
fn talent_loadouts_modern_dialog_draws_the_mainline_panel_button_sheets() {
    set_thread_skin(ActiveSkin::Modern);
    let mut state = state();
    let editor = &mut state.talents.as_mut().unwrap().editor;
    editor.loadout_dialog = Some(LoadoutDialog::Edit(1));
    editor.loadout_name = "Raid".into();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(spellbook_frame_screen).sync(&shared, &mut registry);
    game_engine_ui_model::spellbook_frame_component::apply_spellbook_postsetup(
        &state,
        &mut registry,
    );
    for name in [
        "TalentLoadoutSave",
        "TalentLoadoutDelete",
        "TalentLoadoutCancel",
    ] {
        let Some(WidgetData::Button(button)) = &registry
            .get(registry.get_by_name(name).unwrap())
            .unwrap()
            .widget_data
        else {
            panic!("dialog button")
        };
        assert_eq!(
            button.normal_texture,
            Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(
                130828
            ))
        );
        assert_eq!(
            button.pushed_texture,
            Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(
                130825
            ))
        );
        assert_eq!(
            button.disabled_texture,
            Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(
                130824
            ))
        );
    }
}

#[test]
fn talent_loadouts_ignore_list_from_other_spec() {
    let state = state();
    let view = state.talents.as_ref().unwrap();
    let mut editor: TalentEditor = view.editor.clone();
    editor.receive_loadouts(list(70, 2));
    assert_eq!(editor.loadout_caption(), "Raid");
}
