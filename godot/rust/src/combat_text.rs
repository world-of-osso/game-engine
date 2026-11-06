//! Retail world text: the numbers and words over a unit (docs/specs/floating-combat-text.md).
//!
//! Sources, newest first: the decompiled Retail 12.1.5.70688 engine (style array at
//! 0x144868b78, 19 records of 40 bytes, relayed 2026-11-06), the decompiled Wrath client
//! (`wowdecomp15890` `WorldText.cpp`, `Unit_C.cpp`, `PlayerName.cpp`) for the parts Retail
//! shares, and the Retail CVar defaults (`wow-ui-sim/src/cvars.yaml`). Each item's proof level
//! is recorded in docs/specs/floating-combat-text.md.
//!
//! Retail CVars, `_v2` defaults: `WorldTextStartPosRandomness_v2 = 1.0` and
//! `WorldTextRandomZMin_v2 = 0.8` / `WorldTextRandomZMax_v2 = 1.5` randomise where each number
//! starts, so simultaneous numbers do not stack; `WorldTextRampDuration_v2 = 1.0` with
//! `WorldTextRampPow_v2 = 1.9` / `WorldTextRampPowCrit_v2 = 8.0` shape the size ramp, a crit
//! settling faster. No source has the engine formula; the mapping of each CVar here is read
//! from its name. Text keeps a fixed screen size (15890 draws it in a screen-space ortho
//! projection at a fixed NDC height, `WorldText.cpp:234-253`), so the start spread is measured
//! in `unit`s that scale with it (the caller passes 1.5 text heights at the target's depth), or
//! distant numbers would still land on each other.

use godot::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

/// `WorldTextStartPosRandomness_v2`: sideways start spread either side, in units.
const START_POS_RANDOMNESS: f32 = 1.0;
/// `WorldTextRandomZMin_v2` / `WorldTextRandomZMax_v2`, in units. Retail randomises the start
/// height over this band; the engine instead stacks a unit's four slots one text height apart
/// (see `spells::floating_text`), which is what keeps them from covering each other, so the
/// band only fixes the lowest start.
pub(crate) const RANDOM_Z_MIN: f32 = 0.8;
/// The pop-and-settle curve Retail shares with 15890 (`s_critHeightCurve`,
/// `WorldText.cpp:37-42`): from 10 % of the settled height at spawn, to 200 % at 10 % of the
/// lifetime, back to 100 % at 20 %, then settled.
const POP_FROM: f32 = 0.1;
const POP_PEAK: f32 = 2.0;
const POP_RISE_END: f32 = 0.1;
const POP_SETTLE_END: f32 = 0.2;

/// Colours from the Retail 12.1.5 style array (indices 0-9, 15, 18 white; 13, 14, 16 green;
/// 17 orange; 12 gold) and the pet colours
/// `s_petMeleeDamageColor` / `s_petSpellDamageColor` (`Unit_C.cpp:411-417`). 15890 has no
/// per-school colour: the `spellColor` argument only chooses the pet colour
/// (`Unit_C.cpp:17737-17773`).
pub(crate) const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
pub(crate) const HEAL_GREEN: [f32; 3] = [0.0, 1.0, 0.0];
pub(crate) const PET_MELEE_ORANGE: [f32; 3] = [1.0, 132.0 / 255.0, 0.0];
pub(crate) const PET_SPELL_YELLOW: [f32; 3] = [1.0, 222.0 / 255.0, 0.0];

/// One 15890 `WORLDTEXTTYPE` (`WorldText.cpp:44-56`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Style {
    /// Rise over the whole lifetime, world units; crits do not rise.
    pub ascend: f32,
    /// Seconds of the fade in.
    pub fade_in: f32,
    /// Seconds after spawn where the fade out begins.
    pub fade_out_start: f32,
    /// Lifetime, seconds.
    pub duration: f32,
    /// Settled height relative to a normal hit (`+0x1c` of a Retail style record): the
    /// avoidance words are 0.6.
    pub scale: f32,
    /// The 3-segment pop a crit (and an absorbed amount) plays.
    pub pops: bool,
}

