//! Retail world text: the numbers and words over a unit (docs/specs/floating-combat-text.md).
//!
//! The decompiled Wrath client (`wowdecomp15890` `WorldText.cpp`, `Unit_C.cpp`,
//! `PlayerName.cpp`) is the only local source for the renderer; the Retail CVar defaults
//! (`wow-ui-sim/src/cvars.yaml`) supply the modern numbers, and each item's proof level is
//! recorded in the spec.
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
/// `WorldTextRandomZMin_v2` / `WorldTextRandomZMax_v2`, in units.
const RANDOM_Z_MIN: f32 = 0.8;
const RANDOM_Z_MAX: f32 = 1.5;
/// `WorldTextRampDuration_v2`, seconds.
const RAMP_SECS: f32 = 1.0;
/// `WorldTextRampPow_v2` / `WorldTextRampPowCrit_v2`.
const RAMP_POW: f32 = 1.9;
const RAMP_POW_CRIT: f32 = 8.0;
/// Extra size at spawn, decaying over the ramp. No source states this constant; 15890's
/// crit height curve peaks at 2.0× the settled height (`s_critHeightCurve`,
/// `WorldText.cpp:37-42`), which bounds this at 1.0.
const RAMP_POP: f32 = 0.5;

/// Colours from 15890 `s_worldTextTypes[]` (`WorldText.cpp:44-56`) and the pet colours
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
}

/// 15890 type 0 (`WorldText.cpp:46`): damage.
const HIT_STYLE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 0.76,
    duration: 1.5,
};
/// 15890 type 2 (`WorldText.cpp:48`): damage crit.
const CRIT_STYLE: Style = Style {
    ascend: 0.0,
    fade_in: 0.15,
    fade_out_start: 1.0,
    duration: 1.5,
};
/// 15890 type 3 (`WorldText.cpp:49`): Miss, Dodge, Parry, Block, Evade, Immune, Deflect,
/// Reflect.
const WORD_STYLE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 1.0,
    duration: 1.5,
};
/// 15890 type 6 (`WorldText.cpp:52`): heal.
const HEAL_STYLE: Style = Style {
    ascend: 2.0,
    fade_in: 0.15,
    fade_out_start: 0.76,
    duration: 1.5,
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
    pub style: Style,
}

