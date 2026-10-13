//! Mainline CompactArenaFrame: default EditMode dimensions72+26 by36+8,
//! RIGHT/RIGHT at0,0; Forever shares elements and only changes atlas art.
use super::{DynName, PvpView, frame, panel_button, text};
use crate::ui::screens::compact_unit_frame_component::{
    CompactUnitView, UnitStatus, compact_unit_frame,
};
use shared::protocol::{ArenaUnitSnapshot, BattlegroundSnapshot, GroupRoleSnapshot};
use ui_toolkit::{rsx, strata::FrameStrata, widget_def::Element};
const WIDTH: f32 = 98.0;
const HEIGHT: f32 = 44.0;

pub(super) fn rated_panel(view: &PvpView) -> Element {
    let mut elements = Vec::new();
    if let Some(snapshot) = &view.snapshot {
        elements.extend(text(
            "ConquestFrameBalance",
            &format!("Conquest: {}", snapshot.conquest),
            (236.0, 41.0, 370.0, 22.0),
        ));
    }
    use shared::protocol::PvpBracketSnapshot as Bracket;
    // PVPUI.xml: Conquest(220,22), Inset(4,85), 393px wide; first TOP(-8), subsequent BOTTOM+0.
    for (index, (name, title, detail, action, bracket, available)) in [
        (
            "ConquestFrameRatedSoloShuffle",
            "Solo Shuffle",
            "Unrated",
            "pvp:rated:shuffle",
            Some(Bracket::SoloShuffle),
            true,
        ),
        (
            "ConquestFrameRatedBGBlitz",
            "Battleground Blitz",
            "Unavailable",
            "",
            None,
            false,
        ),
        (
            "ConquestFrameArena2v2",
            "2v2",
            "Rated Arena",
            "pvp:rated:2v2",
            Some(Bracket::Arena2v2),
            true,
        ),
        (
            "ConquestFrameArena3v3",
            "3v3",
            "Rated Arena",
            "pvp:rated:3v3",
            Some(Bracket::Arena3v3),
            true,
        ),
        (
            "ConquestFrameRatedBG",
            "Battlegrounds",
            "Unavailable",
            "",
            Some(Bracket::RatedBattleground),
            false,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let rating = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| {
                snapshot
                    .brackets
                    .iter()
                    .find(|stats| Some(&stats.bracket) == bracket.as_ref())
            })
            .and_then(|stats| stats.rating);
        elements.extend(activity_button(ActivityButton {
            name,
            title,
            detail,
            action,
            enabled: available && !view.pending,
            selected: bracket.as_ref() == Some(&view.rated_bracket),
            rect: (238.0, 115.0 + index as f32 * 56.0, 56.0),
            rated: true,
            rating,
        }));
    }
    elements.extend(crate::bank_art::label(
        "PVPUIError".into(),
        &view.error,
        (
            (view.viewport[0] - 512.0) / 2.0 - 100.0,
            122.0 - 84.0,
            512.0,
            60.0,
        ),
        (16.0, "1,0.1,0.1,1", "CENTER"),
    ));
    let queued = view
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.queue.is_some());
    elements.extend(panel_button(
        "ConquestFrameJoinButton".into(),
        if queued { "Leave Queue" } else { "Join Battle" },
        if queued {
            "pvp:dequeue"
        } else {
            "pvp:rated:queue"
        },
        !view.pending && view.battleground().is_none(),
        (363.0, 404.0, 116.0, 22.0),
    ));
    elements
}

pub(super) fn queue_choices(view: &PvpView, top: f32) -> Element {
    [
        (
            "HonorFrameArena2v2Button",
            "Arena Skirmish 2v2",
            "pvp:activity:2v2",
            top,
            shared::protocol::ArenaQueueMode::Skirmish2v2,
        ),
        (
            "HonorFrameArena3v3Button",
            "Arena Skirmish 3v3",
            "pvp:activity:3v3",
            top + 55.0,
            shared::protocol::ArenaQueueMode::Skirmish3v3,
        ),
    ]
    .into_iter()
    .flat_map(|(name, title, action, top, mode)| {
        activity_button(ActivityButton {
            name,
            title,
            detail: "",
            action,
            enabled: !view.pending,
            selected: view.selected_arena == Some(mode),
            rect: (0.0, top, 54.0),
            rated: false,
            rating: None,
        })
    })
    .collect()
}
struct ActivityButton<'a> {
    name: &'a str,
    title: &'a str,
    detail: &'a str,
    action: &'a str,
    enabled: bool,
    selected: bool,
    rect: (f32, f32, f32),
    rated: bool,
    rating: Option<u32>,
}

