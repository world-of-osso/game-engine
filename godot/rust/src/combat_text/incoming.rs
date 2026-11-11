//! Screen-anchored messages from Blizzard_CombatText.
use game_engine_core::combat_text_options::CombatTextOptions;
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

const DAMAGE_RED: [f32; 3] = [1.0, 0.1, 0.1];
const SPELL_PURPLE: [f32; 3] = [0.79, 0.3, 0.85];
const HEAL_GREEN: [f32; 3] = [0.1, 1.0, 0.1];
const SPELL_WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const SCROLL_SECONDS: f32 = 1.9;
const FADE_START_SECONDS: f32 = 1.3;
const NORMAL_HEIGHT: f32 = 25.0;
const CRIT_MIN_HEIGHT: f32 = 30.0;
const CRIT_MAX_HEIGHT: f32 = 60.0;
const CRIT_GROW_SECONDS: f32 = 0.05;
const CRIT_SHRINK_END_SECONDS: f32 = 0.2;
const SCROLL_DISTANCE: f32 = 225.0;
const FOUNTAIN_RADIUS: f32 = 150.0;
const TEXT_START_Y: f32 = 384.0;
const TEXT_MAX_SPREAD: f32 = 130.0;
const TEXT_SPACING: f32 = 10.0;
const MESSAGE_HALF_HEIGHT: f32 = 16.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    pub text: String,
    pub color: [f32; 3],
    pub crit: bool,
    pub stagger: bool,
}

/// Translate the combat-log subset carried by the client into self-frame messages.
pub(crate) fn message_for(
    event: &CombatLogEvent,
    options: &CombatTextOptions,
    healer_name: Option<&str>,
) -> Option<Message> {
    if !options.self_enabled {
        return None;
    }
    match event.kind {
        CombatLogKind::Damage if event.amount > 0 => Some(incoming_damage(event, options)),
        CombatLogKind::Heal if event.amount > 0 => Some(incoming_heal(event, options, healer_name)),
        CombatLogKind::Miss(kind) => incoming_miss(event, options, kind),
        _ => None,
    }
}

fn incoming_damage(event: &CombatLogEvent, options: &CombatTextOptions) -> Message {
    let spell = event.spell_id.is_some();
    let (text, color, stagger) = if event.blocked > 0 && !event.crit {
        (
            format!("{} (Block)", grouped(event.blocked)),
            if spell { SPELL_PURPLE } else { DAMAGE_RED },
            false,
        )
    } else {
        let mut text = format!("-{}", grouped(event.amount));
        if options.reductions {
            for (amount, label) in [(event.absorbed, "absorbed"), (event.resisted, "resisted")] {
                if amount > 0 {
                    text.push_str(&format!("  ({} {label})", grouped(amount)));
                }
            }
        }
        (
            text,
            if spell { SPELL_PURPLE } else { DAMAGE_RED },
            !event.crit && !spell,
        )
    };
    Message {
        text,
        color,
        crit: event.crit,
        stagger,
    }
}

fn incoming_heal(
    event: &CombatLogEvent,
    options: &CombatTextOptions,
    healer_name: Option<&str>,
) -> Message {
    let mut text = format!("+{}", grouped(event.amount));
    let show_name =
        options.friendly_healers && healer_name.is_some() && (!event.periodic || event.crit);
    if show_name {
        text.push_str(&format!(" [{}]", healer_name.unwrap_or_default()));
    }
    if options.healing_absorb_self && event.absorbed > 0 {
        text.push_str(&format!("  ({} absorbed)", grouped(event.absorbed)));
    }
    Message {
        text,
        color: HEAL_GREEN,
        crit: event.crit,
        stagger: false,
    }
}