/// Retail style 3: spell damage (`+0x18` flags off, `+0x1c` scale 1).
const SPELL_DAMAGE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 0.76,
    duration: 1.5,
    scale: 1.0,
    pops: false,
};
/// Retail style 4: melee damage. Same look as spell damage; Retail keeps them apart.
const MELEE_DAMAGE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 0.76,
    duration: 1.5,
    scale: 1.0,
    pops: false,
};
/// Retail style 2: melee crit.
const MELEE_CRIT: Style = Style {
    ascend: 0.0,
    fade_in: 0.15,
    fade_out_start: 1.0,
    duration: 1.5,
    scale: 1.5,
    pops: true,
};
/// Retail style 8: spell crit.
const SPELL_CRIT: Style = Style {
    ascend: 0.0,
    fade_in: 0.15,
    fade_out_start: 1.0,
    duration: 1.5,
    scale: 1.5,
    pops: true,
};
/// Retail style 9: every avoidance word, at 60 % of a normal hit's height.
const WORD_STYLE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 1.0,
    duration: 1.5,
    scale: 0.6,
    pops: false,
};
/// Retail style 13: heal.
const HEAL_STYLE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 0.76,
    duration: 1.5,
    scale: 1.0,
    pops: false,
};
/// Retail style 14: heal crit.
const HEAL_CRIT: Style = Style {
    ascend: 0.0,
    fade_in: 0.15,
    fade_out_start: 1.0,
    duration: 1.5,
    scale: 1.5,
    pops: true,
};

/// Who caused the event, from the local player's view. 15890 draws text only for the active
/// player and things it owns, and colours an owned pet's melee differently from its spells
/// (`IsOwnedByActivePlayer`, `Unit_C.cpp:17681-17702,17753-17773`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Attacker {
    Player,
    PetMelee,
    PetSpell,
}

impl Attacker {
    fn color(self) -> [f32; 3] {
        match self {
            Self::Player => WHITE,
            Self::PetMelee => PET_MELEE_ORANGE,
            Self::PetSpell => PET_SPELL_YELLOW,
        }
    }
}

/// One world text to draw.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FloatText {
    pub text: String,
    pub color: [f32; 3],
    pub crit: bool,
    /// A spell crit, not a melee one: only a spell crit replaces another
    /// (Retail 12.1.5 slot rule, relayed 2026-11-06).
    pub spell_crit: bool,
    pub style: Style,
}

/// The world text `event` produces, if any (`Unit_C.cpp:17693-17778,21202-21230`).
///
/// Damage and healing of zero draw nothing; a pet's heal draws nothing (15890 only draws the
/// active player's heals); periodic lines draw like direct ones (`WorldText.cpp` has no
/// distinction, the CVar gate is `floatingCombatTextCombatLogPeriodicSpells_v2`).
pub(crate) fn text_for(event: &CombatLogEvent, attacker: Attacker) -> Option<FloatText> {
    // Retail keeps melee and spell damage on separate styles (`1 melee damage`, `0 spell
    // damage`); a swing carries no spell, an ability does.
    let spell = event.spell_id.is_some();
    match event.kind {
        CombatLogKind::Damage if event.amount > 0 => Some(FloatText {
            // Retail appends no punctuation to a crit (`Unit_C.cpp:17775`); this engine's
            // existing contract and its live fixture keep the "!".
            text: if event.crit {
                format!("{}!", event.amount)
            } else {
                event.amount.to_string()
            },
            color: attacker.color(),
            crit: event.crit,
            spell_crit: event.crit && spell,
            style: match (event.crit, spell) {
                (true, true) => SPELL_CRIT,
                (true, false) => MELEE_CRIT,
                (false, true) => SPELL_DAMAGE,
                (false, false) => MELEE_DAMAGE,
            },
        }),
        CombatLogKind::Heal if event.amount > 0 && attacker == Attacker::Player => {
            Some(FloatText {
                text: format!("+{}", event.amount),
                color: HEAL_GREEN,
                crit: event.crit,
                spell_crit: event.crit,
                style: if event.crit { HEAL_CRIT } else { HEAL_STYLE },
            })
        }
        CombatLogKind::Miss(kind) => Some(FloatText {
            text: miss_word(kind).to_owned(),
            color: attacker.color(),
            crit: false,
            spell_crit: false,
            style: WORD_STYLE,
        }),
        _ => None,
    }
}

