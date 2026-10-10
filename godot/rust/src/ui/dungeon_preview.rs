//! Offline Stockade snapshots through the production tracker and catalog reducers.
use godot::prelude::*;

use super::{RegistryUi, dungeon_preview, party_preview};
use game_engine_ui_model::{
    achievements::AchievementWindow, dungeon_progress::DungeonObjectives,
    objective_tracker_component::ObjectiveTrackerState,
};
use shared::protocol::*;

pub(super) fn tracker() -> ObjectiveTrackerState {
    let mut objectives = DungeonObjectives::default();
    objectives.begin_map(34);
    objectives.apply(
        DungeonProgress {
            map_id: 34,
            instance_id: 7,
            difficulty_id: 1,
            encounters: ["Randolph Moloch", "Lord Overheat", "Hogger"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| DungeonEncounterProgress {
                    encounter_id: index as u32 + 1,
                    name: name.into(),
                    defeated: index == 0,
                    optional: None,
                    flags: 0,
                })
                .collect(),
        },
        "Stormwind Stockade".into(),
    );
    ObjectiveTrackerState {
        dungeon: objectives.block,
        ..Default::default()
    }
}
pub(super) fn window() -> AchievementWindow {
    let mut window = AchievementWindow::default();
    window.action(game_engine_ui_model::achievements::OPEN_ACTION);
    window.apply(AchievementCatalogPage::Categories {
        categories: vec![
            AchievementCategoryEntry {
                category_id: 168,
                parent_id: -1,
                order_index: 1,
                name: "Dungeons & Raids".into(),
            },
            AchievementCategoryEntry {
                category_id: 14808,
                parent_id: 168,
                order_index: 1,
                name: "Classic".into(),
            },
        ],
        next_id: None,
    });
    window.action("achievement:category:14808");
    window.apply(AchievementCatalogPage::Category {
        category_id: 14808,
        achievements: vec![AchievementCatalogEntry {
            achievement_id: 633,
            name: "Stormwind Stockade".into(),
            description: "Defeat Hogger.".into(),
            points: 10,
            icon_fdid: 134163,
            earned: true,
            earned_at: Some(1791244800),
            progress_supported: true,
            criteria: vec![AchievementCriterionLine {
                tree_id: 3386,
                order_index: 0,
                criteria_id: 18527,
                description: "Hogger".into(),
                current: 1,
                required: 1,
                completed: true,
                progress_supported: true,
            }],
            next_criteria_id: None,
        }],
        next_id: None,
    });
    window
}
pub(super) fn cache_art() -> Result<(), String> {
    use godot::obj::Singleton;
    let data = godot::classes::ProjectSettings::singleton().globalize_path("res://../data");
    let root = std::path::PathBuf::from(data.to_string());
    let resolver = crate::assets::creature::local_resolver(&root);
    let fdids = game_engine_ui_model::achievements::ART
        .iter()
        .copied()
        .chain([134163, 5320671])
        .chain(game_engine_ui_model::panel_style_data::metal_sheet_fdids(
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        ));
    for fdid in fdids {
        let path = root.join("textures").join(format!("{fdid}.blp"));
        if !path.exists() && resolver.ensure_cached(fdid, &path)?.is_none() {
            return Err(format!(
                "Offline capture: FDID {fdid} missing from local CASC"
            ));
        }
    }
    Ok(())
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_dungeon_tracker_preview(&mut self) -> GString {
        self.show_dungeon_capture(ui_toolkit::atlas::ActiveSkin::Modern, false)
    }

    #[func]
    pub fn show_forever_dungeon_tracker_preview(&mut self) -> GString {
        self.show_dungeon_capture(ui_toolkit::atlas::ActiveSkin::Forever, false)
    }

    #[func]
    pub fn show_achievement_preview(&mut self) -> GString {
        self.show_dungeon_capture(ui_toolkit::atlas::ActiveSkin::Modern, true)
    }

    #[func]
    pub fn show_forever_achievement_preview(&mut self) -> GString {
        self.show_dungeon_capture(ui_toolkit::atlas::ActiveSkin::Forever, true)
    }

    #[func]
    pub fn show_achievement_toast_preview(&mut self) -> GString {
        let result = self
            .prepare_dungeon_capture(ui_toolkit::atlas::ActiveSkin::Modern)
            .and_then(|()| {
                self.show_achievement_toast(shared::protocol::AchievementToastSnapshot {
                    achievement_id: 633,
                    name: "Stormwind Stockade".into(),
                    points: 10,
                })
            });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    fn prepare_dungeon_capture(
        &mut self,
        skin: ui_toolkit::atlas::ActiveSkin,
    ) -> Result<(), String> {
        party_preview::load_data_root()?;
        ui_toolkit::atlas::set_thread_skin(skin);
        dungeon_preview::cache_art()?;
        self.set_ui_scale(1.0)
    }

    fn show_dungeon_capture(
        &mut self,
        skin: ui_toolkit::atlas::ActiveSkin,
        window: bool,
    ) -> GString {
        let result = (|| {
            self.prepare_dungeon_capture(skin)?;
            if window {
                self.show_achievement_window(dungeon_preview::window())
            } else {
                self.show_objective_tracker(dungeon_preview::tracker())
            }
        })();
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
