//! Local Retail12.1.0.69933 identities: Gladius821/Deadman Blade1181 (collected),
//! TransmogSet13
//! "Wild Combatant's Chain Armor" members70596 helm/70594 chest/70595 gauntlets, TransmogIllusion3
//! Lifestealing (default collected) and2 Unholy Weapon (condition-locked), OutfitEntry2/3
//! awarded and4 purchased.
use super::*;
use shared::item_data::ItemDefinitionSource;
use shared::protocol::*;

fn source(
    id: u32,
    appearance: u32,
    item: u32,
    display: u32,
    icon: u32,
    name: &str,
    inv: u8,
) -> TransmogSource {
    TransmogSource {
        source_id: id,
        source_type: if id == 1181 { 4 } else { 3 },
        appearance_id: appearance,
        item_id: item,
        display_id: display,
        icon_fdid: icon,
        name: name.into(),
        inventory_type: inv,
    }
}

fn gladius() -> TransmogSource {
    source(821, 154, 2488, 1542, 135274, "Gladius", 13)
}

fn deadman() -> TransmogSource {
    source(1181, 389, 3295, 1546, 135274, "Deadman Blade", 13)
}

fn chain_armor() -> TransmogSource {
    source(
        70594,
        25939,
        124923,
        142240,
        1092121,
        "Wild Combatant's Chain Armor",
        5,
    )
}

fn gauntlets() -> TransmogSource {
    source(
        70595,
        25940,
        124924,
        142246,
        1092122,
        "Wild Combatant's Chain Gauntlets",
        10,
    )
}

fn helm() -> TransmogSource {
    source(
        70596,
        25941,
        124925,
        142242,
        1092123,
        "Wild Combatant's Chain Helm",
        1,
    )
}

fn slot(
    slot: EquipmentSlot,
    guid: u64,
    item: u32,
    inv: u8,
    sources: Vec<TransmogSource>,
) -> TransmogSlot {
    TransmogSlot {
        slot,
        item_guid: guid,
        item_id: item,
        definition_source: ItemDefinitionSource::Retail,
        inventory_type: inv,
        applied: TransmogChoice::Original,
        cost: Some(1500),
        can_hide: false,
        sources,
    }
}

fn piece(
    source: TransmogSource,
    collection: TransmogCollectionState,
    slot: EquipmentSlot,
) -> TransmogCatalogueSource {
    TransmogCatalogueSource {
        source,
        collection,
        slots: vec![slot],
    }
}

fn illusion(id: u32, enchantment: u32, name: &str, icon: u32, collected: bool) -> TransmogIllusion {
    TransmogIllusion {
        id,
        enchantment_id: enchantment,
        name: name.into(),
        icon_fdid: icon,
        cost: 10000,
        unlock_condition_id: if collected { 0 } else { 41116 },
        flags: 0,
        collected,
    }
}

fn entry(
    id: u32,
    name: &str,
    order: u32,
    cost: u64,
    source: u8,
    flags: u32,
) -> TransmogOutfitEntry {
    TransmogOutfitEntry {
        id,
        name: name.into(),
        order_index: order,
        cost,
        source,
        flags,
        set_type: u8::from(id != 1),
    }
}

fn wardrobe() -> TransmogState {
    TransmogState {
        npc: 54442,
        slots: vec![
            slot(EquipmentSlot::Chest, 91002, 124923, 5, vec![chain_armor()]),
            slot(EquipmentSlot::Hands, 91003, 124924, 10, vec![]),
            slot(
                EquipmentSlot::MainHand,
                91001,
                2850,
                13,
                vec![gladius(), deadman()],
            ),
        ],
        sets: vec![TransmogSet {
            set_id: 13,
            parent_set_id: 80,
            name: "Wild Combatant's Chain Armor".into(),
            source_ids: vec![70594, 70595, 70596],
            primary_appearances: vec![
                piece(
                    helm(),
                    TransmogCollectionState::Uncollected,
                    EquipmentSlot::Head,
                ),
                piece(
                    chain_armor(),
                    TransmogCollectionState::Collected,
                    EquipmentSlot::Chest,
                ),
                piece(
                    gauntlets(),
                    TransmogCollectionState::Uncollected,
                    EquipmentSlot::Hands,
                ),
            ],
        }],
        outfit_entries: vec![
            entry(1, "Default", 1, 0, 0, 1),
            entry(2, "Outfit 1", 2, 0, 1, 35),
            entry(3, "Outfit 2", 3, 0, 1, 3),
            entry(4, "", 4, 1000000, 2, 0),
        ],
        illusions: vec![
            illusion(2, 1899, "Unholy Weapon", 136225, false),
            illusion(3, 1898, "Lifestealing", 136130, true),
            illusion(13, 5393, "Crusader", 236246, true),
        ],
        outfit_slots: vec![TransmogOutfitSlot {
            id: 13,
            slot: EquipmentSlot::MainHand,
            flags: 3,
            collection_type: 0,
        }],
        ..Default::default()
    }
}

