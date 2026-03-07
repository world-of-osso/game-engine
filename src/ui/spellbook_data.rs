#[derive(Debug, Clone, Copy)]
pub struct SpellbookSpell {
    pub id: u32,
    pub name: &'static str,
    pub passive: bool,
    pub icon_file_data_id: u32,
    pub cooldown_seconds: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SpellbookTab {
    pub name: &'static str,
    pub spells: &'static [SpellbookSpell],
}

const GENERAL_SPELLS: &[SpellbookSpell] = &[
    spell_icon(6603, "Auto Attack", 135274),
    spell_icon_cd(8690, "Hearthstone", 134414, 600.0),
    spell_icon_cd(7328, "Redemption", 135955, 600.0),
];

const PALADIN_SPELLS: &[SpellbookSpell] = &[
    spell_icon(35395, "Crusader Strike", 135891),
    spell_icon(19750, "Flash of Light", 135907),
    spell_icon(85673, "Word of Glory", 133192),
    spell_icon_cd(853, "Hammer of Justice", 135963, 60.0),
    spell_icon(275779, "Judgment", 135959),
    spell_icon(465, "Devotion Aura", 135893),
    spell_icon_cd(1022, "Blessing of Protection", 135964, 300.0),
    spell_icon(1044, "Blessing of Freedom", 135968),
    spell_icon_cd(642, "Divine Shield", 524354, 300.0),
    spell_icon_cd(633, "Lay on Hands", 135928, 600.0),
    spell_icon_cd(190784, "Divine Steed", 1360759, 45.0),
    spell_icon_cd(96231, "Rebuke", 523893, 15.0),
    spell_icon_cd(10326, "Turn Evil", 571559, 15.0),
    spell_icon(213644, "Cleanse Toxins", 135953),
    spell_icon_cd(6940, "Blessing of Sacrifice", 135966, 120.0),
    spell_icon_cd(31884, "Avenging Wrath", 135875, 120.0),
    spell_icon_cd(375576, "Divine Toll", 6035315, 60.0),
    spell_icon_cd(115750, "Blinding Light", 571553, 90.0),
    spell_icon(32223, "Crusader Aura", 135890),
    spell_icon(317920, "Concentration Aura", 135933),
    spell_icon(183435, "Retribution Aura", 135889),
    spell_icon(5502, "Sense Undead", 135974),
    spell_icon(121183, "Contemplation", 134916),
    passive_icon(137026, "Plate Specialization", 236216),
    passive_icon(385125, "Of Dusk and Dawn", 461859),
];

const PROTECTION_SPELLS: &[SpellbookSpell] = &[
    spell_icon_cd(31935, "Avenger's Shield", 135874, 15.0),
    spell_icon(53595, "Hammer of the Righteous", 236253),
    spell_icon_cd(26573, "Consecration", 135926, 12.0),
    spell_icon(53600, "Shield of the Righteous", 236265),
    spell_icon_cd(31850, "Ardent Defender", 135870, 120.0),
    spell_icon_cd(86659, "Guardian of Ancient Kings", 135919, 300.0),
    spell_icon_cd(62124, "Hand of Reckoning", 135984, 8.0),
    spell_icon_cd(498, "Divine Protection", 524353, 60.0),
    spell_icon_cd(327193, "Moment of Glory", 237537, 90.0),
    spell_icon_cd(378974, "Bastion of Light", 535594, 120.0),
    spell_icon_cd(387174, "Eye of Tyr", 1272527, 60.0),
    spell_icon(204019, "Blessed Hammer", 535595),
    passive_icon(85043, "Grand Crusader", 133176),
    passive_icon(152261, "Holy Shield", 1526019),
    passive_icon(76671, "Mastery: Divine Bulwark", 135923),
    passive_icon(280373, "Redoubt", 132359),
];

const HOLY_SPELLS: &[SpellbookSpell] = &[
    spell_icon(20473, "Holy Shock", 135972),
    spell_icon(82326, "Holy Light", 135981),
    spell_icon_cd(85222, "Light of Dawn", 461859, 12.0),
    spell_icon(4987, "Cleanse", 135949),
    spell_icon(53563, "Beacon of Light", 236247),
    spell_icon_cd(105809, "Holy Avenger", 571555, 180.0),
    spell_icon_cd(200652, "Tyr's Deliverance", 1122562, 90.0),
    passive_icon(53576, "Infusion of Light", 236254),
    passive_icon(183997, "Mastery: Lightbringer", 133041),
];

const RETRIBUTION_SPELLS: &[SpellbookSpell] = &[
    spell_icon(184575, "Blade of Justice", 1360757),
    spell_icon(85256, "Templar's Verdict", 461860),
    spell_icon_cd(255937, "Wake of Ashes", 1112939, 30.0),
    spell_icon_cd(184662, "Shield of Vengeance", 236264, 120.0),
    spell_icon_cd(343527, "Execution Sentence", 613954, 30.0),
    spell_icon_cd(343721, "Final Reckoning", 135878, 60.0),
    spell_icon_cd(383185, "Exorcism", 135903, 30.0),
    passive_icon(267344, "Art of War", 236246),
    passive_icon(231832, "Blade of Wrath", 1360757),
    passive_icon(269569, "Zeal", 135961),
];

pub const SPELLBOOK_TABS: &[SpellbookTab] = &[
    SpellbookTab {
        name: "General",
        spells: GENERAL_SPELLS,
    },
    SpellbookTab {
        name: "Paladin",
        spells: PALADIN_SPELLS,
    },
    SpellbookTab {
        name: "Protection",
        spells: PROTECTION_SPELLS,
    },
    SpellbookTab {
        name: "Holy",
        spells: HOLY_SPELLS,
    },
    SpellbookTab {
        name: "Retribution",
        spells: RETRIBUTION_SPELLS,
    },
];

const fn spell_icon(id: u32, name: &'static str, icon_file_data_id: u32) -> SpellbookSpell {
    SpellbookSpell {
        id,
        name,
        passive: false,
        icon_file_data_id,
        cooldown_seconds: 0.0,
    }
}

const fn passive_icon(id: u32, name: &'static str, icon_file_data_id: u32) -> SpellbookSpell {
    SpellbookSpell {
        id,
        name,
        passive: true,
        icon_file_data_id,
        cooldown_seconds: 0.0,
    }
}

const fn spell_icon_cd(
    id: u32,
    name: &'static str,
    icon_file_data_id: u32,
    cooldown_seconds: f32,
) -> SpellbookSpell {
    SpellbookSpell {
        id,
        name,
        passive: false,
        icon_file_data_id,
        cooldown_seconds,
    }
}
