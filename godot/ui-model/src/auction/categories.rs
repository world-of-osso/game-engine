//! Retail category/subcategory/sub-subcategory class filters.
//! Mainline/Blizzard_AuctionData.lua:15-211 and Shared/Blizzard_AuctionData.lua:133-149.
//! Generated subclass names/IDs/order below are the local Retail ItemSubClass.csv
//! snapshot (12.1.0.69933), excluding hidden auction subclasses (DisplayFlags bit 2).
//! Legendary-only implicit filters and WoW Token are outside the query contract.
use crate::auction_house_frame_component::CategoryRow;
use shared::protocol::{AuctionItemFilter, AuctionSearchQuery};
use std::sync::OnceLock;

pub const CATEGORIES: [(&str, u8); 14] = [
    ("Weapons", 2),
    ("Armor", 4),
    ("Containers", 1),
    ("Gems", 3),
    ("Item Enhancements", 8),
    ("Consumables", 0),
    ("Glyphs", 16),
    ("Reagents", 7),
    ("Recipes", 9),
    ("Profession Equipment", 19),
    ("Housing", 20),
    ("Battle Pets", 17),
    ("Quest Items", 12),
    ("Miscellaneous", 15),
];

struct CategoryNode {
    name: &'static str,
    filters: Vec<AuctionItemFilter>,
    children: Vec<CategoryNode>,
}

fn leaf(
    name: &'static str,
    class_id: u8,
    subclass_id: Option<u8>,
    inventory_type: Option<u8>,
) -> CategoryNode {
    CategoryNode {
        name,
        filters: vec![AuctionItemFilter {
            class_id,
            subclass_id,
            inventory_type,
        }],
        children: Vec::new(),
    }
}

fn subclass(name: &'static str, class: u8, subclass: u8) -> CategoryNode {
    leaf(name, class, Some(subclass), None)
}

fn group(name: &'static str, children: Vec<CategoryNode>) -> CategoryNode {
    let filters = children
        .iter()
        .flat_map(|child| child.filters.iter().cloned())
        .collect();
    CategoryNode {
        name,
        filters,
        children,
    }
}

fn weapons() -> Vec<CategoryNode> {
    vec![
        group(
            "One-Handed",
            [
                ("Axe", 0),
                ("Mace", 4),
                ("Sword", 7),
                ("Warglaives", 9),
                ("Dagger", 15),
                ("Fist Weapon", 13),
                ("Wand", 19),
            ]
            .into_iter()
            .map(|(name, id)| subclass(name, 2, id))
            .collect(),
        ),
        group(
            "Two-Handed",
            [
                ("Axe", 1),
                ("Mace", 5),
                ("Sword", 8),
                ("Polearm", 6),
                ("Staff", 10),
            ]
            .into_iter()
            .map(|(name, id)| subclass(name, 2, id))
            .collect(),
        ),
        group(
            "Ranged",
            [("Bow", 2), ("Crossbow", 18), ("Gun", 3), ("Thrown", 16)]
                .into_iter()
                .map(|(name, id)| subclass(name, 2, id))
                .collect(),
        ),
        group(
            "Miscellaneous",
            vec![subclass("Fishing Pole", 2, 20), subclass("Other", 2, 14)],
        ),
    ]
}

