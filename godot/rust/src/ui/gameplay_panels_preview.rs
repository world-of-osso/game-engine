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
    transmog_frame::{TransmogView, transmog_screen},
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
        "transmog" => {
            let mut book = TransmogBook::default();
            book.state = Some(shared::protocol::TransmogState::default());
            book.tab = WardrobeTab::Sets;
            book.filters_open = true;
            shared.insert(TransmogView {
                book,
                viewport_width: 1920.0,
                ..Default::default()
            });
            transmog_screen(&shared)
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

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn show_gameplay_panels_preview(&mut self, forever: bool, page: GString) -> GString {
        let page = page.to_string();
        if !["meter", "tracking", "transmog", "difficulty"].contains(&page.as_str()) {
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
            self.show_viewport_screen(Preview { page, skin }, screen, ScreenPostsetup::None)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