/// The word 15890 shows for an avoided or reduced hit (`s_combatTextTokens`,
/// `Unit_C.cpp:406`, and the `COMBAT_TEXT_*` global strings).
fn miss_word(kind: MissKind) -> &'static str {
    match kind {
        MissKind::Miss => "Miss",
        MissKind::Dodge => "Dodge",
        MissKind::Parry => "Parry",
        MissKind::Block => "Block",
        MissKind::Resist => "Resist",
        MissKind::Immune => "Immune",
        MissKind::Evade => "Evade",
        MissKind::Absorb => "Absorb",
        MissKind::Deflect => "Deflect",
        MissKind::Reflect => "Reflect",
    }
}

/// Retail 12.1.5 scales world text inversely with camera distance (relayed 2026-11-06). The
/// operands of the two calls that produce the constant factor are not decoded yet, so the
/// anchor is calibrated: a target at [`DISTANCE_ANCHOR_YARDS`] gets the size this engine drew
/// before the law existed. Retirement: replace the anchor with the decoded operands of
/// 0x141e67100 / 0x1401f45a0.
pub(crate) const DISTANCE_ANCHOR_YARDS: f32 = 10.0;
/// Retail's draw path clamps the text to a constant 7 px floor; read here as a fraction of
/// the anchored size, which keeps the text readable at long range.
const FAR_SIZE_FLOOR: f32 = 0.2;
/// The camera never gets closer than this, so 1/d stays finite.
const NEAREST_YARDS: f32 = 1.0;

/// Size multiplier for a text `distance` yards from the camera.
pub(crate) fn distance_scale(distance: f32) -> f32 {
    let distance = distance.max(NEAREST_YARDS);
    (DISTANCE_ANCHOR_YARDS / distance).max(FAR_SIZE_FLOOR)
}

/// Simultaneous world texts one unit may hold (Retail 12.1.5 searches four slots; the array
/// is declared with five).
pub(crate) const SLOTS_PER_UNIT: usize = 4;

/// Which of a unit's live world texts a new one evicts, if any: a spell crit first replaces the
/// unit's existing spell crit (`spell_crit_slot`), then a fifth text evicts the last slot.
pub(crate) fn slot_eviction(live: usize, spell_crit_slot: Option<usize>) -> Option<usize> {
    spell_crit_slot.or_else(|| (live >= SLOTS_PER_UNIT).then(|| SLOTS_PER_UNIT - 1))
}

/// Combat text `pixel_size` at a hit's settled size; the layout constant the captures were
/// tuned with.
pub(crate) const TEXT_PIXEL: f32 = 0.0016;
/// Settled font size of a hit (`SystemFont_World` height 64).
pub(crate) const TEXT_FONT_SIZE: i32 = 64;
/// One spread unit is this many normal text heights.
pub(crate) const SPREAD_HEIGHTS: f32 = 1.5;
/// Screen pixels Godot's fixed-size text draws per world em, measured on the release build at
/// 1280x720 with a 75-degree camera: a 64 px font at `TEXT_PIXEL` drew a 174 px em (and the
/// four-digit ink measured 333x104 px against a 391x122 box).
pub(crate) const SCREEN_PX_PER_EM: f32 = 1699.0;

