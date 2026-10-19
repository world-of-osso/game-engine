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
            flags: 0,
            statistic_value: None,
            earned: true,
            earned_at: Some(1791244800),
            progress_supported: true,
            criteria: vec![AchievementCriterionLine {
                tree_id: 3386,
                order_index: 0,
                criteria_id: 18527,
                description: "Hogger".into(),
                flags: 0,
                current: 1,
                required: 1,
                completed: true,
                progress_supported: true,
            }],
            next_criteria_id: None,
        }],
        next_id: None,
    });
    window.apply(AchievementCatalogPage::Summary(AchievementSummary {
        scope: AchievementScope::All,
        points: 10,
        total: 1,
        completed: 1,
        categories: vec![AchievementCategoryTotals {
            category_id: 168,
            total: 1,
            completed: 1,
        }],
        recent: window.entries().to_vec(),
    }));
    window
}
fn category_pending_window(comparison: bool, phase: &str) -> Result<AchievementWindow, String> {
    let mut view = if phase == "rows" {
        window()
    } else {
        AchievementWindow::default()
    };
    view.visible = true;
    if comparison {
        view.action("achievement:compare:42");
    }
    view.selected_category = Some(14808);
    match phase {
        "pending" | "rows" => {}
        "empty" => view.pages.entry(14808).or_default().loaded = true,
        "search" => {
            view.action("achievement:search:bear");
            // Explicit full-list pending-state fixture, not typing-to-navigate behavior.
            view.full_search = true;
        }
        "error" => {
            view.selected_category = None;
            view.selected_achievement = Some(633);
        }
        _ => return Err(format!("Unknown category presentation phase: {phase}")),
    }
    Ok(view)
}

/// Explicit received-payload UI controls only; no network or persisted gameplay facts.
fn search_selection_window(many_results: bool) -> AchievementWindow {
    let mut view = window();
    let template = view.entries()[0].clone();
    view.action("achievement:search_focus");
    view.action("achievement:search:Level");
    let count = if many_results { 6 } else { 1 };
    view.apply(AchievementCatalogPage::Search {
        text: "Level".into(),
        scope: AchievementScope::All,
        achievements: (0..count)
            .map(|index| AchievementCatalogEntry {
                achievement_id: 6 + index,
                name: format!("Level {}", (index + 1) * 10),
                criteria: Vec::new(),
                ..template.clone()
            })
            .collect(),
        next_id: None,
    });
    view
}

/// Explicit supplied search-page metadata, not semantic search or realm progress.
fn full_results_preview_window(full: bool) -> AchievementWindow {
    let mut view = window();
    if !full {
        return view;
    }
    view.categories.insert(
        92,
        AchievementCategoryEntry {
            category_id: 92,
            parent_id: -1,
            order_index: 1,
            name: "Characters".into(),
        },
    );
    let template = view.entries()[0].clone();
    view.action("achievement:search:fixture");
    view.apply(AchievementCatalogPage::Search {
        text: "fixture".into(),
        scope: AchievementScope::All,
        achievements: [
            (6, "Level 10", true),
            (7, "Level 20", false),
            (8, "Level 30", false),
            (9, "Level 40", false),
            (10, "Level 50", false),
            (633, "Stormwind Stockade", true),
        ]
        .into_iter()
        .map(|(achievement_id, name, earned)| AchievementCatalogEntry {
            achievement_id,
            name: name.into(),
            earned,
            criteria: Vec::new(),
            ..template.clone()
        })
        .collect(),
        next_id: Some(633),
    });
    view.action("achievement:search_all");
    view
}

