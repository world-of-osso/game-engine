//! Cached Retail Blizzard_RaidWarning: four timed center-screen message slots.
use crate::{
    chat_data::monster_emote_text,
    ui::{
        strata::FrameStrata,
        widgets::font_string::{FontColor, GameFont},
    },
};
use shared::protocol::{ChatMessage, ChatType};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const MAX_WARNINGS: usize = 4;
const FADE_IN: f32 = 0.2;
const HOLD: f32 = 10.0;
const FADE_OUT: f32 = 3.0;
const LIFETIME: f32 = FADE_IN + HOLD + FADE_OUT;
const WIDTH: f32 = 800.0;
const TOP: f32 = 182.0;
const LINE_HEIGHT: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WarningKind {
    BossEmote,
    RaidWarning,
}

impl WarningKind {
    fn color(self) -> [f32; 3] {
        match self {
            Self::BossEmote => [1.0, 0.867, 0.0],
            Self::RaidWarning => [1.0, 0.282, 0.0],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WarningLine {
    pub text: String,
    pub kind: WarningKind,
    age: f32,
}

impl WarningLine {
    pub fn alpha(&self) -> f32 {
        if self.age < FADE_IN {
            return self.age / FADE_IN;
        }
        ((LIFETIME - self.age) / FADE_OUT).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RaidWarnings {
    pub lines: Vec<WarningLine>,
}

impl RaidWarnings {
    pub fn receive_chat(&mut self, message: &ChatMessage) {
        if matches!(message.channel, ChatType::RaidBossEmote(_)) {
            self.add(
                monster_emote_text(&message.content, &message.sender),
                WarningKind::BossEmote,
            );
        }
    }

    pub fn add(&mut self, text: String, kind: WarningKind) {
        if self.lines.len() == MAX_WARNINGS {
            self.lines.remove(0);
        }
        self.lines.push(WarningLine {
            text,
            kind,
            age: 0.0,
        });
    }

    pub fn advance(&mut self, delta: f32) {
        for line in &mut self.lines {
            line.age += delta.max(0.0);
        }
        self.lines.retain(|line| line.age < LIFETIME);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

struct DynName(String);

pub fn raid_warning_screen(ctx: &SharedContext) -> Element {
    let warnings = ctx
        .get::<RaidWarnings>()
        .expect("RaidWarnings in SharedContext");
    let lines: Element = (0..MAX_WARNINGS)
        .flat_map(|index| warning_line(index, warnings.lines.get(index)))
        .collect();
    rsx! {
        r#frame {
            name: "RaidWarningFrame",
            width: WIDTH,
            height: {LINE_HEIGHT * MAX_WARNINGS as f32},
            strata: FrameStrata::High,
            mouse_enabled: false,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: TOP,
            {lines}
        }
    }
}

fn warning_line(index: usize, line: Option<&WarningLine>) -> Element {
    let text = line.map_or("", |line| line.text.as_str());
    let hidden = line.is_none();
    let [r, g, b] = line.map_or([1.0, 0.867, 0.0], |line| line.kind.color());
    let color = FontColor::new(r, g, b, line.map_or(0.0, WarningLine::alpha));
    rsx! {
        fontstring {
            name: {DynName(format!("RaidWarningFrameLine{}", index + 1))},
            text,
            font: GameFont::FrizQuadrata,
            font_size: LINE_HEIGHT,
            font_color: color,
            justify_h: "CENTER",
            width: WIDTH,
            height: LINE_HEIGHT,
            hidden,
            mouse_enabled: false,
            strata: FrameStrata::High,
            pos_type: "absolute",
            left: 0.0,
            top: {LINE_HEIGHT * index as f32},
        }
    }
}
