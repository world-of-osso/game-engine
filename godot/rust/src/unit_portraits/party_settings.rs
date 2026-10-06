//! Offline full Options HUD view, using the same policy and screen as GameClient.
use game_engine_core::client_options_data::{
    CameraOptionsFile, GraphicsOptionsFile, HudOptionsFile, SoundOptionsFile,
};
use game_engine_ui_model::game_menu_component::{GameMenuView, GameMenuViewModel};
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, OptionsCategory};
use game_engine_ui_model::options_menu_data as policy;

pub(super) fn options_view(layout: LayoutOptionsView) -> GameMenuViewModel {
    let graphics = policy::graphics_draft_from_file(&GraphicsOptionsFile::default());
    let sound = policy::sound_draft_from_file(&SoundOptionsFile::default());
    let camera = policy::camera_draft_from_file(&CameraOptionsFile::default());
    let hud = policy::hud_draft_from_file(&HudOptionsFile::default());
    policy::build_view_model(&policy::OptionsModel {
        logged_in: true,
        view: GameMenuView::Options,
        category: OptionsCategory::Hud,
        modal_position: [0.0, 0.0],
        draft_graphics: graphics.clone(),
        committed_graphics: graphics,
        draft_sound: sound.clone(),
        committed_sound: sound,
        draft_camera: camera.clone(),
        committed_camera: camera,
        draft_hud: hud.clone(),
        committed_hud: hud,
        draft_bindings: Default::default(),
        committed_bindings: Default::default(),
        binding_section: game_engine_core::input_bindings_data::BindingSection::Movement,
        binding_capture: policy::BindingCapture::None,
        layout,
    })
}