fn keyboard_preview_window(phase: &str) -> Result<AchievementWindow, String> {
    let mut view = search_selection_window(true);
    match phase {
        "many" => {}
        "two" => view.search_results.entries.truncate(2),
        "short" => {
            view.action("achievement:search:Le");
        }
        "pending" => {
            view.action("achievement:search:Level");
        }
        "empty" => {
            view.action("achievement:search:Level");
            view.apply(AchievementCatalogPage::Search {
                text: "Level".into(),
                scope: AchievementScope::All,
                achievements: Vec::new(),
                next_id: None,
            });
        }
        _ => return Err(format!("Unknown search keyboard preview phase: {phase}")),
    }
    Ok(view)
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

    /// Offline geometry only; comparison action does not fabricate a server reply.
    #[func]
    pub fn show_achievement_header_preview(
        &mut self,
        forever: bool,
        comparison: bool,
        query: GString,
    ) -> GString {
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        let result = self.prepare_dungeon_capture(skin).and_then(|()| {
            let mut window = AchievementWindow::default();
            let action = if comparison {
                "achievement:compare:42"
            } else {
                game_engine_ui_model::achievements::OPEN_ACTION
            };
            window.action(action);
            window.action(&format!("achievement:search:{query}"));
            // Offline browse selection only; typing no longer replaces the category body.
            window.selected_category = Some(92);
            self.show_achievement_window(window)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    /// Offline cache-state controls; the existing Stockade fixture supplies loaded rows.
    #[func]
    pub fn show_achievement_category_pending_preview(
        &mut self,
        forever: bool,
        comparison: bool,
        phase: GString,
    ) -> GString {
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        let result = self.prepare_dungeon_capture(skin).and_then(|()| {
            self.show_achievement_window(category_pending_window(comparison, &phase.to_string())?)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    /// Offline native focus controls use the production achievement action dispatcher.
    #[func]
    pub fn show_achievement_search_selection_preview(
        &mut self,
        forever: bool,
        many_results: bool,
    ) -> GString {
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        let result = self
            .prepare_dungeon_capture(skin)
            .and_then(|()| self.show_achievement_window(search_selection_window(many_results)));
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    pub fn achievement_search_selection_preview_dispatch(&mut self, action: GString) -> GString {
        let Some(mut view) = self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<AchievementWindow>())
            .cloned()
        else {
            return GString::from("Achievement search selection preview not mounted");
        };
        crate::achievements::apply_achievement_ui_action(&mut view, self, &action.to_string());
        let result = self.set_achievement_window(view);
        GString::from(result.err().unwrap_or_default().as_str())
    }

    /// Offline metadata controls; native key/pointer events feed the production dispatcher.
    #[func]
    pub fn show_achievement_search_keyboard_preview(
        &mut self,
        forever: bool,
        phase: GString,
    ) -> GString {
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        let result = self.prepare_dungeon_capture(skin).and_then(|()| {
            self.show_achievement_window(keyboard_preview_window(&phase.to_string())?)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    /// Full-search row presentation controls; all progress values are supplied UI fixtures.
    #[func]
    pub fn show_achievement_full_results_preview(&mut self, forever: bool, full: bool) -> GString {
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        let result = self
            .prepare_dungeon_capture(skin)
            .and_then(|()| self.show_achievement_window(full_results_preview_window(full)));
        GString::from(result.err().unwrap_or_default().as_str())
    }

    /// Inspect the supplied offline UI fixture's conservation boundary, not realm state.
    #[func]
    pub fn achievement_full_search_preview_state(&self) -> GString {
        let Some(view) = self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<AchievementWindow>())
        else {
            return GString::from("Achievement full-search preview not mounted");
        };
        GString::from(serde_json::json!({
            "visible": view.visible,
            "query": view.search_text,
            "category": view.selected_category,
            "selected": view.selected_achievement,
            "focused": view.search_focused,
            "ids": view.search_results.entries.iter().map(|entry| entry.achievement_id).collect::<Vec<_>>(),
            "next": view.search_results.next,
            "loaded": view.search_results.loaded,
        }).to_string().as_str())
    }

    /// Return actual outgoing catalog requests, without starting a transport or fabricating replies.
    #[func]
    pub fn achievement_search_keyboard_preview_dispatch(&mut self) -> GString {
        let Some(mut view) = self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<AchievementWindow>())
            .cloned()
        else {
            return GString::from("Achievement keyboard preview not mounted");
        };
        let mut requests = Vec::new();
        loop {
            let action = self.pop_action().to_string();
            if action.is_empty() {
                break;
            }
            requests.extend(crate::achievements::apply_achievement_ui_action(
                &mut view, self, &action,
            ));
        }
        if let Err(error) = self.set_achievement_window(view) {
            return GString::from(error.as_str());
        }
        match serde_json::to_string(&requests) {
            Ok(json) => GString::from(json.as_str()),
            Err(error) => GString::from(format!("Serialize keyboard requests: {error}").as_str()),
        }
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
                    icon_fdid: 134163,
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
