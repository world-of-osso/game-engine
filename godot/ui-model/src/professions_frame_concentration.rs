//! Mainline ProfessionsCurrencyTemplate (Templates.xml322–344, Crafting.xml124–128).
use super::ProfessionView;
use crate::bank_art::{HIGHLIGHT_FONT_COLOR, label, texture};
use crate::game_tooltip::GameTooltip;
use crate::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, TooltipPresentation, description_lines,
};
use shared::concentration::ConcentrationSnapshot;
use ui_toolkit::{rsx, widget_def::Element};

pub fn amount_text(snapshot: &ConcentrationSnapshot) -> String {
    let amount = snapshot
        .quantity
        .map_or_else(|| "—".into(), |quantity| quantity.to_string());
    // PROFESSIONS_CRAFTING_CURRENCY_LABEL_FORMAT: highlighted %d/%d, no grouping.
    format!("{amount}/{}", snapshot.currency.cap)
}

pub(super) fn display(view: &ProfessionView) -> Element {
    let Some(snapshot) = &view.concentration else {
        return vec![];
    };
    let mut children = texture(
        "ProfessionsConcentrationIcon".into(),
        snapshot.currency.icon,
        (0.0, 4.0, 24.0, 24.0),
        "1,1,1,1",
    );
    children.extend(label(
        "ProfessionsConcentrationAmount".into(),
        &amount_text(snapshot),
        (30.0, 6.0, 80.0, 16.0),
        (12.0, HIGHLIGHT_FONT_COLOR, "LEFT"),
    ));
    rsx! { r#frame { name: "ProfessionsConcentrationDisplay", width: 110.0, height: 32.0,
        tooltip_provider: "profession_concentration",
        left: 120.0, top: 35.0, pos_type: "absolute", mouse_enabled: true, {children}
    } }
}

pub fn tooltip(snapshot: &ConcentrationSnapshot) -> GameTooltip {
    let mut lines = Vec::new();
    if snapshot.quantity == Some(snapshot.currency.cap) {
        lines.push(TooltipLineState::colored(
            crate::auction_mail::text(
                "PROFESSIONS_CRAFTING_CONCENTRATION_CURRENCY_TOOLTIP_WARNING",
            ),
            [1.0, 0.1, 0.1, 1.0],
        ));
    }
    lines.extend(description_lines(
        crate::auction_mail::text("PROFESSIONS_CRAFTING_CONCENTRATION_CURRENCY_TOOLTIP"),
        TOOLTIP_DESCRIPTION_COLOR,
    ));
    if let Err(gap) = snapshot.recharge_origin {
        lines.extend(description_lines(&gap.to_string(), [1.0, 0.1, 0.1, 1.0]));
    }
    GameTooltip::new(
        TooltipPresentation {
            title: snapshot.currency.name.clone(),
            title_color: [1.0; 4],
            title_suffix: Some((format!("  {}", amount_text(snapshot)), [1.0; 4])),
            lines,
            ..Default::default()
        },
        None,
    )
}
