//! UIButtonMixin static content and SetOwner policy through the existing tooltip model.
use super::{GameTooltip, OwnerSide, TooltipAnchor};
use crate::item_tooltip::RED_FONT_COLOR;
use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipLineState, TooltipPresentation,
    description_lines,
};
use ui_toolkit::tooltip::{TooltipAnchor as AuthoredAnchor, TooltipBinding};

impl GameTooltip {
    /// Dynamic provider keys are resolved from live host state, never cached as static content.
    pub fn from_binding(binding: &TooltipBinding, enabled: bool, rect: [f32; 4]) -> Option<Self> {
        if binding.provider.is_some() {
            return None;
        }
        let (content, anchor, offset) = if enabled {
            (enabled_content(binding)?, binding.anchor, binding.offset)
        } else {
            (
                disabled_content(binding)?,
                binding.disabled_anchor,
                binding.disabled_offset,
            )
        };
        let mut tooltip = Self::new(content, None);
        tooltip.anchor = binding_anchor(anchor, rect, offset);
        Some(tooltip)
    }
}

fn enabled_content(binding: &TooltipBinding) -> Option<TooltipPresentation> {
    if binding.title.is_none() && binding.text.is_none() {
        return None;
    }
    let text_color = binding.text_color.unwrap_or(TOOLTIP_DESCRIPTION_COLOR);
    let mut lines = body_lines(binding, text_color).into_iter();
    // AddLine uses textLeft1Font even when UIButtonMixin did not call SetTitle.
    let (title, title_color) = match &binding.title {
        Some(title) => (title.clone(), binding.title_color.unwrap_or(TOOLTIP_WHITE)),
        None => (
            lines.next().map(|line| line.left_text).unwrap_or_default(),
            text_color,
        ),
    };
    Some(TooltipPresentation {
        title,
        title_color,
        lines: lines.collect(),
        ..Default::default()
    })
}

fn disabled_content(binding: &TooltipBinding) -> Option<TooltipPresentation> {
    Some(TooltipPresentation {
        title: binding.disabled_text.clone()?,
        title_color: RED_FONT_COLOR,
        ..Default::default()
    })
}

fn body_lines(binding: &TooltipBinding, color: [f32; 4]) -> Vec<TooltipLineState> {
    let Some(text) = binding.text.as_deref() else {
        return Vec::new();
    };
    if binding.wrap {
        description_lines(text, color)
    } else {
        text.lines()
            .map(|line| TooltipLineState::colored(line, color))
            .collect()
    }
}

/// Convert Retail x-right/y-up owner offsets once; runtime anchors use UI y-down.
pub fn binding_anchor(anchor: AuthoredAnchor, rect: [f32; 4], [x, y]: [f32; 2]) -> TooltipAnchor {
    match anchor {
        AuthoredAnchor::Default => TooltipAnchor::Default,
        AuthoredAnchor::Cursor => TooltipAnchor::Cursor,
        AuthoredAnchor::CursorRight => TooltipAnchor::CursorRight,
        owner => TooltipAnchor::OwnerOffset {
            rect,
            side: owner_side(owner),
            offset: [x, -y],
        },
    }
}

fn owner_side(anchor: AuthoredAnchor) -> OwnerSide {
    match anchor {
        AuthoredAnchor::Right => OwnerSide::Right,
        AuthoredAnchor::Left => OwnerSide::Left,
        AuthoredAnchor::Top => OwnerSide::Top,
        AuthoredAnchor::Bottom => OwnerSide::Bottom,
        AuthoredAnchor::TopLeft => OwnerSide::TopLeft,
        AuthoredAnchor::TopRight => OwnerSide::TopRight,
        AuthoredAnchor::BottomLeft => OwnerSide::BottomLeft,
        AuthoredAnchor::BottomRight => OwnerSide::BottomRight,
        AuthoredAnchor::BagSlot => OwnerSide::BagSlot,
        AuthoredAnchor::ByCenter => OwnerSide::ByCenter,
        _ => unreachable!("non-owner tooltip anchor handled before owner-side conversion"),
    }
}