fn armor() -> Vec<CategoryNode> {
    let mut children = Vec::new();
    for (name, id) in [("Plate", 4), ("Mail", 3), ("Leather", 2), ("Cloth", 1)] {
        let mut material = subclass(name, 4, id);
        material.children = [
            ("Head", 1),
            ("Shoulder", 3),
            ("Chest", 5),
            ("Waist", 6),
            ("Legs", 7),
            ("Feet", 8),
            ("Wrist", 9),
            ("Hands", 10),
        ]
        .into_iter()
        .map(|(name, slot)| {
            let mut node = leaf(name, 4, Some(id), Some(slot));
            if slot == 5 {
                node.filters.push(AuctionItemFilter {
                    class_id: 4,
                    subclass_id: Some(id),
                    inventory_type: Some(20),
                });
            }
            node
        })
        .collect();
        children.push(material);
    }
    children.push(group(
        "Miscellaneous",
        vec![
            leaf("Neck", 4, Some(0), Some(2)),
            leaf("Cloak", 4, Some(1), Some(16)),
            leaf("Finger", 4, Some(0), Some(11)),
            leaf("Trinket", 4, Some(0), Some(12)),
            leaf("Held In Off-hand", 4, Some(0), Some(23)),
            subclass("Shield", 4, 6),
            leaf("Shirt", 4, Some(0), Some(4)),
            leaf("Head", 4, Some(0), Some(1)),
        ],
    ));
    children.push(subclass("Cosmetic", 4, 5));
    children
}

fn professions() -> Vec<CategoryNode> {
    [
        ("Inscription", 12),
        ("Tailoring", 6),
        ("Leatherworking", 1),
        ("Jewelcrafting", 11),
        ("Alchemy", 2),
        ("Blacksmithing", 0),
        ("Engineering", 7),
        ("Enchanting", 8),
        ("Mining", 5),
        ("Herbalism", 3),
        ("Skinning", 10),
        ("Cooking", 4),
        ("Fishing", 9),
    ]
    .into_iter()
    .map(|(name, id)| {
        let mut node = subclass(name, 19, id);
        node.children = vec![
            leaf("Tools", 19, Some(id), Some(29)),
            leaf("Accessories", 19, Some(id), Some(30)),
        ];
        node
    })
    .collect()
}

const GENERATED_SUBCLASSES: &[(u8, &[(u8, &str)])] = [
    (
        0,
        &[
            (0, "Explosives and Devices"),
            (1, "Potions"),
            (2, "Elixirs"),
            (3, "Flasks & Phials"),
            (5, "Food & Drink"),
            (7, "Bandages"),
            (9, "Vantus Runes"),
            (12, "Relic"),
            (8, "Other"),
        ],
    ),
    (
        1,
        &[
            (0, "Bag"),
            (2, "Herb Bag"),
            (3, "Enchanting Bag"),
            (4, "Engineering Bag"),
            (5, "Gem Bag"),
            (6, "Mining Bag"),
            (7, "Leatherworking Bag"),
            (8, "Inscription Bag"),
            (9, "Tackle Box"),
            (10, "Cooking Bag"),
            (11, "Reagent Bag"),
        ],
    ),
    (
        3,
        &[
            (11, "Artifact Relic"),
            (0, "Intellect"),
            (1, "Agility"),
            (2, "Strength"),
            (3, "Stamina"),
            (5, "Critical Strike"),
            (6, "Mastery"),
            (7, "Haste"),
            (8, "Versatility"),
            (9, "Other"),
            (10, "Multiple Stats"),
        ],
    ),
    (
        7,
        &[
            (5, "Cloth"),
            (6, "Leather"),
            (7, "Metal & Stone"),
            (8, "Cooking"),
            (9, "Herb"),
            (12, "Enchanting"),
            (16, "Inscription"),
            (4, "Jewelcrafting"),
            (1, "Parts"),
            (10, "Elemental"),
            (18, "Optional Reagents"),
            (19, "Finishing Reagents"),
            (11, "Other"),
        ],
    ),
    (
        8,
        &[
            (0, "Head"),
            (1, "Neck"),
            (2, "Shoulder"),
            (3, "Cloak"),
            (4, "Chest"),
            (5, "Wrist"),
            (6, "Hands"),
            (7, "Waist"),
            (8, "Legs"),
            (9, "Feet"),
            (10, "Finger"),
            (11, "Weapon"),
            (12, "Two-Handed Weapon"),
            (13, "Shield/Off-hand"),
            (14, "Misc"),
        ],
    ),
    (
        9,
        &[
            (1, "Leatherworking"),
            (2, "Tailoring"),
            (3, "Engineering"),
            (4, "Blacksmithing"),
            (6, "Alchemy"),
            (8, "Enchanting"),
            (10, "Jewelcrafting"),
            (11, "Inscription"),
            (5, "Cooking"),
            (7, "First Aid"),
            (9, "Fishing"),
            (0, "Book"),
        ],
    ),
    (
        16,
        &[
            (1, "Warrior"),
            (2, "Paladin"),
            (3, "Hunter"),
            (4, "Rogue"),
            (5, "Priest"),
            (6, "Death Knight"),
            (7, "Shaman"),
            (8, "Mage"),
            (9, "Warlock"),
            (10, "Monk"),
            (11, "Druid"),
            (12, "Demon Hunter"),
        ],
    ),
    (17, &[(0, "BattlePet")]),
];

