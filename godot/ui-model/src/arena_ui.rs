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
    let mut elements = text("ConquestFrameTitle", "Rated", (194.0, 52.0, 340.0, 22.0));
    if let Some(snapshot) = &view.snapshot {
        elements.extend(text(
            "ConquestFrameBalance",
            &format!("Conquest: {}", snapshot.conquest),
            (194.0, 90.0, 340.0, 22.0),
        ));
    }
    let shuffle = view.rated_bracket == shared::protocol::PvpBracketSnapshot::SoloShuffle;
    let selected = if view.rated_bracket == shared::protocol::PvpBracketSnapshot::Arena2v2 {
        "2v2"
    } else {
        "3v3"
    };
    let policy = if shuffle {
        "Solo Shuffle • Unrated • Levels60–90\nSolo healers/damage; two healers/four damage"
    } else {
        &format!(
            "{selected} • Full max-level group; leader queues\nRating uses the pinned TC reference"
        )
    };
    elements.extend(text(
        "ConquestFramePolicy",
        policy,
        (194.0, 122.0, 340.0, 36.0),
    ));
    for (name, title, action, top) in [
        ("ConquestFrameArena2v2", "2v2", "pvp:rated:2v2", 164.0),
        ("ConquestFrameArena3v3", "3v3", "pvp:rated:3v3", 210.0),
    ] {
        elements.extend(panel_button(
            name.into(),
            title,
            action,
            !view.pending,
            (194.0, top, 340.0, 40.0),
        ));
    }
    elements.extend(panel_button(
        "ConquestFrameRatedSoloShuffle".into(),
        "Solo Shuffle — Unrated",
        "pvp:rated:shuffle",
        !view.pending,
        (194.0, 256.0, 340.0, 40.0),
    ));
    elements.extend(panel_button(
        "ConquestFrameRatedBG".into(),
        "Rated Battlegrounds — unavailable",
        "",
        false,
        (194.0, 302.0, 340.0, 40.0),
    ));
    elements.extend(text("PVPUIError", &view.error, (194.0, 348.0, 340.0, 32.0)));
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
        (365.0, 388.0, 170.0, 22.0),
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
        ),
        (
            "HonorFrameArena3v3Button",
            "Arena Skirmish 3v3",
            "pvp:activity:3v3",
            top + 40.0,
        ),
    ]
    .into_iter()
    .flat_map(|(name, title, action, top)| {
        panel_button(
            name.into(),
            title,
            action,
            !view.pending,
            (0.0, top, 324.0, 40.0),
        )
    })
    .collect()
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