fn incoming_miss(
    event: &CombatLogEvent,
    options: &CombatTextOptions,
    kind: MissKind,
) -> Option<Message> {
    let spell = event.spell_id.is_some();
    let is_reduction = matches!(kind, MissKind::Block | MissKind::Resist | MissKind::Absorb);
    let allowed = if is_reduction {
        options.reductions
    } else {
        options.avoidance
    };
    if !allowed {
        return None;
    }
    let color = match (kind, spell) {
        (
            MissKind::Miss
            | MissKind::Dodge
            | MissKind::Parry
            | MissKind::Evade
            | MissKind::Immune
            | MissKind::Deflect,
            true,
        ) => SPELL_WHITE,
        (MissKind::Resist | MissKind::Absorb, true) => SPELL_PURPLE,
        (MissKind::Block, true) => SPELL_WHITE,
        (MissKind::Reflect, true) => SPELL_WHITE,
        _ => DAMAGE_RED,
    };
    Some(Message {
        text: miss_word(kind).to_owned(),
        color,
        crit: false,
        stagger: !spell
            && matches!(
                kind,
                MissKind::Miss | MissKind::Dodge | MissKind::Parry | MissKind::Evade
            ),
    })
}

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

fn grouped(amount: i32) -> String {
    let digits = amount.unsigned_abs().to_string();
    let first_group = digits.len() % 3;
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    if amount < 0 {
        formatted.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (index - first_group).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}

/// Choose the next self-frame line from the previous labels' bottom-origin screen positions.
pub(crate) fn stack_start_y(mode: i32, viewport_height: f32, previous: &[f32]) -> f32 {
    if previous.is_empty() {
        return TEXT_START_Y;
    }
    let scale_y = viewport_height / 768.0;
    let gap = MESSAGE_HALF_HEIGHT / scale_y + TEXT_SPACING;
    previous
        .iter()
        .copied()
        .fold(TEXT_START_Y, |position, previous| {
            if mode == 2 {
                position.max(previous + gap)
            } else {
                position.min(previous - gap)
            }
        })
}

#[derive(Clone, Copy)]
pub(crate) struct Motion {
    pub mode: i32,
    pub crit: bool,
    /// Retail's bottom-origin start position, scaled by viewport height/768.
    pub start: [f32; 2],
    pub x_dir: f32,
    pub viewport: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionFrame {
    pub position: [f32; 2],
    pub alpha: f32,
    pub height: f32,
}

impl Motion {
    pub(crate) fn frame(self, age: f32) -> MotionFrame {
        let age = age.clamp(0.0, SCROLL_SECONDS);
        let time = age / SCROLL_SECONDS;
        let start_y = self.start[1] * self.viewport[1] / 768.0;
        let (x, y) = if self.crit {
            (self.start[0], start_y)
        } else if self.mode != 1 && self.mode != 2 {
            let angle = std::f32::consts::FRAC_PI_2 * time;
            (
                self.start[0] - self.x_dir * FOUNTAIN_RADIUS * (1.0 - angle.cos()),
                start_y + FOUNTAIN_RADIUS * angle.sin(),
            )
        } else {
            let direction = if self.mode == 2 { -1.0 } else { 1.0 };
            (self.start[0], start_y + direction * SCROLL_DISTANCE * time)
        };
        let fade =
            ((age - FADE_START_SECONDS) / (SCROLL_SECONDS - FADE_START_SECONDS)).clamp(0.0, 1.0);
        let height = if self.crit {
            if age <= CRIT_GROW_SECONDS {
                CRIT_MIN_HEIGHT + (CRIT_MAX_HEIGHT - CRIT_MIN_HEIGHT) * age / CRIT_GROW_SECONDS
            } else if age <= CRIT_SHRINK_END_SECONDS {
                CRIT_MAX_HEIGHT
                    - (CRIT_MAX_HEIGHT - CRIT_MIN_HEIGHT) * (age - CRIT_GROW_SECONDS)
                        / (CRIT_SHRINK_END_SECONDS - CRIT_GROW_SECONDS)
            } else {
                CRIT_MIN_HEIGHT
            }
        } else {
            NORMAL_HEIGHT
        } * self.viewport[1]
            / 768.0;
        MotionFrame {
            position: [self.viewport[0] * 0.5 + x, self.viewport[1] - y],
            alpha: 1.0 - fade,
            height,
        }
    }
}
