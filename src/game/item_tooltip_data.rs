//! Item tooltip data model.
//!
//! Structured representation of WoW-style item tooltips with name, quality,
//! stats, equip effects, set bonuses, sell price, and binding info.

/// Item binding type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BindType {
    #[default]
    None,
    BindOnPickup,
    BindOnEquip,
    BindOnUse,
}

impl BindType {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::BindOnPickup => "Binds when picked up",
            Self::BindOnEquip => "Binds when equipped",
            Self::BindOnUse => "Binds when used",
        }
    }
}

/// A single stat line (e.g. "+15 Strength").
#[derive(Clone, Debug, PartialEq)]
pub struct StatLine {
    pub label: String,
    pub value: i32,
}

impl StatLine {
    pub fn display(&self) -> String {
        if self.value >= 0 {
            format!("+{} {}", self.value, self.label)
        } else {
            format!("{} {}", self.value, self.label)
        }
    }
}

/// An equip effect or use effect line.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectLine {
    pub prefix: String,
    pub description: String,
}

impl EffectLine {
    pub fn display(&self) -> String {
        if self.prefix.is_empty() {
            self.description.clone()
        } else {
            format!("{}: {}", self.prefix, self.description)
        }
    }
}

/// Complete item tooltip data.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ItemTooltip {
    pub name: String,
    pub quality: crate::bag_data::ItemQuality,
    pub item_level: u32,
    pub bind_type: BindType,
    /// Equipment slot (e.g. "Head", "One-Hand Sword"). Empty for non-equipment.
    pub slot_text: String,
    /// Armor type (e.g. "Plate", "Cloth"). Empty for non-armor.
    pub armor_type: String,
    pub armor_value: u32,
    pub stats: Vec<StatLine>,
    pub effects: Vec<EffectLine>,
    /// Set name (empty if not part of a set).
    pub set_name: String,
    pub set_pieces: Vec<String>,
    pub set_bonuses: Vec<(u32, String)>,
    /// Flavor text (italic, yellow).
    pub flavor_text: String,
    /// Sell price in copper.
    pub sell_price: u64,
    /// Required level to use.
    pub required_level: u32,
}

impl ItemTooltip {
    /// Quality-colored name as [r, g, b, a].
    pub fn name_color(&self) -> [f32; 4] {
        let border = self.quality.border_color();
        // Parse "r,g,b,a" string — for tooltip, use the border color
        // but ensure alpha is 1.0
        parse_color_str(border)
    }

    /// Whether to show the armor value line.
    pub fn has_armor(&self) -> bool {
        self.armor_value > 0
    }

    /// Total number of tooltip lines (for layout sizing).
    pub fn line_count(&self) -> usize {
        let mut count = 1; // name
        if self.item_level > 0 {
            count += 1;
        }
        if self.bind_type != BindType::None {
            count += 1;
        }
        if !self.slot_text.is_empty() {
            count += 1;
        }
        if self.armor_value > 0 {
            count += 1;
        }
        count += self.stats.len();
        count += self.effects.len();
        if !self.set_name.is_empty() {
            count += 1 + self.set_pieces.len() + self.set_bonuses.len();
        }
        if !self.flavor_text.is_empty() {
            count += 1;
        }
        if self.sell_price > 0 {
            count += 1;
        }
        if self.required_level > 0 {
            count += 1;
        }
        count
    }

    /// Format sell price as "Xg Ys Zc".
    pub fn sell_price_text(&self) -> String {
        if self.sell_price == 0 {
            return String::new();
        }
        let g = self.sell_price / 10_000;
        let s = (self.sell_price % 10_000) / 100;
        let c = self.sell_price % 100;
        if g > 0 {
            format!("{g}g {s}s {c}c")
        } else if s > 0 {
            format!("{s}s {c}c")
        } else {
            format!("{c}c")
        }
    }
}

fn parse_color_str(s: &str) -> [f32; 4] {
    let parts: Vec<f32> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    if parts.len() >= 3 {
        [parts[0], parts[1], parts[2], 1.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bag_data::ItemQuality;

    #[test]
    fn stat_line_positive() {
        let stat = StatLine {
            label: "Strength".into(),
            value: 15,
        };
        assert_eq!(stat.display(), "+15 Strength");
    }

    #[test]
    fn stat_line_negative() {
        let stat = StatLine {
            label: "Spirit".into(),
            value: -5,
        };
        assert_eq!(stat.display(), "-5 Spirit");
    }

    #[test]
    fn effect_line_with_prefix() {
        let effect = EffectLine {
            prefix: "Equip".into(),
            description: "Increases haste by 5%.".into(),
        };
        assert_eq!(effect.display(), "Equip: Increases haste by 5%.");
    }

    #[test]
    fn effect_line_no_prefix() {
        let effect = EffectLine {
            prefix: String::new(),
            description: "Restores 20 health per 5 sec.".into(),
        };
        assert_eq!(effect.display(), "Restores 20 health per 5 sec.");
    }

    #[test]
    fn bind_type_labels() {
        assert_eq!(BindType::None.label(), "");
        assert_eq!(BindType::BindOnPickup.label(), "Binds when picked up");
        assert_eq!(BindType::BindOnEquip.label(), "Binds when equipped");
        assert_eq!(BindType::BindOnUse.label(), "Binds when used");
    }

    #[test]
    fn sell_price_formatting() {
        let mut tip = ItemTooltip::default();
        assert_eq!(tip.sell_price_text(), "");
        tip.sell_price = 150342;
        assert_eq!(tip.sell_price_text(), "15g 3s 42c");
        tip.sell_price = 350;
        assert_eq!(tip.sell_price_text(), "3s 50c");
        tip.sell_price = 42;
        assert_eq!(tip.sell_price_text(), "42c");
    }

    #[test]
    fn line_count_minimal() {
        let tip = ItemTooltip {
            name: "Hearthstone".into(),
            ..Default::default()
        };
        assert_eq!(tip.line_count(), 1); // just the name
    }

    #[test]
    fn line_count_full_item() {
        let tip = ItemTooltip {
            name: "Ashkandi".into(),
            quality: ItemQuality::Epic,
            item_level: 77,
            bind_type: BindType::BindOnPickup,
            slot_text: "Two-Hand Sword".into(),
            armor_value: 0,
            stats: vec![
                StatLine {
                    label: "Strength".into(),
                    value: 40,
                },
                StatLine {
                    label: "Stamina".into(),
                    value: 30,
                },
            ],
            effects: vec![EffectLine {
                prefix: "Equip".into(),
                description: "Chance on hit".into(),
            }],
            flavor_text: "The dark blade of legend.".into(),
            sell_price: 150000,
            required_level: 60,
            ..Default::default()
        };
        // name(1) + ilvl(1) + bind(1) + slot(1) + stats(2) + effects(1) + flavor(1) + price(1) + req_level(1) = 10
        assert_eq!(tip.line_count(), 10);
    }

    #[test]
    fn name_color_from_quality() {
        let tip = ItemTooltip {
            quality: ItemQuality::Epic,
            ..Default::default()
        };
        let color = tip.name_color();
        // Epic border starts with "0.64"
        assert!((color[0] - 0.64).abs() < 0.01);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn has_armor_check() {
        let no = ItemTooltip::default();
        assert!(!no.has_armor());
        let yes = ItemTooltip {
            armor_value: 100,
            ..Default::default()
        };
        assert!(yes.has_armor());
    }
}
