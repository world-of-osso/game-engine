//! Dedicated edit-mode production UI and offline capture entrypoints.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::hud_edit_component::*;
use godot::prelude::*;
use ui_toolkit::{atlas::ActiveSkin, screen::SharedContext, widget_def::Element};

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
    shared.insert(
        ctx.get::<game_engine_ui_model::hud_edit::EditModeActive>()
            .copied()
            .unwrap_or_default(),
    );
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
