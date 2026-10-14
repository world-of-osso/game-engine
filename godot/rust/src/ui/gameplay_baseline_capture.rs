//! Temporary before capture: 642fcee41 renderers, source dimensions/anchors unchanged.
use super::{RegistryUi, ScreenPostsetup, gameplay_panels_preview, party_preview};
use game_engine_ui_model::{
    gameplay_baseline_battleground as pvp, gameplay_baseline_raid_info as raid,
};
use godot::prelude::*;
use ui_toolkit::atlas::ActiveSkin;

fn pvp_view(page: &str) -> pvp::PvpView {
    let mut view = pvp::PvpView {
        visible: page != "scoreboard",
        scoreboard: page == "scoreboard",
        viewport: [1920.0, 1080.0],
        snapshot: gameplay_panels_preview::scoreboard_view().snapshot,
        ..Default::default()
    };
    if page.starts_with("arena") {
        view.snapshot
            .as_mut()
            .expect("Baseline snapshot")
            .battleground = None;
        view.category = if page == "arena-rated" {
            pvp::PvpCategory::Rated
        } else {
            pvp::PvpCategory::QuickMatch
        };
    }
    view
}
fn raid_view() -> Result<raid::RaidInfoState, String> {
    let current = gameplay_panels_preview::raid_state()?;
    Ok(raid::RaidInfoState {
        visible: true,
        rows: current
            .rows
            .into_iter()
            .map(|row| raid::SavedInstanceRow {
                lock: row.lock,
                name: row.name,
                difficulty: row.difficulty,
                extend_disabled: row.extend_disabled,
            })
            .collect(),
        ..Default::default()
    })
}
#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_gameplay_baseline_preview(&mut self, forever: bool, page: GString) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            ui_toolkit::atlas::set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let parent = self.hud_parent()?;
            let mut registry = parent.registry();
            super::register_metal_frame_style(
                &mut registry,
                game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
            )?;
            if page == "raid" {
                self.show_viewport_screen_in(
                    raid_view()?,
                    raid::screen,
                    ScreenPostsetup::None,
                    registry,
                    parent,
                )
            } else {
                self.show_viewport_screen_in(
                    pvp_view(&page.to_string()),
                    pvp::pvp_screen,
                    ScreenPostsetup::None,
                    registry,
                    parent,
                )
            }
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