pub(super) fn book() -> TransmogBook {
    let mut book = TransmogBook::default();
    book.receive(wardrobe());
    book.money = 5000;
    book
}

#[test]
fn transmog_source_and_completion_filters_preserve_pending_choices_and_quotes() {
    let mut book = book();
    book.select(EquipmentSlot::MainHand);
    assert!(book.stage(TransmogChoice::Source(1181)));
    book.excluded_source_types.insert(4); // Enum.TransmogSource.WorldDrop
    assert_eq!(
        book.page_sources()
            .iter()
            .map(|s| s.source_id)
            .collect::<Vec<_>>(),
        vec![821]
    );
    assert_eq!(book.pending_cost(), Some(1500));
    assert_eq!(
        book.pending[&EquipmentSlot::MainHand],
        TransmogChoice::Source(1181)
    );
    book.excluded_source_types.clear();
    assert_eq!(book.page_sources().len(), 2);
    book.set_tab(WardrobeTab::Sets);
    book.exclude_uncollected_sets = true;
    assert!(book.available_sets().is_empty());
    book.exclude_uncollected_sets = false;
    assert_eq!(book.available_sets().len(), 1);
    for piece in &mut book.state.as_mut().unwrap().sets[0].primary_appearances {
        piece.collection = TransmogCollectionState::Collected;
    }
    book.exclude_collected_sets = true;
    assert!(book.available_sets().is_empty());
    assert_eq!(book.pending_cost(), Some(1500));
}

#[test]
fn transmog_outfit_list_shows_awarded_entries_and_defaults_to_the_first() {
    let book = book();
    assert_eq!(
        book.outfit_rows(),
        vec![
            OutfitRow {
                id: 2,
                name: "Outfit 1".into(),
                icon_fdid: 0,
            },
            OutfitRow {
                id: 3,
                name: "Outfit 2".into(),
                icon_fdid: 0,
            },
        ]
    );
    assert_eq!(book.viewed_outfit, Some(2));
}

#[test]
fn transmog_save_outfit_pays_through_apply_then_commits_the_viewed_outfit() {
    let mut book = book();
    book.select(EquipmentSlot::MainHand);
    assert!(book.stage(TransmogChoice::Source(1181)));
    assert!(book.can_save_outfit());
    let Some(TransmogSend::Apply(apply)) = book.save_outfit() else {
        panic!("pending appearance must use the paid Apply path");
    };
    assert_eq!(
        apply.changes,
        vec![TransmogChange {
            slot: EquipmentSlot::MainHand,
            item_guid: 91001,
            choice: TransmogChoice::Source(1181),
        }]
    );
    assert!(
        book.take_queued_save().is_none(),
        "commit waits for the Apply result"
    );
    assert!(
        book.save_outfit().is_none(),
        "no double save while applying"
    );
    let mut applied = wardrobe();
    applied.slots[2].applied = TransmogChoice::Source(1181);
    book.receive(applied.clone());
    let save = book.take_queued_save().expect("commit after paid Apply");
    assert_eq!(save.npc, 54442);
    assert_eq!(save.outfit.id, 2);
    assert_eq!(save.outfit.name, "Outfit 1");
    assert_eq!(
        save.outfit.choices,
        vec![
            TransmogOutfitChoice {
                slot: EquipmentSlot::Chest,
                choice: TransmogChoice::Original
            },
            TransmogOutfitChoice {
                slot: EquipmentSlot::Hands,
                choice: TransmogChoice::Original
            },
            TransmogOutfitChoice {
                slot: EquipmentSlot::MainHand,
                choice: TransmogChoice::Source(1181)
            },
        ]
    );
    assert!(save.outfit.illusions.is_empty());
    assert_eq!(book.saving, Some(SaveKind::Outfit));
    applied.outfits = vec![save.outfit.clone()];
    book.receive(applied);
    assert_eq!(book.saving, None);
    assert!(book.pending.is_empty());
}