fn activity_button(props: ActivityButton<'_>) -> Element {
    // PVPUI.xml:730 rated365x56; :647 casual365-wide, Lua1131-1138 height(296-26)/5=54.
    let (left, top, height) = props.rect;
    let prefix = if props.rated {
        "pvpqueue-button"
    } else {
        "pvpqueue-button-casual"
    };
    let up = format!("{prefix}-up");
    let down = format!("{prefix}-down");
    let name = props.name;
    let title = props.title;
    let detail = props.detail;

    let title_height = if props.rated { 20.0 } else { 16.0 };
    let title_top = if props.rated {
        11.0
    } else {
        height / 2.0 + 1.0 - title_height / 2.0
    };
    let mut children = crate::bank_art::label(
        format!("{name}Text"),
        title,
        (20.0, title_top, 325.0, title_height),
        (
            title_height,
            if props.enabled {
                "1,1,1,1"
            } else {
                "0.5,0.5,0.5,1"
            },
            "LEFT",
        ),
    );
    if props.rated {
        children.extend(crate::bank_art::label(
            format!("{name}TypeText"),
            detail,
            (19.0, 34.0, 326.0, 12.0),
            (12.0, "0.5,0.5,0.5,1", "LEFT"),
        ));
        if let Some(rating) = props.rating {
            children.extend(crate::bank_art::label(
                format!("{name}CurrentRating"),
                &rating.to_string(),
                (202.5, 21.0, 100.0, 14.0),
                (14.0, "1,0.82,0,1", "LEFT"),
            ));
        }
    }
    let template = if props.rated {
        &crate::selection_list::PVP_RATED_ACTIVITY
    } else {
        &crate::selection_list::PVP_CASUAL_ACTIVITY
    };
    let mut row = crate::selection_list::row(
        name.into(),
        props.action,
        props.enabled,
        (left, top, 365.0, height),
        (template, props.selected),
        children,
    );
    let ui_toolkit::widget_def::WidgetChild::Widget(button) = &mut row[0] else {
        unreachable!("source activity row is a button")
    };
    for (key, art) in [
        ("button_atlas_up", up.as_str()),
        ("button_atlas_pressed", down.as_str()),
        ("button_atlas_disabled", up.as_str()),
    ] {
        button
            .attrs
            .push(ui_toolkit::widget_def::Attr::new_dynamic(key, art.into()));
    }
    row
}

pub(super) fn arena_hud(view: &PvpView, bg: &BattlegroundSnapshot) -> Element {
    let arena = bg.arena.as_ref().expect("arena HUD");
    let status = if let Some(remaining) = arena
        .preparation_remaining_secs
        .filter(|seconds| *seconds > 0)
    {
        format!("Preparation: {remaining} seconds")
    } else {
        bg.name.clone()
    };
    let status = arena.shuffle.as_ref().map_or(status.clone(), |shuffle| {
        format!(
            "Round {}/{} • {status}",
            shuffle.round, shuffle.total_rounds
        )
    });
    let mut hud = text("ArenaPreparationTimer", &status, (0.0, 0.0, 430.0, 24.0));
    hud.extend(panel_button(
        "WorldStateScoreboardButton".into(),
        "Scoreboard",
        "pvp:scoreboard",
        true,
        (150.0, 26.0, 130.0, 22.0),
    ));
    let enemies: Vec<_> = arena
        .units
        .iter()
        .filter(|unit| unit.team != bg.team)
        .collect();
    let height = enemies.len() as f32 * HEIGHT;
    let mut units = Vec::new();
    for (index, unit) in enemies.iter().enumerate() {
        units.extend(enemy_frame(index, unit));
    }
    let left = view.viewport[0] - WIDTH;
    let top = (view.viewport[1] - height) / 2.0;
    let mut elements = frame(
        "WorldStateAlwaysUpFrame",
        (430.0, 54.0),
        ((view.viewport[0] - 430.0) / 2.0, 18.0),
        hud,
    );
    elements.extend(
        rsx! { r#frame { name:"CompactArenaFrame",width:WIDTH,height,left,top,
        strata:FrameStrata::Low,pos_type:"absolute", {units} } },
    );
    elements
}
fn enemy_frame(index: usize, unit: &ArenaUnitSnapshot) -> Element {
    let name = format!("CompactArenaFrameMember{}", index + 1);
    let health_fraction =
        (unit.max_health > 0).then(|| unit.health as f32 / unit.max_health as f32);
    let status = if unit.health == 0 {
        UnitStatus::Dead
    } else {
        UnitStatus::Online
    };
    let view = CompactUnitView {
        name: unit.name.clone(),
        class_rgb: crate::damage_meter_data::class_color(unit.class),
        health_fraction,
        power: None,
        role: GroupRoleSnapshot::None,
        status,
        in_range: true,
        selected: false,
        ready: None,
        debuffs: Vec::new(),
    };
    let top = index as f32 * HEIGHT;
    let mut frame = compact_unit_frame(&name, &view, (0.0, top, WIDTH, HEIGHT), None);
    let target = format!("pvp:target:{}", unit.entity);
    frame.extend(
        rsx! { button { name:{DynName(format!("{name}Target"))},width:WIDTH,height:HEIGHT,
        left:0.0,top,pos_type:"absolute",strata:FrameStrata::Medium,button_default_skin:false,
        onclick:{target.as_str()},text:"" } },
    );
    frame
}