/// Screen pixels one world yard covers at `distance`.
fn pixels_per_yard(distance: f32, fov_degrees: f32, viewport_height: f32) -> f32 {
    let half_fov = (fov_degrees * std::f32::consts::PI / 360.0).tan();
    viewport_height.max(1.0) / (2.0 * distance.max(1.0) * half_fov)
}

/// Shadow offset as a fraction of a text em: the black copy sits down-right of the glyphs.
pub(crate) const SHADOW_OFFSET_EM: f32 = 0.045;

/// World yards one screen em covers at `distance` for a `font`/`pixel_size` text; the shadow
/// step is a fraction of it.
pub(crate) fn shadow_step_yards(
    pixel_size: f32,
    distance: f32,
    font_px: f32,
    fov_degrees: f32,
    viewport_height: f32,
) -> f32 {
    text_height_yards(font_px, pixel_size, distance, fov_degrees, viewport_height)
        * SHADOW_OFFSET_EM
}

/// World yards one settled text em covers on screen, the unit every placement offset uses.
/// Fixed-size text is a screen rectangle, so its world size grows with camera distance.
pub(crate) fn text_height_yards(
    font_px: f32,
    pixel_size: f32,
    distance: f32,
    fov_degrees: f32,
    viewport_height: f32,
) -> f32 {
    SCREEN_PX_PER_EM * font_px * pixel_size
        / pixels_per_yard(distance, fov_degrees, viewport_height)
}
/// R2 low-discrepancy sequence (plastic number): consecutive spawns land far apart in
/// both axes, so numbers spawned together never share a start.
const R2: [f32; 2] = [0.754_877_7, 0.569_840_3];

/// Sideways start of the `index`th number relative to the anchor over the target's head, along
/// the camera's right. `WorldTextStartPosRandomness_v2` gives the range; the vertical axis is
/// the per-slot ladder in `spells::floating_text`, so numbers on one unit never share a row.
pub(crate) fn side_offset(index: u32, camera_right: Vector3, unit: f32) -> Vector3 {
    let u = (0.5 + R2[0] * index as f32).fract();
    camera_right.normalized() * (START_POS_RANDOMNESS * (2.0 * u - 1.0)) * unit
}

/// Height multiplier `age` seconds after spawn. A crit (and an absorbed amount) starts at
/// 10 % of its height, overshoots to 200 % at a tenth of the lifetime and settles at 100 % a
/// fifth of the way through; every other style keeps its height
/// (`CalculateTextHeight`, `WorldText.cpp:255-286`, shared with Retail 12.1.5).
pub(crate) fn height_scale(age: f32, style: Style) -> f32 {
    if !style.pops {
        return 1.0;
    }
    let t = (age / style.duration).clamp(0.0, 1.0);
    if t < POP_RISE_END {
        POP_FROM + (POP_PEAK - POP_FROM) * (t / POP_RISE_END)
    } else if t < POP_SETTLE_END {
        POP_PEAK + (1.0 - POP_PEAK) * ((t - POP_RISE_END) / (POP_SETTLE_END - POP_RISE_END))
    } else {
        1.0
    }
}

