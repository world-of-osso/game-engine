//! Retail Status Text (`statusTextDisplay`): the value text of unit frame health and
//! power bars, as `TextStatusBarMixin:UpdateTextStringWithValues` writes it
//! (Blizzard_TextStatusBar/TextStatusBar.lua:101-215).

use serde::{Deserialize, Serialize};

/// Settings → Interface → Display "Status Text" dropdown, CVar `statusTextDisplay`
/// (Blizzard_SettingsDefinitions_Frame/Interface.lua:57-105). Retail default `NONE`
/// (wow-ui-sim cvars.yaml:1413; the proxy setting's `defaultValue = 4`, Interface.lua:101).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum StatusTextDisplay {
    /// `STATUS_TEXT_VALUE` "Numeric Value".
    Numeric,
    /// `STATUS_TEXT_PERCENT` "Percentage".
    Percent,
    /// `STATUS_TEXT_BOTH` "Both".
    Both,
    /// `NONE`: text only while the bar is moused over.
    #[default]
    None,
}

impl StatusTextDisplay {
    pub const ALL: [Self; 4] = [Self::Numeric, Self::Percent, Self::Both, Self::None];

    /// The Retail dropdown value (Interface.lua:65-74).
    pub fn value(self) -> u8 {
        match self {
            Self::Numeric => 1,
            Self::Percent => 2,
            Self::Both => 3,
            Self::None => 4,
        }
    }

    pub fn from_value(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.value() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Numeric => "Numeric Value",
            Self::Percent => "Percentage",
            Self::Both => "Both",
            Self::None => "None",
        }
    }

    /// `SetValue` writes CVar `statusText` "1" for every choice but None
    /// (Interface.lua:77-90); unit frame bars keep their text shown while it is "1"
    /// (`cvar = "statusText"`, `textLockable`, TextStatusBar.lua:115).
    fn locks_text_shown(self) -> bool {
        self != Self::None
    }
}

/// The three font strings of a unit frame bar (`TextString`, `LeftText`, `RightText`);
/// an empty string is a hidden one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusBarText {
    pub center: String,
    pub left: String,
    pub right: String,
}

/// Per-bar `TextStatusBar` settings the unit frames set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStatusBar {
    /// `zeroText`: TargetFrame bars show "" at 0 (TargetFrame.lua:768).
    pub zero_text: Option<&'static str>,
    /// Both mode puts the percentage in `LeftText` only for health bars and Mana
    /// (`not self.powerToken or self.powerToken == "MANA"`, TextStatusBar.lua:180).
    pub percent_in_both: bool,
}

impl TextStatusBar {
    pub const HEALTH: Self = Self {
        zero_text: None,
        percent_in_both: true,
    };

    pub fn power(is_mana: bool) -> Self {
        Self {
            zero_text: None,
            percent_in_both: is_mana,
        }
    }

    /// TargetFrame / boss frame bars (`TargetFrameStatusBarMixin:OnLoad`).
    pub fn with_zero_text(self, zero_text: &'static str) -> Self {
        Self {
            zero_text: Some(zero_text),
            ..self
        }
    }

    /// `UpdateTextStringWithValues` for every unit frame bar: `LeftText` and `RightText`
    /// exist, `capNumericDisplay` is set (UnitFrame.lua:58-64) and no prefix is used.
    /// `hovered` is the `lockShow` the bar's `OnEnter` adds (TextStatusBar.xml:7,
    /// TextStatusBar.lua:217-220).
    pub fn text(
        self,
        value: i64,
        max: i64,
        display: StatusTextDisplay,
        hovered: bool,
    ) -> StatusBarText {
        let mut text = StatusBarText::default();
        if max <= 0 || !(display.locks_text_shown() || hovered) {
            return text;
        }
        if let (0, Some(zero_text)) = (value, self.zero_text) {
            text.center = zero_text.to_owned();
            return text;
        }
        let value_display = abbreviate_large_numbers(value);
        match display {
            StatusTextDisplay::Numeric | StatusTextDisplay::None => {
                text.center = format!("{value_display} / {}", abbreviate_large_numbers(max));
            }
            StatusTextDisplay::Both => {
                if self.percent_in_both {
                    text.left = format!("{}%", percent(value, max));
                }
                text.right = value_display;
            }
            StatusTextDisplay::Percent => text.center = format!("{}%", percent(value, max)),
        }
        text
    }
}

/// `math.ceil((value / valueMax) * 100)` in the same double arithmetic, so 7 / 100 is 8.
pub fn percent(value: i64, max: i64) -> i64 {
    ((value as f64 / max as f64) * 100.0).ceil() as i64
}

/// `AbbreviateLargeNumbers` for enUS. Retail implements it natively (no published body,
/// LocalizationDocumentation.lua:10); this is its last published Lua, shared by the
/// Classic clients (wow-ui-source classic Blizzard_UIParent/Shared/UIParent.lua:774-785):
/// over 8 digits drop 6 and add `SECOND_NUMBER_CAP` " M", over 5 drop 3 and add
/// `FIRST_NUMBER_CAP` " K", over 3 group with `BreakUpLargeNumbers`.
pub fn abbreviate_large_numbers(value: i64) -> String {
    let digits = value.to_string();
    let len = digits.len();
    if len > 8 {
        format!("{} M", &digits[..len - 6])
    } else if len > 5 {
        format!("{} K", &digits[..len - 3])
    } else if len > 3 {
        break_up_large_numbers(value)
    } else {
        digits
    }
}

/// `BreakUpLargeNumbers` for enUS: thousands grouped with ",".
pub fn break_up_large_numbers(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        grouped.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
