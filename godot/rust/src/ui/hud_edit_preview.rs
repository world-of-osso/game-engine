//! Dedicated edit-mode production UI and offline capture entrypoints.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::hud_edit_component::*;
use godot::prelude::*;
use ui_toolkit::{atlas::ActiveSkin, screen::SharedContext, widget_def::Element};

pub(super) fn register_edit_panel_style(registry: &mut ui_toolkit::registry::FrameRegistry) {
    super::register_auction_popup_style(registry);
}

fn edit_screen(ctx: &SharedContext) -> Element {
    let mut elements = edit_mode_side_bar_previews(ctx);
    elements.extend(edit_mode_overlay_screen(ctx));
    if ctx.get::<EditModePanelState>().is_some() {
        elements.extend(edit_mode_panel_screen(ctx));
    }
    elements
}

impl RegistryUi {
    pub(crate) fn show_hud_edit(&mut self) -> Result<(), String> {
        self.show_viewport_screen(
            EditModeOverlayState::default(),
            edit_screen,
            ScreenPostsetup::None,
        )?;
        register_edit_panel_style(
            &mut self
                .model
                .as_mut()
                .ok_or("HUD editor registry missing")?
                .registry,
        );
        self.set_state(EditModePanelState::default())
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_hudedit_preview(&mut self) -> GString {
        self.capture_hud_edit(ActiveSkin::Modern)
    }

    #[func]
    pub fn show_forever_hudedit_preview(&mut self) -> GString {
        self.capture_hud_edit(ActiveSkin::Forever)
    }

    /// Clear the offline snapshot's transient state before creating a real GameClient.
    #[func]
    pub fn finish_hudedit_preview(&mut self) {
        super::hud_edit_layout::publish_editor_active(false);
        ui_toolkit::atlas::set_thread_skin(ActiveSkin::Modern);
    }

    fn capture_hud_edit(&mut self, skin: ActiveSkin) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            ui_toolkit::atlas::set_thread_skin(skin);
            super::hud_edit_layout::publish_editor_active(true);
            self.set_ui_scale(1.0)?;
            self.show_viewport_screen(
                EditModeOverlayState::default(),
                preview_screen,
                ScreenPostsetup::None,
            )?;
            register_edit_panel_style(
                &mut self
                    .model
                    .as_mut()
                    .ok_or("HUD preview registry missing")?
                    .registry,
            );
            self.set_state(EditModePanelState {
                layout_name: format!("{skin:?}"),
                preset: true,
                ..Default::default()
            })?;
            let registry = self.registry().ok_or("Preview registry missing")?;
            let boxes =
                super::hud_edit_layout::collect_selection_boxes(registry, Some("player_frame"));
            self.set_state(EditModeOverlayState { boxes })
        })();
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

pub(super) fn preview_screen(ctx: &SharedContext) -> Element {
    use game_engine_ui_model::bags_bar_component::{BagBarState, bags_bar_screen};
    use game_engine_ui_model::buff_frame_component::{BuffFrameState, buff_frame_screen};
    use game_engine_ui_model::chat_frame_component::{ChatFrameView, chat_frame_screen};
    use game_engine_ui_model::damage_meter_component::damage_meter_screen;
    use game_engine_ui_model::damage_meter_data::DamageMeterView;
    use game_engine_ui_model::inworld_unit_frames_component::{
        InWorldUnitFramesState, SmallUnitFrameState, UnitFrameState, inworld_unit_frames_screen,
    };
    use game_engine_ui_model::main_action_bar_component::{
        MainActionBarState, main_action_bar_screen,
    };
    use game_engine_ui_model::minimap::{MinimapClusterState, minimap_cluster_screen};
    use game_engine_ui_model::objective_tracker_component::{
        ObjectiveTrackerState, objective_tracker_screen,
    };
    use game_engine_ui_model::xp_bar_component::{XpBarState, xp_bar_screen};
    let mut shared = SharedContext::new();
    shared.insert(game_engine_ui_model::hud_edit::EditModeActive(true));
    shared.insert(*ctx.get::<ActiveSkin>().expect("preview skin"));
    let target = UnitFrameState::named("Training Dummy");
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        player: UnitFrameState::named("Hudedit"),
        target: Some(target.clone()),
        target_of_target: Some(SmallUnitFrameState::from(&target)),
        focus: Some(SmallUnitFrameState::from(&target)),
        target_cast: None,
        pet: None,
        bosses: Vec::new(),
        menu: Default::default(),
        personal_resource: None,
    });
    shared.insert(MainActionBarState {
        player_class: Some(2),
        ..Default::default()
    });
    shared.insert(BagBarState::default());
    shared.insert(ChatFrameView::default());
    shared.insert(MinimapClusterState {
        zone_text: "Northshire Valley".into(),
        ..Default::default()
    });
    shared.insert(ObjectiveTrackerState::default());
    insert_missing_mover_previews(&mut shared);
    shared.insert(BuffFrameState::default());
    shared.insert(XpBarState {
        xp: 350,
        next_level_xp: 1000,
        rested_xp: 200,
        hovered: false,
    });
    shared.insert(DamageMeterView::default());
    let mut elements: Element = [
        inworld_unit_frames_screen(&shared),
        main_action_bar_screen(&shared),
        bags_bar_screen(&shared),
        chat_frame_screen(&shared),
        minimap_cluster_screen(&shared),
        objective_tracker_screen(&shared),
        buff_frame_screen(&shared),
        xp_bar_screen(&shared),
        damage_meter_screen(&shared),
        game_engine_ui_model::ui_errors_frame_component::ui_errors_frame_screen(&shared),
        game_engine_ui_model::casting_bar_frame_component::casting_bar_frame_screen(&shared),
        game_engine_ui_model::group_frames_component::group_frames_screen(&shared),
        game_engine_ui_model::micro_menu::micro_menu_screen(&shared),
    ]
    .into_iter()
    .flatten()
    .collect();
    elements.extend(edit_mode_side_bar_previews(ctx));
    elements.extend(edit_mode_overlay_screen(ctx));
    if ctx.get::<EditModePanelState>().is_some() {
        elements.extend(edit_mode_panel_screen(ctx));
    }
    elements
}

fn insert_missing_mover_previews(shared: &mut SharedContext) {
    use game_engine_core::ui_layout_data::LayoutSettings;
    use game_engine_ui_model::compact_unit_frame_component::{CompactUnitView, UnitStatus};
    use game_engine_ui_model::group_frames_component::GroupFramesState;
    shared.insert(game_engine_ui_model::ui_errors_data::UiErrorsData::default());
    shared.insert(
        game_engine_ui_model::casting_bar_frame_component::CastingBarState {
            visible: true,
            spell_name: "Flash of Light".into(),
            progress: 0.5,
            ..Default::default()
        },
    );
    let member = CompactUnitView {
        name: "Hudedit".into(),
        class_rgb: [0.96, 0.55, 0.73],
        health_fraction: Some(0.75),
        power: None,
        role: shared::protocol::GroupRoleSnapshot::Healer,
        status: UnitStatus::Online,
        in_range: true,
        selected: false,
        ready: None,
        debuffs: Vec::new(),
    };
    shared.insert(GroupFramesState {
        party: vec![member.clone(); 5],
        raid: vec![vec![member; 5]; 8],
        ..Default::default()
    });
    // Explicit offline inventory: expose the normally hidden micro menu, without changing presets.
    shared.insert(LayoutSettings {
        show_micro_menu: Some(true),
        ..Default::default()
    });
}
