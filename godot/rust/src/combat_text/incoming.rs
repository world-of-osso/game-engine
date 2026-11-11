//! Screen-anchored messages from Blizzard_CombatText.
use game_engine_core::combat_text_options::CombatTextOptions;
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

const DAMAGE_RED: [f32; 3] = [1.0, 0.1, 0.1];
const SPELL_PURPLE: [f32; 3] = [0.79, 0.3, 0.85];
const HEAL_GREEN: [f32; 3] = [0.1, 1.0, 0.1];
const SPELL_WHITE: [f32; 3] = [1.0, 1.0, 1.0];
pub(crate) const SCROLL_SECONDS: f32 = 1.9;
const FADE_START_SECONDS: f32 = 1.3;
pub(crate) const NORMAL_HEIGHT: f32 = 25.0;
pub(crate) const CRIT_MIN_HEIGHT: f32 = 30.0;
const CRIT_MAX_HEIGHT: f32 = 60.0;
pub(crate) const FONT_BOX_HEIGHT: f32 = 80.0;
pub(crate) const REFERENCE_VIEWPORT_HEIGHT: f32 = 768.0;
pub(crate) const REFERENCE_VIEWPORT_WIDTH: f32 = 1024.0;
pub(crate) const STAGGER_STEP_COUNT: i32 = 21;
pub(crate) const STAGGER_START: i32 = -10;
pub(crate) const TEXT_OFFSET_ADJUSTMENT: f32 = 80.0;
const CRIT_GROW_SECONDS: f32 = 0.05;
const CRIT_SHRINK_END_SECONDS: f32 = 0.2;
const SCROLL_DISTANCE: f32 = 225.0;
const FOUNTAIN_RADIUS: f32 = 150.0;
pub(crate) const TEXT_START_Y: f32 = 384.0;
pub(crate) const TEXT_MAX_SPREAD: f32 = 130.0;
pub(crate) const TEXT_POOL_SIZE: usize = 20;
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
    if !miss_is_enabled(kind, options) {
        return None;
    }
    Some(Message {
        text: miss_word(kind).to_owned(),
        color: miss_color(kind, spell),
        crit: false,
        stagger: !spell && miss_staggers(kind),
    })
}

fn miss_is_enabled(kind: MissKind, options: &CombatTextOptions) -> bool {
    let is_reduction = matches!(kind, MissKind::Block | MissKind::Resist | MissKind::Absorb);
    if is_reduction {
        options.reductions
    } else {
        options.avoidance
    }
}

fn miss_color(kind: MissKind, spell: bool) -> [f32; 3] {
    match (kind, spell) {
        (
            MissKind::Miss
            | MissKind::Dodge
            | MissKind::Parry
            | MissKind::Evade
            | MissKind::Immune
            | MissKind::Deflect
            | MissKind::Block
            | MissKind::Reflect,
            true,
        ) => SPELL_WHITE,
        (MissKind::Resist | MissKind::Absorb, true) => SPELL_PURPLE,
        _ => DAMAGE_RED,
    }
}

fn miss_staggers(kind: MissKind) -> bool {
    matches!(
        kind,
        MissKind::Miss | MissKind::Dodge | MissKind::Parry | MissKind::Evade
    )
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
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    if amount < 0 {
        formatted.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}

/// Nominal self-frame font height for this crit state and viewport.
pub(crate) fn font_height(crit: bool, viewport_height: f32) -> f32 {
    let height = if crit { CRIT_MIN_HEIGHT } else { NORMAL_HEIGHT };
    height * viewport_height / REFERENCE_VIEWPORT_HEIGHT
}

/// Choose the next self-frame line from the previous labels' bottom-origin screen positions.
pub(crate) fn stack_start_y(mode: i32, viewport_height: f32, previous: &[f32]) -> f32 {
    if previous.is_empty() {
        return TEXT_START_Y;
    }
    let scale_y = viewport_height / REFERENCE_VIEWPORT_HEIGHT;
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
        MotionFrame {
            position: self.position(age),
            alpha: self.alpha(age),
            height: self.text_height(age),
        }
    }

    fn position(self, age: f32) -> [f32; 2] {
        let time = age / SCROLL_SECONDS;
        let start_y = self.start[1] * self.viewport[1] / REFERENCE_VIEWPORT_HEIGHT;
        let (x, y) = if self.crit {
            (self.start[0], start_y)
        } else if self.mode != 1 && self.mode != 2 {
            fountain_position(self.start[0], start_y, self.x_dir, time)
        } else {
            standard_position(self.mode, self.start[0], start_y, time)
        };
        [self.viewport[0] * 0.5 + x, self.viewport[1] - y]
    }

    fn alpha(self, age: f32) -> f32 {
        let fade =
            ((age - FADE_START_SECONDS) / (SCROLL_SECONDS - FADE_START_SECONDS)).clamp(0.0, 1.0);
        1.0 - fade
    }

    fn text_height(self, age: f32) -> f32 {
        crit_height(age, self.crit) * self.viewport[1] / REFERENCE_VIEWPORT_HEIGHT
    }
}

fn fountain_position(start_x: f32, start_y: f32, x_dir: f32, time: f32) -> (f32, f32) {
    let angle = std::f32::consts::FRAC_PI_2 * time;
    (
        start_x - x_dir * FOUNTAIN_RADIUS * (1.0 - angle.cos()),
        start_y + FOUNTAIN_RADIUS * angle.sin(),
    )
}

fn standard_position(mode: i32, start_x: f32, start_y: f32, time: f32) -> (f32, f32) {
    let direction = if mode == 2 { -1.0 } else { 1.0 };
    (start_x, start_y + direction * SCROLL_DISTANCE * time)
}

fn crit_height(age: f32, crit: bool) -> f32 {
    if !crit {
        return NORMAL_HEIGHT;
    }
    if age <= CRIT_GROW_SECONDS {
        CRIT_MIN_HEIGHT + (CRIT_MAX_HEIGHT - CRIT_MIN_HEIGHT) * age / CRIT_GROW_SECONDS
    } else if age <= CRIT_SHRINK_END_SECONDS {
        CRIT_MAX_HEIGHT
            - (CRIT_MAX_HEIGHT - CRIT_MIN_HEIGHT) * (age - CRIT_GROW_SECONDS)
                / (CRIT_SHRINK_END_SECONDS - CRIT_GROW_SECONDS)
    } else {
        CRIT_MIN_HEIGHT
    }
}
