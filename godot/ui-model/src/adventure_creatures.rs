//! EncounterCreatureButtonTemplate XML525–581; Lua1044–1054/1455–1517.
use ui_toolkit::{rsx, widget_def::Element};

use crate::{
    adventure_guide::View,
    adventure_guide_art::{Art, art, highlight},
    bank_art::{WHITE, texture},
    game_tooltip::GameTooltip,
    quest_art::DynName,
    tooltip_presentation::{TooltipLineState, TooltipPresentation},
};

pub(crate) fn buttons(view: &View) -> Element {
    view.model_creatures()
        .into_iter()
        .scan(100., |top, creature| {
            let selected = view.creature == Some(creature.id);
            let height = if selected { 61. } else { 49. };
            let result = creature_button(creature, selected, *top);
            *top += height - 8.;
            Some(result)
        })
        .flatten()
        .collect()
}

fn creature_button(
    creature: &game_engine_core::adventure_guide::Creature,
    selected: bool,
    top: f32,
) -> Element {
    let name = format!("JournalCreature{}", creature.id);
    let (width, height, icon) = if selected {
        (64., 61., 40.)
    } else {
        (50., 49., 36.)
    };
    let mut content = texture(
        format!("{name}Portrait"),
        creature.portrait,
        ((width - icon) / 2., (height - icon) / 2., icon, icon),
        WHITE,
    );
    content.extend(art(
        format!("{name}ButtonArt"),
        Art::InstanceRing,
        (0., 0., width, height),
    ));
    content.extend(highlight(
        format!("{name}ButtonHighlight"),
        Art::InstanceRing,
        (0., 0., width, height),
    ));
    let action = format!("journal:creature:{}", creature.id);
    rsx! { button { name: {DynName(name)}, left: 402., top, width, height, pos_type: "absolute", frame_level: 2., disabled: selected, onclick: {action.as_str()}, button_default_skin: false, {content} } }
}

pub fn tooltip(view: &View, frame: &str) -> Option<GameTooltip> {
    let id: u32 = frame.strip_prefix("JournalCreature")?.parse().ok()?;
    let creature = view
        .model_creatures()
        .into_iter()
        .find(|creature| creature.id == id)?;
    let lines = if creature.description.is_empty() {
        Vec::new()
    } else {
        vec![TooltipLineState::new(&creature.description)]
    };
    Some(GameTooltip::new(
        TooltipPresentation {
            title: creature.name.clone(),
            title_color: [1.; 4],
            lines,
            ..Default::default()
        },
        None,
    ))
}
