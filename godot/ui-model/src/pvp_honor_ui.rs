//! Retail Mainline PVPUI.xml804–861/1670–1728; Lua1904–1947/1975–1983.
//! No ratio, badge level or next reward is invented while HONOR_LEVEL_XP_CURVE is open.
use super::{PvpView, frame};
use crate::bank_art::label;
use crate::game_tooltip::GameTooltip;
use crate::quest_art::{NORMAL_FONT_COLOR, named_atlas_texture};
use crate::tooltip_presentation::{TooltipLineState, TooltipPresentation};
use ui_toolkit::widget_def::Element;

pub(super) fn honor_inset(view: &PvpView) -> Element {
    let Some(snapshot) = &view.snapshot else {
        return vec![];
    };
    let title = snapshot.honor_level.map_or_else(
        || "Honor Level —".to_string(),
        |level| format!("Honor Level {level}"),
    );
    let mut ring = named_atlas_texture(
        "PVPUIHonorLevelBorder".into(),
        "pvpqueue-sidebar-honorbar-background",
        (-34.0, -20.5, 196.0, 171.0),
    );
    ring.extend(label(
        "PVPUIHonorLevelLabel".into(),
        &title,
        (-36.0, -17.0, 200.0, 16.0),
        (12.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    let mut children = named_atlas_texture(
        "PVPUIHonorInsetBackground".into(),
        "pvpqueue-sidebar-background",
        (3.0, 3.0, 160.0, 375.0),
    );
    children.extend(frame(
        "PVPUIHonorLevelDisplay",
        (128.0, 128.0),
        (19.0, 25.0),
        ring,
    ));
    // PVPUIFrame563 + Lua225 extra width; XML inset RIGHT-6, width166, top22/bottom25.
    frame("PVPUIHonorInset", (166.0, 381.0), (616.0, 22.0), children)
}

impl PvpView {
    pub fn honor_tooltip(&self) -> Option<GameTooltip> {
        let snapshot = self.snapshot.as_ref()?;
        Some(GameTooltip::new(
            TooltipPresentation {
                title: "Lifetime Honor".into(),
                // Unknown max must not be replaced by the spendable currency cap15000.
                lines: vec![TooltipLineState::new(
                    snapshot.lifetime_honor_xp.to_string(),
                )],
                ..TooltipPresentation::hidden()
            },
            None,
        ))
    }
}
