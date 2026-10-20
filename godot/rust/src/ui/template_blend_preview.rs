//! Supplied offline fixtures for the ADD-blend panel groups other previews do not reach.
//! Each group feeds a minimal view through its production `show_*` host; nothing here
//! sets art, blend, postsetup or authority. All names, ids and amounts are supplied
//! fixture data, not server state.
//!
//! | panel | ADD regions | hover target |
//! |---|---|---|
//! | AdventureGuide | `EncounterJournalPage0ButtonHighlight` | `EncounterJournalPage0` |
//! | TradingPost | `PerksProgramRefundIcon4` | none (always drawn) |
//! | CharacterCurrency | `TokenHeader2Highlight{Left,Middle,Right}` | `TokenHeader2` |
//! | CollectionsPetSpellSelect | `PetJournalSpellSelectSelected` | none (checked option) |
//! | EntranceDifficulty | `EntranceDifficultyHighlight2` | `EntranceDifficultyButton2` |
//! | HousingStorageBasic | `HouseEditorStored1693HoverBackground`, `HouseEditorBasicDecorModeButtonHover` | `HouseEditorStored1693`, `HouseEditorBasicDecorModeButton` |
//! | WorldMap | `QuestLogTitle332Selected`, `WorldMapPin0Highlight` | none (selected row, supplied highlighted quest) |
use super::{RegistryUi, party_preview};
use godot::prelude::*;
use std::sync::Arc;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

const PANELS: &str = "AdventureGuide, TradingPost, CharacterCurrency, \
    CollectionsPetSpellSelect, EntranceDifficulty, HousingStorageBasic, WorldMap";

/// Supplied Deadmines opened through the production instance action: its encounter page
/// carries the EncounterJournal side tabs (`TabHighlight`, XML:657 ADD).
fn adventure_guide(
    viewport: [f32; 2],
) -> Result<game_engine_ui_model::adventure_guide::View, String> {
    use game_engine_core::adventure_guide::{Catalog, Instance};
    use game_engine_ui_model::adventure_guide::View;
    let defaults = View::default();
    let catalog = Catalog {
        instances: vec![Instance {
            id: 63,
            name: "Deadmines".into(),
            description: String::new(),
            map: 36,
            background: 522336,
            button: 522352,
            lore: 526404,
            dungeon: true,
            tiers: vec![defaults.tier],
        }],
        map_difficulties: [(36, vec![1, 2])].into(),
        ..Default::default()
    };
    let mut view = View {
        visible: true,
        viewport,
        catalog: Arc::new(catalog),
        ..defaults
    };
    view.apply("journal:instance:63")?;
    Ok(view)
}

/// One purchased product still inside its supplied refund window.
fn trading_post() -> game_engine_ui_model::trading_post::View {
    use shared::protocol::{
        TradingPostCategory, TradingPostItem, TradingPostReplyKind, TradingPostSnapshot,
    };
    let snapshot = TradingPostSnapshot {
        npc_net_id: Some(185468),
        tender: 500,
        points: 0,
        now_unix: 1790812900,
        month_end_unix: 1793491200,
        frozen_item_id: None,
        categories: vec![TradingPostCategory {
            id: 1,
            name: "Transmog".into(),
            model_scene_id: 430,
        }],
        items: vec![TradingPostItem {
            id: 4,
            category_id: 1,
            item_id: 189869,
            name: "Distinguished Warcloak".into(),
            description: "A red cloak".into(),
            icon_fdid: 135729,
            quality: 1,
            usable: Some(true),
            price: 100,
            purchased: true,
            collected: true,
            refund_seconds: 3600,
            ui_group_name: String::new(),
            ui_group_priority: 0,
            model_scene_id: 583,
            creature_display_id: 0,
            preview_item_ids: vec![189869],
            grant_unavailable: None,
        }],
        thresholds: vec![],
        source_gaps: vec![],
        error: None,
        catalog_complete: true,
        reply_kind: TradingPostReplyKind::Query,
    };
    let mut view = game_engine_ui_model::trading_post::View {
        visible: true,
        ..Default::default()
    };
    view.receive(Arc::new(snapshot));
    view
}

