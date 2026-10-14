//! Offline production-screen fixtures for Retail control comparisons, without network mutation.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use game_engine_ui_model::{
    damage_meter_component::damage_meter_screen,
    inworld_unit_frames_component::{
        DifficultyMenuEntry, DifficultyMenuState, InWorldUnitFramesState, UnitFrameMenuState,
        UnitFrameState, inworld_unit_frames_screen,
    },
    minimap::{MinimapClusterState, TrackingState, minimap_cluster_screen},
    transmog::{TransmogBook, WardrobeTab},
    transmog_frame::TransmogView,
};
use godot::prelude::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    screen::SharedContext,
    widget_def::Element,
};

#[derive(Clone, PartialEq)]
struct Preview {
    page: String,
    skin: ActiveSkin,
}

fn screen(ctx: &SharedContext) -> Element {
    let preview = ctx.get::<Preview>().expect("gameplay preview");
    let mut shared = SharedContext::new();
    shared.insert(preview.skin);
    match preview.page.as_str() {
        "meter" => {
            let mut view = super::meter_preview::view();
            view.menu_open = true;
            view.type_menu_open = true;
            shared.insert(view);
            damage_meter_screen(&shared)
        }
        "tracking" => {
            shared.insert(MinimapClusterState {
                tracking: TrackingState {
                    open: true,
                    ..Default::default()
                },
                ..Default::default()
            });
            minimap_cluster_screen(&shared)
        }
        "difficulty" => {
            shared.insert(InWorldUnitFramesState {
                show_player_frame: false,
                show_target_frame: false,
                target_cast: None,
                player: UnitFrameState::named("Player"),
                target: None,
                target_of_target: None,
                focus: None,
                pet: None,
                bosses: vec![],
                personal_resource: None,
                menu: UnitFrameMenuState {
                    difficulty_menu: Some(DifficultyMenuState {
                        x: 800.0,
                        y: 200.0,
                        entries: [
                            (1, "Normal", false),
                            (2, "Heroic", true),
                            (23, "Mythic", false),
                        ]
                        .into_iter()
                        .map(|(difficulty_id, label, checked)| DifficultyMenuEntry {
                            difficulty_id,
                            label: label.into(),
                            checked,
                            enabled: true,
                        })
                        .collect(),
                    }),
                    ..Default::default()
                },
            });
            inworld_unit_frames_screen(&shared)
        }
        _ => unreachable!("validated gameplay preview page"),
    }
}

fn raid_state() -> Result<game_engine_ui_model::raid_info::RaidInfoState, String> {
    use game_engine_ui_model::raid_info::{RaidCatalog, RaidInfoState};
    let catalog = RaidCatalog::load(&game_engine_ui_model::paths::resolve_data_path(
        "db2/12.1.0.69933",
    ))?;
    // MapDifficulty.csv: Vanilla40=9, ten-player=3, twenty-five-player=4.
    let locks: Vec<_> = [
        (409, 9),
        (469, 9),
        (533, 3),
        (249, 3),
        (532, 3),
        (565, 4),
        (544, 4),
        (580, 4),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(index, (map_id, difficulty_id))| shared::protocol::InstanceLockInfo {
            map_id,
            difficulty_id,
            instance_id: index as u32 + 1,
            time_remaining_secs: 4000,
            completed_mask: 0,
            locked: true,
            extended: false,
        },
    )
    .collect();
    Ok(RaidInfoState {
        visible: true,
        rows: catalog.rows(&locks, 0)?,
        ..Default::default()
    })
}

