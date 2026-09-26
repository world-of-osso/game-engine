use shared::protocol::TooltipDrop;

use super::*;

fn item(item_id: u32, name: &str, quality: u8, appearance_id: Option<u32>) -> TooltipItem {
    TooltipItem {
        item_id,
        name: name.into(),
        quality,
        appearance_id,
    }
}

fn drop(item: TooltipItem, chance: f32) -> TooltipDrop {
    TooltipDrop { item, chance }
}

/// The server's answer for Defias Thug (38), from world.db (game-server
/// `creature_tooltip_tests.rs`).
fn defias_thug() -> CreatureTooltip {
    CreatureTooltip {
        entry: 38,
        subname: String::new(),
        creature_type: 7,
        drops: vec![
            drop(item(752, "Red Burlap Bandana", 1, None), 60.0),
            drop(item(2070, "Darnassian Bleu", 1, None), 20.1),
            drop(item(159, "Refreshing Spring Water", 1, None), 9.9),
            drop(item(2057, "Pitted Defias Shortsword", 1, Some(646)), 2.0),
            drop(item(805, "Small Red Pouch", 1, None), 0.04),
            drop(item(828, "Small Blue Pouch", 1, None), 0.04),
            drop(item(4496, "Small Brown Pouch", 1, None), 0.04),
            drop(item(5571, "Small Black Pouch", 1, None), 0.04),
            drop(item(5572, "Small Green Pouch", 1, None), 0.04),
        ],
        vendor_items: Vec::new(),
    }
}

/// Corina Steele (54), Weaponsmith in Northshire.
fn corina_steele() -> CreatureTooltip {
    CreatureTooltip {
        entry: 54,
        subname: "Weaponsmith".into(),
        creature_type: 7,
        drops: Vec::new(),
        vendor_items: vec![
            item(2488, "Gladius", 1, Some(154)),
            item(2489, "Two-Handed Sword", 1, Some(607)),
            item(2490, "Tomahawk", 1, Some(388)),
            item(2491, "Large Axe", 1, Some(857)),
            item(2492, "Cudgel", 1, Some(858)),
            item(2493, "Wooden Mallet", 1, Some(859)),
            item(2494, "Stiletto", 1, Some(860)),
            item(2495, "Walking Stick", 1, Some(861)),
        ],
    }
}

fn collection(ids: &[u32]) -> AppearanceCollection {
    let mut collection = AppearanceCollection::default();
    collection.learn_many(ids.iter().copied());
    collection
}

/// `(mark, left text, right text)` of each line.
fn rows(state: &TooltipFrameState) -> Vec<(Option<ItemMark>, &str, &str)> {
    state
        .lines
        .iter()
        .map(|line| {
            (
                line.item_mark,
                line.left_text.as_str(),
                line.right_text.as_str(),
            )
        })
        .collect()
}

#[test]
fn defias_thug_tooltip_lists_uncollected_first_then_by_chance_and_truncates() {
    let data = defias_thug();
    let input = NpcTooltipInput {
        entry: 38,
        name: "Defias Thug",
        reaction: Reaction::Hostile,
        level: Some(3),
        faction: None,
        data: Some(&data),
    };

    let tooltip = npc_tooltip(&input, &AppearanceCollection::default());

    assert_eq!(tooltip.title, "Defias Thug");
    assert_eq!(tooltip.title_color, FACTION_RED);
    use ItemMark::{Uncollected, Unmarked};
    assert_eq!(
        rows(&tooltip),
        [
            (None, "Level 3 Humanoid", ""),
            (None, "Drops", ""),
            (Some(Uncollected), "Pitted Defias Shortsword", "2%"),
            (Some(Unmarked), "Red Burlap Bandana", "60%"),
            (Some(Unmarked), "Darnassian Bleu", "20.1%"),
            (Some(Unmarked), "Refreshing Spring Water", "9.9%"),
            (Some(Unmarked), "Small Red Pouch", "0.04%"),
            (None, "+4 more", ""),
        ]
    );
    // Common quality names are white.
    assert_eq!(tooltip.lines[3].left_color, TOOLTIP_WHITE);
}

#[test]
fn a_collected_appearance_shows_the_check_and_loses_its_priority() {
    let data = defias_thug();
    let input = NpcTooltipInput {
        entry: 38,
        name: "Defias Thug",
        reaction: Reaction::Hostile,
        level: Some(3),
        faction: None,
        data: Some(&data),
    };

    let tooltip = npc_tooltip(&input, &collection(&[646]));

    let names: Vec<_> = rows(&tooltip)[2..7]
        .iter()
        .map(|(mark, name, _)| (*mark, *name))
        .collect();
    assert_eq!(
        names,
        [
            (Some(ItemMark::Unmarked), "Red Burlap Bandana"),
            (Some(ItemMark::Unmarked), "Darnassian Bleu"),
            (Some(ItemMark::Unmarked), "Refreshing Spring Water"),
            (Some(ItemMark::Collected), "Pitted Defias Shortsword"),
            (Some(ItemMark::Unmarked), "Small Red Pouch"),
        ]
    );
}

