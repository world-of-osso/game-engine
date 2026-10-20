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
    CollectionsPetSpellSelect, EntranceDifficulty, HousingStorageBasic, WorldMap, GuildBank, GuildTabard, AchievementStatistics, Transmog";

/// Cached Blackfathom/encounter112 opened through production instance, encounter and
/// model-page actions; this reaches both side-tab and creature-ring ADD regions.
fn adventure_guide(
    viewport: [f32; 2],
) -> Result<game_engine_ui_model::adventure_guide::View, String> {
    use game_engine_core::adventure_guide::Catalog;
    use game_engine_ui_model::adventure_guide::View;
    let catalog = Catalog::load(&game_engine_ui_model::paths::resolve_data_path(
        "cache/adventure_guide.sqlite",
    ))?;
    let mut view = View {
        visible: true,
        viewport,
        catalog: Arc::new(catalog),
        ..Default::default()
    };
    view.apply("journal:instance:67")?;
    view.apply("journal:encounter:112")?;
    view.apply("journal:page:model")?;
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

fn achievement_statistics() -> game_engine_ui_model::achievements::AchievementWindow {
    use game_engine_ui_model::achievements::{AchievementWindow, CategoryPage};
    use shared::protocol::{AchievementCatalogEntry, AchievementCategoryEntry};
    let mut window = AchievementWindow {
        visible: true,
        statistics: true,
        selected_category: Some(14807),
        ..Default::default()
    };
    window.categories.insert(
        14807,
        AchievementCategoryEntry {
            category_id: 14807,
            parent_id: 1,
            order_index: 1,
            name: "Dungeons & Raids".into(),
        },
    );
    let entries = [1068, 1069]
        .into_iter()
        .map(|achievement_id| AchievementCatalogEntry {
            achievement_id,
            name: format!("Statistic {achievement_id}"),
            description: String::new(),
            points: 0,
            icon_fdid: 0,
            flags: 1,
            earned: false,
            earned_at: None,
            progress_supported: false,
            criteria: vec![],
            next_criteria_id: None,
            statistic_value: Some(7),
        })
        .collect();
    window.pages.insert(
        14807,
        CategoryPage {
            entries,
            next: None,
            loaded: true,
        },
    );
    window
}

fn transmog() -> game_engine_ui_model::transmog_frame::TransmogView {
    use shared::{
        item_data::ItemDefinitionSource,
        protocol::{EquipmentSlot, TransmogChoice, TransmogSlot, TransmogState},
    };
    let mut book = game_engine_ui_model::transmog::TransmogBook::default();
    book.state = Some(TransmogState {
        npc: 54442,
        slots: vec![TransmogSlot {
            slot: EquipmentSlot::MainHand,
            item_guid: 91001,
            item_id: 2850,
            definition_source: ItemDefinitionSource::Retail,
            inventory_type: 13,
            applied: TransmogChoice::Original,
            cost: Some(1500),
            can_hide: false,
            sources: vec![],
        }],
        ..Default::default()
    });
    game_engine_ui_model::transmog_frame::TransmogView {
        book,
        viewport_width: 1920.0,
        ..Default::default()
    }
}

fn guild_bank() -> game_engine_ui_model::guild_bank::NativeGuildBankView {
    use game_engine_ui_model::guild_bank_frame_component::{GuildBankFrameState, GuildSideTab};
    game_engine_ui_model::guild_bank::NativeGuildBankView {
        frame: GuildBankFrameState {
            visible: true,
            title: "Guild Vault".into(),
            tabs: vec![GuildSideTab {
                icon_fdid: 132074,
                selected: true,
            }],
            ..Default::default()
        },
        bags: Default::default(),
    }
}

fn guild_tabard() -> Result<game_engine_ui_model::guild_tabards::GuildTabards, String> {
    use shared::protocol::{GuildTabardReply, GuildTabardVendorSnapshot};
    let mut view = game_engine_ui_model::guild_tabards::GuildTabards::default();
    view.begin_open(701);
    view.receive(GuildTabardReply::Vendor(GuildTabardVendorSnapshot {
        npc: 701,
        npc_name: "Jondor Steelbrow".into(),
        own_rank: Some(0),
        design: None,
        cost: 100000,
    }))?;
    Ok(view)
}

/// Map & Quest Log with one selected quest and its supplied highlighted POI pin.
fn world_map(
    viewport: [f32; 2],
) -> Result<game_engine_ui_model::world_map_frame_component::WorldMapFrameState, String> {
    use game_engine_ui_model::quest_log_frame_component::{
        QuestDifficulty, QuestLogFrameState, QuestLogGroup, QuestLogRow,
    };
    use game_engine_ui_model::world_map_frame_component::{
        MapHighlight, MapPin, MapPinType, MapTile, WorldMapFrameState,
    };
    let catalog = game_engine_ui_model::ui_map_data::UiMapCatalog::load(
        &game_engine_ui_model::paths::resolve_data_path("db2/active"),
    )?;
    let art = catalog.art(37).ok_or("Elwynn map art missing")?;
    let continent = catalog.art(13).ok_or("Eastern Kingdoms map art missing")?;
    let tiles = continent
        .placed_tiles()
        .into_iter()
        .map(|tile| MapTile {
            fdid: tile.fdid,
            rect: tile.rect,
            tex_coords: tile.tex_coords,
        })
        .collect();
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
    Ok(WorldMapFrameState {
        viewport,
        map_id: 13,
        map_name: "Eastern Kingdoms".into(),
        tiles,
        highlight: Some(MapHighlight {
            map_id: 37,
            name: "Elwynn Forest".into(),
            fdid: art.highlight_fdid,
            rect: [0.4184, 0.7322, 0.0834, 0.0834],
        }),
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
    .with_quest_panel(log, false))
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
                "WorldMap" => self.show_world_map(world_map(viewport)?),
                "GuildBank" => self.show_guild_bank(guild_bank()),
                "GuildTabard" => self.show_guild_tabard(guild_tabard()?),
                "AchievementStatistics" => self.show_achievement_window(achievement_statistics()),
                "Transmog" => self.show_transmog(transmog()),
                other => Err(format!(
                    "Unknown template blend panel {other}; expected one of {PANELS}"
                )),
            }
        });
        result.err().unwrap_or_default().as_str().into()
    }
}