fn children(class: u8) -> Vec<CategoryNode> {
    match class {
        2 => weapons(),
        4 => armor(),
        19 => professions(),
        20 => vec![subclass("Decor", 20, 0), subclass("Dye", 20, 1)],
        15 => [
            ("Junk", 0),
            ("Reagent", 1),
            ("Holiday", 3),
            ("Other", 4),
            ("Mount", 5),
            ("Mount Equipment", 6),
        ]
        .into_iter()
        .map(|(name, id)| subclass(name, 15, id))
        .collect(),
        _ => GENERATED_SUBCLASSES
            .iter()
            .find(|(id, _)| *id == class)
            .map(|(_, rows)| {
                rows.iter()
                    .map(|(id, name)| subclass(name, class, *id))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn tree() -> &'static [CategoryNode] {
    static TREE: OnceLock<Vec<CategoryNode>> = OnceLock::new();
    TREE.get_or_init(|| {
        CATEGORIES
            .iter()
            .map(|(name, class)| {
                let mut node = leaf(name, *class, None, None);
                node.children = children(*class);
                if *class == 17 {
                    node.children.push(subclass("Companion Pet", 15, 2));
                    node.filters.push(AuctionItemFilter {
                        class_id: 15,
                        subclass_id: Some(2),
                        inventory_type: None,
                    });
                }
                node
            })
            .collect()
    })
}

fn find(path: &[usize]) -> Option<&'static CategoryNode> {
    let (first, rest) = path.split_first()?;
    let mut node = tree().get(*first)?;
    for index in rest {
        node = node.children.get(*index)?;
    }
    Some(node)
}

pub(super) fn parse_path(token: &str) -> Option<Vec<usize>> {
    let path = token
        .split('/')
        .map(str::parse)
        .collect::<Result<Vec<usize>, _>>()
        .ok()?;
    (path.len() <= 3 && find(&path).is_some()).then_some(path)
}

pub(super) fn select(selected: &mut Vec<usize>, path: Vec<usize>) {
    if selected.starts_with(&path) {
        *selected = path[..path.len() - 1].to_vec();
    } else {
        *selected = path;
    }
}

pub(super) fn apply_filter(query: &mut AuctionSearchQuery, selected: &[usize]) {
    query.class_id = None;
    query.subcategory_filters.clear();
    let Some(node) = find(selected) else {
        return;
    };
    let class = node.filters[0].class_id;
    query.class_id = node
        .filters
        .iter()
        .all(|filter| filter.class_id == class)
        .then_some(class);
    if selected.len() > 1 || query.class_id.is_none() {
        query.subcategory_filters = node.filters.clone();
    }
}

pub(super) fn rows(selected: &[usize]) -> Vec<CategoryRow> {
    let mut out = Vec::new();
    append_rows(tree(), selected, &mut Vec::new(), &mut out);
    out
}

fn append_rows(
    nodes: &[CategoryNode],
    selected: &[usize],
    path: &mut Vec<usize>,
    out: &mut Vec<CategoryRow>,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let active = selected.starts_with(path);
        out.push(CategoryRow {
            name: node.name.into(),
            selected: active,
            path: path
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("/"),
        });
        if active {
            append_rows(&node.children, selected, path, out);
        }
        path.pop();
    }
}