#[test]
fn transmog_apply_refusal_drops_the_queued_outfit_commit() {
    let mut book = book();
    book.select(EquipmentSlot::MainHand);
    book.stage(TransmogChoice::Source(821));
    book.save_outfit().unwrap();
    book.receive_failure(TransmogFailed {
        npc: 54442,
        error: TransmogError::NotEnoughMoney,
    });
    assert!(book.take_queued_save().is_none());
    assert!(!book.busy());
    assert_eq!(book.pending.len(), 1, "pending choice survives a refusal");
}

#[test]
fn transmog_illusion_choice_saves_without_paid_apply_and_reports_refusals() {
    let mut book = book();
    book.select_illusion(EquipmentSlot::MainHand);
    assert!(book.illusion_selected);
    let listed: Vec<_> = book
        .page_items()
        .into_iter()
        .map(|entry| match entry {
            ItemEntry::Illusion(illusion) => illusion.id,
            other => panic!("illusion location lists only illusions, got {other:?}"),
        })
        .collect();
    assert_eq!(
        listed,
        vec![3, 13],
        "condition-locked Unholy Weapon is not collected"
    );
    assert!(!book.stage_illusion(Some(2)));
    assert!(
        !book.stage(TransmogChoice::Source(821)),
        "illusion location stages no appearance"
    );
    assert!(book.stage_illusion(Some(3)));
    assert_eq!(book.effective_illusion(EquipmentSlot::MainHand), Some(3));
    assert_eq!(book.pending_cost(), Some(0));
    let Some(TransmogSend::SaveOutfit(save)) = book.save_outfit() else {
        panic!("illusion-only change saves the outfit directly");
    };
    assert_eq!(
        save.outfit.illusions,
        vec![TransmogOutfitIllusion {
            slot: EquipmentSlot::MainHand,
            illusion_id: 3
        }]
    );
    book.receive_outfit_failure(TransmogOutfitFailed {
        npc: 54442,
        error: TransmogOutfitError::InvalidChoice,
    });
    assert_eq!(book.saving, None);
    assert_eq!(book.error, "You cannot use that appearance.");
    assert_eq!(
        book.pending_illusions.get(&EquipmentSlot::MainHand),
        Some(&Some(3))
    );
    assert!(!book.illusion_allowed(EquipmentSlot::Chest));
}

#[test]
fn transmog_viewing_a_saved_outfit_stages_its_appearances_and_shows_its_illusion() {
    let mut state = wardrobe();
    state.outfits = vec![TransmogOutfit {
        id: 3,
        name: "Duelist".into(),
        icon_fdid: 135343,
        choices: vec![TransmogOutfitChoice {
            slot: EquipmentSlot::MainHand,
            choice: TransmogChoice::Source(1181),
        }],
        illusions: vec![TransmogOutfitIllusion {
            slot: EquipmentSlot::MainHand,
            illusion_id: 13,
        }],
    }];
    let mut book = TransmogBook::default();
    book.receive(state);
    assert_eq!(book.outfit_rows()[1].name, "Duelist");
    assert!(!book.view_outfit(4), "purchasable slot is not owned");
    assert!(book.view_outfit(3));
    assert_eq!(
        book.pending.get(&EquipmentSlot::MainHand),
        Some(&TransmogChoice::Source(1181))
    );
    assert_eq!(book.effective_illusion(EquipmentSlot::MainHand), Some(13));
    assert!(
        book.pending_illusions.is_empty(),
        "saved illusion is not a pending change"
    );
    assert!(book.view_outfit(2));
    assert!(book.pending.is_empty());
    assert_eq!(book.effective_illusion(EquipmentSlot::MainHand), None);
}

#[test]
fn transmog_sets_count_every_slot_and_stage_only_usable_members() {
    let mut book = book();
    book.set_tab(WardrobeTab::Sets);
    let sets = book.page_sets();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].0.set_id, 13);
    assert_eq!(
        sets[0].1,
        SetProgress {
            collected: 1,
            total: 3
        },
        "unequipped head still counts"
    );
    assert_eq!(book.set_match(sets[0].0), None);
    assert!(book.use_set(13));
    assert_eq!(
        book.pending,
        BTreeMap::from([(EquipmentSlot::Chest, TransmogChoice::Source(70594))]),
        "uncollected gauntlets are not staged"
    );
    assert_eq!(
        book.set_match(&book.state.as_ref().unwrap().sets[0]),
        None,
        "Retail requires every covered slot, and the head shows nothing"
    );
    let equipment = book.set_equipment(13).unwrap();
    assert_eq!(
        equipment
            .entries
            .iter()
            .map(|entry| (entry.slot, entry.display_info_id))
            .collect::<Vec<_>>(),
        vec![
            (shared::components::EquipmentVisualSlot::Head, Some(142242)),
            (shared::components::EquipmentVisualSlot::Chest, Some(142240)),
            (shared::components::EquipmentVisualSlot::Hands, Some(142246)),
        ],
        "set model tries on every primary appearance"
    );
}