/// TokenFrame through the production PvP wallet metadata.
fn character_currency() -> Result<game_engine_ui_model::character_frame::CharacterFrameView, String>
{
    use game_engine_ui_model::character_frame::{CharacterFrameView, CharacterTab, currency_rows};
    use shared::protocol::CurrencyEntrySnapshot;
    let currencies = currency_rows(&[
        CurrencyEntrySnapshot {
            id: 1792,
            name: "Honor".into(),
            amount: 1250,
        },
        CurrencyEntrySnapshot {
            id: 1602,
            name: "Conquest".into(),
            amount: 0,
        },
    ])?;
    Ok(CharacterFrameView {
        visible: true,
        tab: CharacterTab::Currency,
        has_currencies: true,
        currencies,
        title: "Honorproof".into(),
        race_id: 1,
        class_id: 1,
        ..Default::default()
    })
}

/// Slotted pet with its first ability flyout open through the production reducer.
fn pet_spell_select(
    viewport: [f32; 2],
) -> Result<game_engine_ui_model::pet_journal::PetJournalView, String> {
    use game_engine_ui_model::pet_journal::{PetJournalView, PetSpeciesVisual};
    use shared::pet_battle::{PetJournal, PetQuality};
    use shared::protocol::{
        CollectionPetSnapshot, CollectionSnapshot, CollectionStateUpdate, OwnedPetBattleSnapshot,
        PetJournalAbility, PetJournalDetailsUpdate,
    };
    let mut journal = PetJournal::default();
    journal
        .add_with_breed(39, 1, PetQuality::Common, 3)
        .map_err(|error| format!("Pet spell select fixture journal: {error:?}"))?;
    let mut view = PetJournalView {
        visible: true,
        viewport,
        ..Default::default()
    };
    view.receive(CollectionStateUpdate {
        snapshot: Some(CollectionSnapshot {
            mounts: vec![],
            pets: vec![CollectionPetSnapshot {
                pet_id: 39,
                name: "Mechanical Squirrel".into(),
                known: true,
                active: false,
            }],
        }),
        pet_journal: Some(journal),
        summoned_pet_id: None,
        message: None,
        error: None,
    });
    view.visuals.insert(
        39,
        PetSpeciesVisual {
            species_id: 39,
            display_id: 7937,
            family: 9,
        },
    );
    view.receive_details(PetJournalDetailsUpdate {
        pets: vec![OwnedPetBattleSnapshot {
            pet_id: 1,
            health: 146,
            max_health: 146,
            power: 27,
            speed: 31,
            next_level_xp: 100,
            selected_abilities: [384, 0, 0],
            choices: vec![PetJournalAbility {
                id: 384,
                name: "Metal Fist".into(),
                icon: 132152,
                family: 9,
                slot: 1,
                required_level: 1,
            }],
        }],
        locked: false,
        error: None,
    });
    if !view.open_ability_choices(0, 1) {
        return Err("Pet spell select fixture did not open slot 0 ability 1".into());
    }
    Ok(view)
}

/// The Stockade bar with Normal selected; Heroic is the unselected hover target.
fn entrance_difficulty(
    screen_width: f32,
) -> Result<game_engine_ui_model::entrance_difficulty_component::EntranceBarState, String> {
    use game_engine_ui_model::dungeon_entrance_data::DifficultyChoice;
    use game_engine_ui_model::entrance_difficulty_component::{
        EntranceBarChoice, EntranceBarState,
    };
    // Same font and size the production entrance bar measures with (entrance_bar.rs).
    let font = super::assets::load_font(ui_toolkit::widgets::font_string::GameFont::FrizQuadrata)?;
    let measure = |text: &str| font.get_string_size_ex(text).font_size(14).done().x.ceil();
    let choices = [(1, "(5) Normal"), (2, "(5) Heroic")]
        .into_iter()
        .map(|(difficulty_id, label)| {
            let choice = DifficultyChoice {
                difficulty_id,
                label: label.into(),
                killed: 0,
                total: 3,
            };
            EntranceBarChoice {
                label_width: measure(&choice.label),
                progress_width: measure(&choice.progress_text()),
                choice,
            }
        })
        .collect();
    Ok(EntranceBarState {
        title: "The Stockade".into(),
        choices,
        selected: Some(1),
        screen_width,
        frame_alpha: 1.0,
        bar_alpha: 1.0,
        box_left: None,
        spinner: None,
    })
}

