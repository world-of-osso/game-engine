//! TransmogSetModelMixin:RefreshTooltip (Blizzard_TransmogTemplates.lua): the set name in
//! its average item quality colour, then each primary appearance, collected pieces in
//! LIGHTYELLOW_FONT_COLOR and the rest as disabled lines.
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{TransmogCollectionState, TransmogSet};

use super::GameTooltip;
use crate::item_catalog::item_catalog_entry_for;
use crate::merchant_data::quality_color;
use crate::tooltip_presentation::{
    GRAY_FONT_COLOR, TooltipLineState, TooltipPresentation, parse_rgba,
};

/// `LIGHTYELLOW_FONT_COLOR`.
const LIGHTYELLOW_FONT_COLOR: [f32; 4] = [1.0, 1.0, 0.6, 1.0];
/// `Enum.ItemQuality.Common`.
const COMMON: u8 = 1;

pub fn transmog_set_tooltip(set: &TransmogSet) -> GameTooltip {
    let qualities: Vec<u32> = set
        .primary_appearances
        .iter()
        .filter_map(|piece| {
            item_catalog_entry_for(ItemDefinitionSource::Retail, piece.source.item_id)
                .map(|item| u32::from(item.quality))
        })
        .collect();
    // Round(totalQuality / numTotal), Common when no quality is known.
    let total: u32 = qualities.iter().sum();
    let quality = if qualities.is_empty() || total == 0 {
        COMMON
    } else {
        ((total as f32 / set.primary_appearances.len() as f32).round()) as u8
    };
    let lines = set
        .primary_appearances
        .iter()
        .map(|piece| {
            let color = if piece.collection == TransmogCollectionState::Collected {
                LIGHTYELLOW_FONT_COLOR
            } else {
                GRAY_FONT_COLOR
            };
            TooltipLineState::colored(piece.source.name.clone(), color)
        })
        .collect();
    GameTooltip::new(
        TooltipPresentation {
            title: set.name.clone(),
            title_color: parse_rgba(quality_color(quality)),
            lines,
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

/// ItemModelBaseMixin:OnEnter for an illusion card (`GetIllusionStrings` name) and
/// TransmogIllusionSlotMixin's tooltip, which shows WEAPON_ENCHANTMENT without an illusion.
pub fn transmog_illusion_tooltip(name: Option<&str>) -> GameTooltip {
    GameTooltip::new(
        TooltipPresentation {
            title: name.unwrap_or("Weapon Enchantment").to_owned(),
            ..TooltipPresentation::hidden()
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{EquipmentSlot, TransmogCatalogueSource, TransmogSource};

    fn piece(
        id: u32,
        item: u32,
        name: &str,
        collection: TransmogCollectionState,
    ) -> TransmogCatalogueSource {
        TransmogCatalogueSource {
            source: TransmogSource {
                source_id: id,
                source_type: 1,
                appearance_id: 0,
                item_id: item,
                display_id: 0,
                icon_fdid: 0,
                name: name.into(),
                inventory_type: 0,
            },
            collection,
            slots: vec![EquipmentSlot::Chest],
        }
    }

    #[test]
    fn transmog_set_tooltip_lists_collected_and_uncollected_pieces() {
        // TransmogSet852 Battlegear of Wrath: Breastplate7030 collected, Helm7027 not.
        let set = TransmogSet {
            set_id: 852,
            parent_set_id: 0,
            name: "Battlegear of Wrath".into(),
            source_ids: vec![7027, 7030],
            primary_appearances: vec![
                piece(
                    7027,
                    16963,
                    "Helm of Wrath",
                    TransmogCollectionState::Uncollected,
                ),
                piece(
                    7030,
                    16966,
                    "Breastplate of Wrath",
                    TransmogCollectionState::Collected,
                ),
            ],
        };
        let tooltip = transmog_set_tooltip(&set).content;
        assert_eq!(tooltip.title, "Battlegear of Wrath");
        let lines: Vec<_> = tooltip
            .lines
            .iter()
            .map(|line| (line.left_text.as_str(), line.left_color))
            .collect();
        assert_eq!(
            lines,
            vec![
                ("Helm of Wrath", GRAY_FONT_COLOR),
                ("Breastplate of Wrath", LIGHTYELLOW_FONT_COLOR),
            ]
        );
    }

    #[test]
    fn transmog_illusion_tooltip_names_the_illusion_or_the_empty_slot() {
        // TransmogIllusion3 -> SpellItemEnchantment1898 "Lifestealing".
        let named = transmog_illusion_tooltip(Some("Lifestealing")).content;
        assert_eq!(
            (named.title.as_str(), named.lines.len()),
            ("Lifestealing", 0)
        );
        let empty = transmog_illusion_tooltip(None).content;
        assert_eq!(empty.title, "Weapon Enchantment");
    }
}