/// The world text `event` produces, if any (`Unit_C.cpp:17693-17778,21202-21230`).
///
/// Damage and healing of zero draw nothing; a pet's heal draws nothing (15890 only draws the
/// active player's heals); periodic lines draw like direct ones (`WorldText.cpp` has no
/// distinction, the CVar gate is `floatingCombatTextCombatLogPeriodicSpells_v2`).
pub(crate) fn text_for(event: &CombatLogEvent, attacker: Attacker) -> Option<FloatText> {
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
            style: if event.crit { CRIT_STYLE } else { HIT_STYLE },
        }),
        CombatLogKind::Heal if event.amount > 0 && attacker == Attacker::Player => {
            Some(FloatText {
                text: format!("+{}", event.amount),
                color: HEAL_GREEN,
                crit: event.crit,
                style: if event.crit { CRIT_STYLE } else { HEAL_STYLE },
            })
        }
        CombatLogKind::Miss(kind) => Some(FloatText {
            text: miss_word(kind).to_owned(),
            color: attacker.color(),
            crit: false,
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

/// R2 low-discrepancy sequence (plastic number): consecutive spawns land far apart in
/// both axes, so numbers spawned together never share a start.
const R2: [f32; 2] = [0.754_877_7, 0.569_840_3];

/// Where the `index`th number starts relative to the anchor over the target's head:
/// sideways along the camera's right, and up by the random Z range, `unit` long each.
pub(crate) fn start_offset(index: u32, camera_right: Vector3, unit: f32) -> Vector3 {
    let [u, v] = R2.map(|alpha| (0.5 + alpha * index as f32).fract());
    let side = camera_right.normalized() * (START_POS_RANDOMNESS * (2.0 * u - 1.0));
    (side + Vector3::UP * ((RANDOM_Z_MAX - RANDOM_Z_MIN) * v)) * unit
}

/// Size multiplier `age` seconds after spawn: pops larger, then ramps down to 1.
pub(crate) fn ramp_scale(age: f32, crit: bool) -> f32 {
    let remaining = (1.0 - age / RAMP_SECS).clamp(0.0, 1.0);
    let pow = if crit { RAMP_POW_CRIT } else { RAMP_POW };
    1.0 + RAMP_POP * remaining.powf(pow)
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
            assert_eq!(text.style, HIT_STYLE);
            assert!(!text.crit);
        }
    }

    #[test]
    fn a_crit_is_bigger_and_does_not_rise() {
        let mut crit = event(CombatLogKind::Damage, 99);
        crit.crit = true;
        let text = text_for(&crit, Attacker::Player).expect("crit draws");
        assert_eq!(text.text, "99!");
        assert_eq!(text.style, CRIT_STYLE);
        assert!(text.crit);
        assert_eq!(text.style.ascend, 0.0);
        assert!(text.style.fade_out_start > HIT_STYLE.fade_out_start);
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
    fn healing_is_a_green_plus_amount_without_a_crit_mark() {
        let mut heal = event(CombatLogKind::Heal, 700);
        heal.crit = true;
        let text = text_for(&heal, Attacker::Player).expect("heal draws");
        assert_eq!(text.text, "+700");
        assert_eq!(text.color, HEAL_GREEN);
        assert_eq!(text.style, CRIT_STYLE);

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
        assert_eq!(fade_alpha(0.0, HIT_STYLE), 0.0);
        assert!((fade_alpha(0.075, HIT_STYLE) - 0.5).abs() < 1e-6);
        assert_eq!(fade_alpha(0.2, HIT_STYLE), 1.0);
        assert_eq!(fade_alpha(HIT_STYLE.fade_out_start, HIT_STYLE), 1.0);
        // 760 ms -> 1500 ms: half way is 1130 ms.
        assert!((fade_alpha(1.13, HIT_STYLE) - 0.5).abs() < 1e-3);
        assert_eq!(fade_alpha(HIT_STYLE.duration, HIT_STYLE), 0.0);
    }

    #[test]
    fn numbers_spawned_together_start_apart() {
        // Showcase: a Frostbolt hit and a crit landed on the same frame, drawn on top of
        // each other. Any four numbers in a row start at least half a unit apart.
        let right = Vector3::new(0.6, 0.0, -0.8);
        let unit = 2.0;
        for first in 0..1000 {
            let starts: Vec<_> = (first..first + 4)
                .map(|index| start_offset(index, right, unit))
                .collect();
            for (i, a) in starts.iter().enumerate() {
                for b in &starts[i + 1..] {
                    assert!(a.distance_to(*b) >= 0.5 * unit, "{first}: {a} vs {b}");
                }
            }
        }
    }

    #[test]
    fn starts_stay_in_the_camera_plane_within_the_random_ranges() {
        let right = Vector3::new(0.0, 0.0, 2.0);
        for index in 0..200 {
            let offset = start_offset(index, right, 1.0);
            assert_eq!(offset.x, 0.0);
            assert!(offset.z.abs() <= START_POS_RANDOMNESS);
            assert!((0.0..=RANDOM_Z_MAX - RANDOM_Z_MIN).contains(&offset.y));
        }
    }

    #[test]
    fn crits_pop_and_settle_faster_than_hits() {
        assert_eq!(ramp_scale(0.0, true), 1.0 + RAMP_POP);
        assert_eq!(ramp_scale(0.0, false), 1.0 + RAMP_POP);
        assert!(ramp_scale(0.3, true) < ramp_scale(0.3, false));
        assert!(ramp_scale(0.3, true) < 1.03);
        assert_eq!(ramp_scale(RAMP_SECS, false), 1.0);
        assert_eq!(ramp_scale(2.0, true), 1.0);
    }
}