/// Basic editor with one stored, editable supplied decor tile.
fn housing_storage_basic(
    viewport: [f32; 2],
) -> game_engine_ui_model::housing_component::HousingView {
    use game_engine_core::housing::{Decor, HousingCatalog, RETAIL_BUILD_KEY};
    use game_engine_ui_model::housing::{HousingAuthority, HousingDashboard, HousingSurface};
    use shared::protocol::{HouseLayout, HousingState, StoredDecor};
    let mut model = HousingDashboard::new(Arc::new(HousingCatalog {
        decor: vec![Decor {
            id: 1693,
            name: "Small Val'sharah Bookcase".into(),
            item_id: 245259,
            model_type: 1,
            model_fdid: 1096761,
            initial_rotation: [0.0; 3],
            initial_scale: 1.0,
            thumbnail_fdid: 7421877,
            order: 328,
            subcategories: vec![],
            sources: vec![],
            dyeable: false,
        }],
        ..Default::default()
    }));
    model.surface = HousingSurface::Editor;
    game_engine_ui_model::housing_component::HousingView {
        model,
        authority: HousingAuthority {
            state: Some(HousingState {
                build_key: RETAIL_BUILD_KEY.into(),
                stored: vec![StoredDecor {
                    decor_id: 1693,
                    quantity: 1,
                }],
                layout: Some(HouseLayout {
                    house_id: 41,
                    can_edit: true,
                    decor: vec![],
                }),
                ..Default::default()
            }),
            error: None,
        },
        viewport,
        money: None,
        missing_thumbnails: Default::default(),
        vendor_names: Default::default(),
    }
}

/// Map & Quest Log with one selected quest and its supplied highlighted POI pin.
fn world_map(
    viewport: [f32; 2],
) -> game_engine_ui_model::world_map_frame_component::WorldMapFrameState {
    use game_engine_ui_model::quest_log_frame_component::{
        QuestDifficulty, QuestLogFrameState, QuestLogGroup, QuestLogRow,
    };
    use game_engine_ui_model::world_map_frame_component::{MapPin, MapPinType, WorldMapFrameState};
    let log = QuestLogFrameState {
        visible: true,
        quest_count: 1,
        max_quests: 35,
        groups: vec![QuestLogGroup {
            sort_id: 1519,
            name: "Stormwind City".into(),
            collapsed: false,
            quests: vec![QuestLogRow {
                quest_id: 332,
                title: "Wine Shop Advert".into(),
                level: 1,
                difficulty: QuestDifficulty::Trivial,
                complete: false,
                watched: true,
                selected: true,
            }],
        }],
        details: None,
    };
    WorldMapFrameState {
        viewport,
        pins: vec![MapPin {
            pin_type: MapPinType::QuestObjective,
            quest_id: Some(332),
            label: "Wine Shop Advert".into(),
            badge: "1".into(),
            x: 0.5,
            y: 0.5,
        }],
        highlighted_quest: Some(332),
        ..Default::default()
    }
    .with_quest_panel(log, false)
}

#[godot_api(secondary)]
impl RegistryUi {
    /// Offline capture of one ADD-blend gap group in the requested skin.
    #[func]
    fn show_template_blend_preview(&mut self, forever: bool, panel: GString) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let size = self
                .base()
                .get_viewport()
                .ok_or("Template blend preview has no viewport")?
                .get_visible_rect()
                .size;
            let viewport = [size.x, size.y];
            match panel.to_string().as_str() {
                "AdventureGuide" => self.show_adventure_guide(adventure_guide(viewport)?),
                "TradingPost" => self.show_trading_post(trading_post()),
                "CharacterCurrency" => self.show_character_frame(character_currency()?),
                "CollectionsPetSpellSelect" => self.show_pet_journal(pet_spell_select(viewport)?),
                "EntranceDifficulty" => self.show_entrance_bar(entrance_difficulty(size.x)?),
                "HousingStorageBasic" => self.show_housing(housing_storage_basic(viewport)),
                "WorldMap" => self.show_world_map(world_map(viewport)),
                other => Err(format!(
                    "Unknown template blend panel {other}; expected one of {PANELS}"
                )),
            }
        });
        result.err().unwrap_or_default().as_str().into()
    }
}