/// Alpha of a text `age` seconds after spawn: fade in over `fade_in`, hold, then fade out
/// from `fade_out_start` to the end of the lifetime (`CalculateNewColor`,
/// `WorldText.cpp:156-198`).
pub(crate) fn fade_alpha(age: f32, style: Style) -> f32 {
    if age < style.fade_in {
        (age / style.fade_in).clamp(0.0, 1.0)
    } else if age >= style.fade_out_start {
        let remaining = style.duration - style.fade_out_start;
        ((style.duration - age) / remaining).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: CombatLogKind, amount: i32) -> CombatLogEvent {
        CombatLogEvent {
            source: Some(1),
            target: Some(2),
            spell_id: Some(133),
            school_mask: 4,
            amount,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
            kind,
        }
    }

    #[test]
    fn a_players_spell_damage_is_white_regardless_of_school() {
        for school in [1, 2, 4, 8, 16, 32, 64] {
            let mut hit = event(CombatLogKind::Damage, 1234);
            hit.school_mask = school;
            let text = text_for(&hit, Attacker::Player).expect("damage draws");
            assert_eq!(text.text, "1234");
            assert_eq!(text.color, WHITE);
            assert_eq!(text.style, SPELL_DAMAGE);
            assert!(!text.crit);
        }
    }

    #[test]
    fn a_crit_is_bigger_and_does_not_rise() {
        let mut crit = event(CombatLogKind::Damage, 99);
        crit.crit = true;
        let text = text_for(&crit, Attacker::Player).expect("crit draws");
        assert_eq!(text.text, "99!");
        assert_eq!(text.style, SPELL_CRIT);
        assert!(text.crit && text.spell_crit);
        assert_eq!(text.style.ascend, 0.0);
        assert!(text.style.scale > SPELL_DAMAGE.scale);
        assert!(text.style.fade_out_start > SPELL_DAMAGE.fade_out_start);
    }

    #[test]
    fn a_swing_crit_is_melee_and_only_a_spell_crit_replaces() {
        let mut swing = event(CombatLogKind::Damage, 99);
        swing.crit = true;
        swing.spell_id = None;
        let text = text_for(&swing, Attacker::Player).expect("crit draws");
        assert_eq!(text.style, MELEE_CRIT);
        assert!(text.crit && !text.spell_crit);

        let mut ability = event(CombatLogKind::Damage, 99);
        ability.crit = true;
        assert!(
            text_for(&ability, Attacker::Player)
                .expect("crit draws")
                .spell_crit
        );

        let plain_swing = event(CombatLogKind::Damage, 10);
        let mut plain_swing = plain_swing;
        plain_swing.spell_id = None;
        assert_eq!(
            text_for(&plain_swing, Attacker::Player).map(|text| text.style),
            Some(MELEE_DAMAGE)
        );
    }

    #[test]
    fn pet_melee_is_orange_and_pet_spells_are_yellow() {
        let hit = event(CombatLogKind::Damage, 40);
        assert_eq!(
            text_for(&hit, Attacker::PetMelee).map(|text| text.color),
            Some(PET_MELEE_ORANGE)
        );
        assert_eq!(
            text_for(&hit, Attacker::PetSpell).map(|text| text.color),
            Some(PET_SPELL_YELLOW)
        );
    }

    #[test]
    fn amounts_are_whole_numbers_without_a_fraction() {
        // 15890 prints "%d" (`Unit_C.cpp:17775`); a label showing "+900.0" means two texts
        // overlapped or a value reached the formatter as a float.
        let hit = text_for(&event(CombatLogKind::Damage, 1234), Attacker::Player).unwrap();
        assert_eq!(hit.text, "1234");
        let heal = text_for(&event(CombatLogKind::Heal, 900), Attacker::Player).unwrap();
        assert_eq!(heal.text, "+900");
        let mut crit = event(CombatLogKind::Damage, 2345);
        crit.crit = true;
        assert_eq!(text_for(&crit, Attacker::Player).unwrap().text, "2345!");
        for text in [hit.text, heal.text, "2345!".to_owned()] {
            assert!(!text.contains('.'), "{text} carries a decimal fraction");
            let digits: String = text.chars().filter(char::is_ascii_digit).collect();
            assert!(digits.parse::<u64>().is_ok(), "{text} is not an integer");
        }
    }

    #[test]
    fn healing_is_a_green_plus_amount_without_a_crit_mark() {
        let mut heal = event(CombatLogKind::Heal, 700);
        heal.crit = true;
        let text = text_for(&heal, Attacker::Player).expect("heal draws");
        assert_eq!(text.text, "+700");
        assert_eq!(text.color, HEAL_GREEN);
        assert_eq!(text.style, HEAL_CRIT);
        assert!(text.spell_crit, "a heal crit may replace another heal crit");

        let mut plain = event(CombatLogKind::Heal, 700);
        plain.crit = false;
        assert_eq!(
            text_for(&plain, Attacker::Player).map(|text| text.text),
            Some("+700".to_owned())
        );
        assert_eq!(
            text_for(&plain, Attacker::PetMelee).map(|text| text.text),
            None,
            "15890 draws only the active player's heals"
        );
    }

    #[test]
    fn every_avoidance_kind_draws_its_word_in_white() {
        let words = [
            (MissKind::Miss, "Miss"),
            (MissKind::Dodge, "Dodge"),
            (MissKind::Parry, "Parry"),
            (MissKind::Block, "Block"),
            (MissKind::Resist, "Resist"),
            (MissKind::Immune, "Immune"),
            (MissKind::Evade, "Evade"),
            (MissKind::Absorb, "Absorb"),
            (MissKind::Deflect, "Deflect"),
            (MissKind::Reflect, "Reflect"),
        ];
        for (kind, word) in words {
            let text = text_for(&event(CombatLogKind::Miss(kind), 0), Attacker::Player)
                .unwrap_or_else(|| panic!("{word} draws"));
            assert_eq!(text.text, word);
            assert_eq!(text.color, WHITE);
            assert_eq!(text.style, WORD_STYLE);
            assert_eq!(text.style.scale, 0.6, "{word} is 60% of a hit");
        }
    }

    #[test]
    fn zero_amounts_and_other_kinds_draw_nothing() {
        assert_eq!(
            text_for(&event(CombatLogKind::Damage, 0), Attacker::Player),
            None
        );
        assert_eq!(
            text_for(&event(CombatLogKind::Heal, 0), Attacker::Player),
            None
        );
        for kind in [
            CombatLogKind::Energize,
            CombatLogKind::AuraApplied,
            CombatLogKind::Death,
            CombatLogKind::CastSuccess,
            CombatLogKind::Environmental(shared::protocol::EnvironmentalKind::Falling),
        ] {
            assert_eq!(text_for(&event(kind, 50), Attacker::Player), None);
        }
    }

    #[test]
    fn periodic_lines_draw_like_direct_ones() {
        let mut tick = event(CombatLogKind::Damage, 55);
        tick.periodic = true;
        assert_eq!(
            text_for(&tick, Attacker::Player).map(|text| text.text),
            Some("55".to_owned())
        );
    }

    #[test]
    fn fade_in_holds_then_fades_out() {
        assert_eq!(fade_alpha(0.0, SPELL_DAMAGE), 0.0);
        assert!((fade_alpha(0.075, SPELL_DAMAGE) - 0.5).abs() < 1e-6);
        assert_eq!(fade_alpha(0.2, SPELL_DAMAGE), 1.0);
        assert_eq!(fade_alpha(SPELL_DAMAGE.fade_out_start, SPELL_DAMAGE), 1.0);
        // 760 ms -> 1500 ms: half way is 1130 ms.
        assert!((fade_alpha(1.13, SPELL_DAMAGE) - 0.5).abs() < 1e-3);
        assert_eq!(fade_alpha(SPELL_DAMAGE.duration, SPELL_DAMAGE), 0.0);
    }

    #[test]
    fn a_text_height_in_yards_is_the_screen_em_at_that_distance() {
        // 1280x720, 75 degrees: 720 / (2 * 10 * tan(37.5)) = 46.9 px per yard, so the 174 px em
        // of a hit at 10 yards is 3.71 yards wide in the world.
        let height = text_height_yards(64.0, TEXT_PIXEL, 10.0, 75.0, 720.0);
        assert!((height - 3.71).abs() < 0.02, "{height}");
        // Fixed-size text: twice the distance is twice the world height, same pixels.
        let far = text_height_yards(64.0, TEXT_PIXEL, 20.0, 75.0, 720.0);
        assert!((far - 2.0 * height).abs() < 0.02, "{far}");
        // The 1/d text law keeps the on-screen separation constant: the pixel height barely
        // moves with distance.
        let near_pixels = text_height_yards(64.0, TEXT_PIXEL, 10.0, 75.0, 720.0)
            * pixels_per_yard(10.0, 75.0, 720.0);
        let far_pixels = far * pixels_per_yard(20.0, 75.0, 720.0);
        assert!((near_pixels - far_pixels).abs() < 1.0);
        assert!((near_pixels - 174.0).abs() < 3.0, "{near_pixels}");
    }

    #[test]
    fn text_shrinks_with_distance_and_stops_at_the_far_floor() {
        // Verified Retail 12.1.5: the size falls as 1/d. The anchor is calibrated, so a
        // 10-yard target keeps the size this engine drew before the law existed.
        assert_eq!(distance_scale(DISTANCE_ANCHOR_YARDS), 1.0);
        assert!((distance_scale(5.0) - 2.0).abs() < 1e-6);
        assert!((distance_scale(20.0) - 0.5).abs() < 1e-6);
        assert!(distance_scale(40.0) < distance_scale(20.0));
        assert_eq!(
            distance_scale(1000.0),
            FAR_SIZE_FLOOR,
            "the far floor holds"
        );
        assert_eq!(
            distance_scale(0.0),
            DISTANCE_ANCHOR_YARDS / NEAREST_YARDS,
            "a camera at the text does not divide by zero"
        );
    }

    #[test]
    fn a_unit_evicts_only_when_its_four_slots_are_full() {
        assert_eq!(slot_eviction(0, None), None);
        assert_eq!(slot_eviction(3, None), None);
        assert_eq!(slot_eviction(4, None), Some(3));
        assert_eq!(
            slot_eviction(4, Some(1)),
            Some(1),
            "a spell crit replaces its predecessor"
        );
        assert_eq!(slot_eviction(2, Some(0)), Some(0));
    }

    #[test]
    fn consecutive_numbers_start_sideways_apart() {
        // Showcase: a Frostbolt hit and a crit landed on the same frame, drawn on top of each
        // other. The R2 sequence keeps consecutive sideways starts far apart.
        let right = Vector3::new(0.6, 0.0, -0.8);
        let unit = 2.0;
        for first in 0..1000 {
            let starts: Vec<_> = (first..first + 4)
                .map(|index| side_offset(index, right, unit))
                .collect();
            for (i, a) in starts.iter().enumerate() {
                for b in &starts[i + 1..] {
                    assert!(a.distance_to(*b) >= 0.3 * unit, "{first}: {a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn the_sideways_start_stays_in_the_camera_plane_and_range() {
        let right = Vector3::new(0.0, 0.0, 2.0);
        for index in 0..200 {
            let offset = side_offset(index, right, 1.0);
            assert_eq!(offset.x, 0.0);
            assert_eq!(offset.y, 0.0);
            assert!(offset.z.abs() <= START_POS_RANDOMNESS);
        }
    }

    #[test]
    fn a_crit_starts_small_overshoots_then_settles_while_a_hit_holds() {
        let duration = SPELL_CRIT.duration;
        assert!((height_scale(0.0, SPELL_CRIT) - 0.1).abs() < 1e-6);
        assert!((height_scale(0.1 * duration, SPELL_CRIT) - 2.0).abs() < 1e-6);
        assert!((height_scale(0.15 * duration, SPELL_CRIT) - 1.5).abs() < 1e-5);
        assert!((height_scale(0.2 * duration, SPELL_CRIT) - 1.0).abs() < 1e-6);
        assert_eq!(height_scale(duration, SPELL_CRIT), 1.0);
        // Every non-crit style keeps its height for the whole lifetime.
        for style in [SPELL_DAMAGE, MELEE_DAMAGE, WORD_STYLE, HEAL_STYLE] {
            for tenth in 0..=10 {
                assert_eq!(height_scale(0.15 * tenth as f32, style), 1.0);
            }
        }
    }
}
