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

pub(super) fn queue_choices(view: &PvpView) -> Element {
    [
        (
            "HonorFrameArena2v2Button",
            "Arena Skirmish 2v2",
            "pvp:activity:2v2",
            136.0,
        ),
        (
            "HonorFrameArena3v3Button",
            "Arena Skirmish 3v3",
            "pvp:activity:3v3",
            182.0,
        ),
    ]
    .into_iter()
    .flat_map(|(name, title, action, top)| {
        panel_button(
            name.into(),
            title,
            action,
            !view.pending,
            (194.0, top, 340.0, 40.0),
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
        class_rgb: [1.0, 0.0, 0.0],
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
    frame.extend(rsx! { button { name:{DynName(format!("{name}Target"))},width:WIDTH,height:HEIGHT,
        left:0.0,top,pos_type:"absolute",strata:FrameStrata::Medium,onclick:{target.as_str()},text:"" } });
    frame
}