#[test]
fn transmog_set_matches_when_every_covered_slot_shows_a_member() {
    let mut state = wardrobe();
    state.sets[0].primary_appearances.remove(0);
    state.sets[0].primary_appearances[1].collection = TransmogCollectionState::Collected;
    state.slots[1].sources = vec![gauntlets()];
    let mut book = TransmogBook::default();
    book.receive(state);
    assert!(book.use_set(13));
    assert_eq!(
        book.set_match(&book.state.as_ref().unwrap().sets[0]),
        Some(true)
    );
}

#[test]
fn transmog_custom_set_creation_uses_current_view_and_loading_stages_it() {
    let mut book = book();
    assert!(
        !book.can_create_custom_set(),
        "all-original view has no appearance"
    );
    book.select(EquipmentSlot::MainHand);
    book.stage(TransmogChoice::Source(821));
    book.open_custom_set_name();
    assert!(book.naming_custom_set);
    assert!(book.create_custom_set("   ").is_none());
    let Some(TransmogSend::SaveCustomSet(save)) = book.create_custom_set(" Duelist ") else {
        panic!("named custom set");
    };
    assert_eq!(save.id, None, "server allocates custom set IDs");
    assert_eq!(save.name, "Duelist");
    assert_eq!(
        save.choices,
        vec![TransmogOutfitChoice {
            slot: EquipmentSlot::MainHand,
            choice: TransmogChoice::Source(821),
        }]
    );
    let mut saved = wardrobe();
    saved.custom_sets = vec![TransmogCustomSet {
        id: 7,
        name: "Duelist".into(),
        choices: save.choices.clone(),
    }];
    book.receive(saved);
    assert!(!book.naming_custom_set);
    book.undo();
    assert!(book.pending.is_empty());
    assert!(book.use_custom_set(7));
    assert_eq!(
        book.pending.get(&EquipmentSlot::MainHand),
        Some(&TransmogChoice::Source(821))
    );
    assert_eq!(book.custom_set_match(book.custom_sets()[0]), Some(true));
}

#[test]
fn transmog_hidden_visual_is_the_first_items_entry_for_hideable_slots() {
    let mut state = wardrobe();
    state.slots[0].can_hide = true;
    let mut book = TransmogBook::default();
    book.receive(state);
    book.select(EquipmentSlot::Chest);
    assert_eq!(
        book.page_items(),
        vec![ItemEntry::Hidden, ItemEntry::Source(&chain_armor())]
    );
    book.set_search("chain".into());
    assert_eq!(book.page_items(), vec![ItemEntry::Source(&chain_armor())]);
}

#[test]
fn transmog_icon_edit_preserves_pending_apply_and_saved_configuration() {
    let mut book = book();
    book.select(EquipmentSlot::MainHand);
    assert!(book.stage(TransmogChoice::Source(1181)));
    let pending = book.pending.clone();
    let money = book.money;
    assert!(book.edit_outfit_info(2));
    assert_eq!(book.outfit_info.as_ref().unwrap().icon_fdid, 0);
    assert!(!book.select_outfit_icon(999999));
    assert!(book.select_outfit_icon(134400));
    book.outfit_info.as_mut().unwrap().name = "Chosen blade".into();
    let Some(TransmogSend::SaveOutfit(save)) = book.commit_outfit_info() else {
        panic!("metadata save");
    };
    assert_eq!(save.outfit.icon_fdid, 134400);
    assert_eq!(save.outfit.name, "Chosen blade");
    assert_eq!(book.pending, pending);
    assert_eq!(book.money, money);
    assert!(!book.applying);
    let mut state = book.state.clone().unwrap();
    state.outfits = vec![save.outfit];
    book.receive(state);
    assert_eq!(book.outfit_rows()[0].icon_fdid, 134400);
    assert_eq!(book.pending, pending);
    assert_eq!(book.money, money);
    assert!(book.edit_outfit_info(2));
    assert_eq!(book.outfit_info.as_ref().unwrap().icon_fdid, 134400);
}