fn scoreboard_view() -> game_engine_ui_model::battleground::PvpView {
    use shared::{
        battleground::{BgPhase, BgTeam, FlagObjective},
        protocol::{BattlegroundPlayerScoreSnapshot, BattlegroundSnapshot, PvpSnapshot},
    };
    game_engine_ui_model::battleground::PvpView {
        scoreboard: true,
        viewport: [1920.0, 1080.0],
        snapshot: Some(PvpSnapshot {
            honor: 0,
            honor_max: 0,
            conquest: 0,
            conquest_max: None,
            brackets: vec![],
            queue: None,
            war_mode: None,
            talents: None,
            battleground: Some(BattlegroundSnapshot {
                match_id: 71,
                battleground_id: 1014,
                name: "Warsong Gulch".into(),
                map_id: 2106,
                team: BgTeam::A,
                phase: BgPhase::InProgress,
                accepted: true,
                invitation_time_left_secs: None,
                score_a: 2,
                score_b: 1,
                flags: FlagObjective::default(),
                world_states: vec![],
                outcome: None,
                players: vec![BattlegroundPlayerScoreSnapshot {
                    character_id: 81,
                    name: "Flagrunner".into(),
                    class: 1,
                    level: 90,
                    team: BgTeam::A,
                    killing_blows: 0,
                    deaths: 0,
                    damage_done: 0,
                    healing_done: 0,
                    objective_scores: vec![(929, 1), (928, 2)],
                    honorable_kills: None,
                    honor_level: None,
                }],
                honor_reward: None,
                arena: None,
            }),
        }),
        ..Default::default()
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_gameplay_panels_preview(&mut self, forever: bool, page: GString) -> GString {
        let page = page.to_string();
        if ![
            "meter",
            "tracking",
            "transmog",
            "difficulty",
            "raid",
            "scoreboard",
            "arena-quick",
            "arena-rated",
            "talent-menu",
            "talent-dialog",
            "talent-choice",
        ]
        .contains(&page.as_str())
        {
            return GString::from("Unknown gameplay preview page");
        }
        let skin = if forever {
            ActiveSkin::Forever
        } else {
            ActiveSkin::Modern
        };
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            if page == "raid" {
                return self.show_raid_info(raid_state()?);
            }
            if page == "scoreboard" {
                return self.show_pvp(scoreboard_view());
            }
            if page.starts_with("arena-") {
                let mut view = scoreboard_view();
                view.visible = true;
                view.scoreboard = false;
                view.snapshot
                    .as_mut()
                    .expect("Offline PvP snapshot")
                    .battleground = None;
                view.category = if page == "arena-rated" {
                    game_engine_ui_model::battleground::PvpCategory::Rated
                } else {
                    game_engine_ui_model::battleground::PvpCategory::QuickMatch
                };
                return self.show_pvp(view);
            }
            if page.starts_with("talent-") {
                let data = game_engine_ui_model::paths::resolve_data_path("");
                let mut state = game_engine_ui_model::spellbook_preview::load_preview_state(
                    &data,
                    game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab::Talents,
                )?;
                let talents = state
                    .talents
                    .as_mut()
                    .ok_or("Talent preview missing graph")?;
                if page == "talent-menu" {
                    talents.editor.loadout_menu = true;
                } else if page == "talent-choice" {
                    let choice = talents
                        .nodes()
                        .find(|node| node.node_type != 3 && node.entries.len() > 1)
                        .ok_or("Authored talent preview has no non-subtree choice")?
                        .id;
                    talents.editor.choice_node = Some(choice);
                } else {
                    talents.editor.loadout_dialog =
                        Some(game_engine_ui_model::talents::LoadoutDialog::Create);
                    talents.editor.loadout_name = "Raid Night".into();
                }
                return self.show_spellbook(state);
            }
            if page == "transmog" {
                let mut book = TransmogBook::default();
                book.state = Some(shared::protocol::TransmogState::default());
                book.tab = WardrobeTab::Sets;
                book.filters_open = true;
                return self.show_transmog(TransmogView {
                    book,
                    viewport_width: 1920.0,
                    ..Default::default()
                });
            }
            self.show_viewport_screen(Preview { page, skin }, screen, ScreenPostsetup::None)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