#[test]
fn fourteen_items_show_five_lines_and_how_many_hidden_ones_are_collected() {
    let items: Vec<SectionItem> = (0..14)
        .map(|index| SectionItem {
            name: format!("Item {index}"),
            quality: 2,
            chance: Some(14.0 - index as f32),
            // Items 0, 4, 8, 12 are uncollected; the rest collected.
            collected: Some(index % 4 != 0),
        })
        .collect();

    let lines = section_lines("Drops", &items);

    let texts: Vec<_> = lines.iter().map(|line| line.left_text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "Drops",
            "Item 0",
            "Item 4",
            "Item 8",
            "Item 12",
            "Item 1",
            "+9 more (9 collected)",
        ]
    );
    assert_eq!(lines[1].item_mark, Some(ItemMark::Uncollected));
    assert_eq!(lines[5].item_mark, Some(ItemMark::Collected));
    assert_eq!(lines[1].left_color, parse_rgba(quality_color(2)));
}

#[test]
fn six_items_are_all_listed() {
    let items: Vec<SectionItem> = (0..6)
        .map(|index| SectionItem {
            name: format!("Item {index}"),
            quality: 1,
            chance: Some(50.0),
            collected: None,
        })
        .collect();

    let lines = section_lines("Drops", &items);

    assert_eq!(lines.len(), 7);
    assert!(lines.iter().all(|line| !line.left_text.starts_with('+')));
}

#[test]
fn vendor_tooltip_lists_sold_items_without_chances() {
    let data = corina_steele();
    let input = NpcTooltipInput {
        entry: 54,
        name: "Corina Steele",
        reaction: Reaction::Friendly,
        level: Some(10),
        faction: Some("Stormwind"),
        data: Some(&data),
    };

    // Gladius shares the Worn Shortsword's appearance 154.
    let tooltip = npc_tooltip(&input, &collection(&[154]));

    assert_eq!(tooltip.title_color, FACTION_GREEN);
    use ItemMark::Uncollected;
    assert_eq!(
        rows(&tooltip),
        [
            (None, "Weaponsmith", ""),
            (None, "Level 10 Humanoid", ""),
            (None, "Stormwind", ""),
            (None, "Sells", ""),
            (Some(Uncollected), "Two-Handed Sword", ""),
            (Some(Uncollected), "Tomahawk", ""),
            (Some(Uncollected), "Large Axe", ""),
            (Some(Uncollected), "Cudgel", ""),
            (Some(Uncollected), "Wooden Mallet", ""),
            (None, "+3 more (1 collected)", ""),
        ]
    );
}

#[test]
fn before_the_server_answers_the_npc_shows_its_basic_lines() {
    let input = NpcTooltipInput {
        entry: 68,
        name: "Stormwind City Guard",
        reaction: Reaction::Friendly,
        level: Some(30),
        faction: Some("Stormwind"),
        data: None,
    };

    let tooltip = npc_tooltip(&input, &AppearanceCollection::default());

    assert_eq!(
        rows(&tooltip),
        [(None, "Level 30", ""), (None, "Stormwind", "")]
    );
}

#[test]
fn player_tooltip_shows_guild_and_level_race_class() {
    let tooltip = player_tooltip(&PlayerTooltipInput {
        name: "Uther",
        reaction: Reaction::Friendly,
        guild: Some("Silver Hand"),
        level: Some(12),
        race: "Human",
        class: "Paladin",
    });

    assert_eq!(tooltip.title, "Uther");
    assert_eq!(tooltip.title_color, TOOLTIP_WHITE);
    assert_eq!(
        rows(&tooltip),
        [
            (None, "<Silver Hand>", ""),
            (None, "Level 12 Human Paladin (Player)", ""),
        ]
    );
    assert!(tooltip.lines.iter().all(|line| line.item_mark.is_none()));
}

#[test]
fn chances_read_as_short_percentages() {
    assert_eq!(chance_text(60.0), "60%");
    assert_eq!(chance_text(20.1), "20.1%");
    assert_eq!(chance_text(0.04), "0.04%");
    assert_eq!(chance_text(0.5), "0.5%");
    assert_eq!(chance_text(0.004), "<0.01%");
}

#[test]
fn npc_tooltips_name_their_creature_record_and_players_none() {
    let input = NpcTooltipInput {
        entry: 38,
        name: "Defias Thug",
        reaction: Reaction::Hostile,
        level: Some(3),
        faction: None,
        data: None,
    };
    let npc = npc_tooltip(&input, &AppearanceCollection::default());
    assert_eq!(npc.record, Some(TooltipRecord::Creature(38)));
    let player = player_tooltip(&PlayerTooltipInput {
        name: "Uther",
        reaction: Reaction::Friendly,
        guild: None,
        level: Some(12),
        race: "Human",
        class: "Paladin",
    });
    assert_eq!(player.record, None);
}
