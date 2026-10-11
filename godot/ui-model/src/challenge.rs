//! Mainline ChallengesKeystoneFrame (398x548) and challenge/scenario objective data.
use crate::objective_tracker_component::{DungeonBlock, ObjectiveLine, ObjectiveLineStyle};
use shared::protocol::{ChallengePhase, ChallengeRequest, ChallengeRun, ChallengeSnapshot};

pub fn clock(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{}:{:02}", secs / 60, secs % 60)
}
pub fn tracker_block(run: &ChallengeRun) -> Option<DungeonBlock> {
    if run.phase == ChallengePhase::Slotted {
        return None;
    }
    let mut lines = Vec::new();
    let time = match run.phase {
        ChallengePhase::Countdown => format!("Starting in {}", clock(run.countdown_ms)),
        ChallengePhase::Completed => format!("Completed: {}", clock(run.elapsed_ms)),
        ChallengePhase::Abandoned => "Run abandoned".into(),
        _ => format!(
            "Time Remaining: {}",
            clock(run.time_limit_ms.saturating_sub(run.elapsed_ms))
        ),
    };
    let mut add = |text: String, complete: bool| {
        lines.push(ObjectiveLine {
            text,
            style: if complete {
                ObjectiveLineStyle::Completed
            } else {
                ObjectiveLineStyle::InProgress
            },
        })
    };
    add(time, false);
    let percent = if run.forces_required == 0 {
        0.0
    } else {
        100.0 * run.forces as f64 / run.forces_required as f64
    };
    add(
        format!(
            "Enemy Forces: {} / {} ({percent:.1}%)",
            run.forces, run.forces_required
        ),
        run.forces >= run.forces_required,
    );
    add(
        format!(
            "Deaths: {} (Time Lost: {})",
            run.deaths,
            clock(run.time_lost_ms)
        ),
        false,
    );
    for boss in &run.bosses {
        add(boss.name.clone(), boss.defeated);
    }
    if let Some(level) = run.resulting_level {
        add(
            format!("Keystone +{level} ({} upgrades)", run.upgrade_levels),
            false,
        );
    }
    if let Some(rating) = run.rating {
        add(format!("Rating: {rating}"), false);
    }
    if !run.key.affixes.is_empty() {
        add(format!("Affixes: {:?}", run.key.affixes), false);
    }
    Some(DungeonBlock {
        challenge: true,
        name: format!("{} +{}", run.name, run.key.level),
        bosses: lines,
    })
}
pub fn slot_cursor(snapshot: &ChallengeSnapshot, guid: u64) -> Option<ChallengeRequest> {
    let font = snapshot.font?;
    snapshot
        .keys
        .iter()
        .any(|key| key.item_guid == guid)
        .then_some(ChallengeRequest::Slot {
            font,
            item_guid: guid,
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mythic_challenge_tracker_displays_authored_time_forces_and_summary_without_rating() {
        let run = ChallengeRun {
            run_id: 1,
            map_id: 2515,
            instance_id: 7,
            name: "The Azure Vault".into(),
            key: shared::protocol::KeystoneInfo {
                item_guid: 42,
                item_id: 180653,
                challenge_id: 401,
                level: 8,
                affixes: vec![],
            },
            phase: ChallengePhase::Running,
            countdown_ms: 0,
            elapsed_ms: 65000,
            time_limit_ms: 2250000,
            deaths: 2,
            time_lost_ms: 0,
            forces: 205,
            forces_required: 410,
            bosses: vec![],
            upgrade_levels: 0,
            resulting_level: None,
            rating: None,
        };
        let block = tracker_block(&run).expect("running tracker");
        assert!(block.bosses.iter().any(|line| line.text.contains("36:25")));
        assert!(
            block
                .bosses
                .iter()
                .any(|line| line.text.contains("205 / 410") && line.text.contains("50.0%"))
        );
        assert!(
            block
                .bosses
                .iter()
                .any(|line| line.text.contains("Deaths: 2"))
        );
        assert!(
            !block
                .bosses
                .iter()
                .any(|line| line.text.contains("Rating: 0"))
        );
        let snapshot = ChallengeSnapshot {
            font: Some(9),
            keys: vec![run.key.clone()],
            run: Some(run.clone()),
            error: None,
        };
        assert_eq!(
            slot_cursor(&snapshot, 42),
            Some(ChallengeRequest::Slot {
                font: 9,
                item_guid: 42
            })
        );
        assert_eq!(slot_cursor(&snapshot, 43), None);
        let mut done = run;
        done.phase = ChallengePhase::Completed;
        done.resulting_level = Some(11);
        done.upgrade_levels = 3;
        assert!(
            tracker_block(&done)
                .unwrap()
                .bosses
                .iter()
                .any(|line| line.text.contains("+11"))
        );
    }
}

use crate::bank_art::label;
use crate::quest_art::{named_atlas_texture, panel_button};
use ui_toolkit::strata::FrameStrata;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeView {
    pub viewport: [f32; 2],
    pub snapshot: ChallengeSnapshot,
}
pub fn challenge_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let view = ctx.get::<ChallengeView>().expect("ChallengeView");
    if let Some(run) = &view.snapshot.run {
        if run.phase == ChallengePhase::Completed {
            return completion_banner(view, run);
        }
    }
    keystone_frame(view)
}
fn keystone_frame(view: &ChallengeView) -> Element {
    let mut children = named_atlas_texture(
        "ChallengesKeystoneBackground".into(),
        "ChallengeMode-KeystoneFrame",
        (0.0, 0.0, 398.0, 548.0),
    );
    children.extend(named_atlas_texture(
        "ChallengesRuneBG".into(),
        "ChallengeMode-RuneBG",
        (19.0, 34.0, 360.0, 360.0),
    ));
    let slotted = view
        .snapshot
        .run
        .as_ref()
        .filter(|run| run.phase == ChallengePhase::Slotted);
    let title = slotted
        .map(|run| format!("Keystone Level {}", run.key.level))
        .unwrap_or_default();
    children.extend(label(
        "ChallengesPowerLevel".into(),
        &title,
        (24.0, 30.0, 350.0, 30.0),
        (20.0, "1,0.82,0,1", "CENTER"),
    ));
    let (name, time) = slotted
        .map(|run| (run.name.clone(), clock(run.time_limit_ms)))
        .unwrap_or_default();
    children.extend(label(
        "ChallengesDungeonName".into(),
        &name,
        (24.0, 358.0, 350.0, 40.0),
        (20.0, "1,0.82,0,1", "CENTER"),
    ));
    children.extend(label(
        "ChallengesTimeLimit".into(),
        &time,
        (24.0, 405.0, 350.0, 30.0),
        (16.0, "1,1,1,1", "CENTER"),
    ));
    let instruction = if slotted.is_none() {
        "Insert your Mythic Keystone"
    } else {
        ""
    };
    children.extend(label(
        "ChallengesInstructions".into(),
        instruction,
        (12.0, 438.0, 374.0, 30.0),
        (16.0, "1,1,1,1", "CENTER"),
    ));
    let slot_icon = if slotted.is_some() {
        crate::bank_art::texture(
            "ChallengesKeystoneIcon".into(),
            4352494,
            (0.0, 0.0, 48.0, 48.0),
            "1,1,1,1",
        )
    } else {
        Vec::new()
    };
    // Like the existing item_slot host: no generic UIPanelButton art over the item.
    children.extend(rsx! {r#frame {name:"ChallengesKeystoneSlot",width:48.0,height:48.0,pos_type:"absolute",left:175.0,top:190.0,mouse_enabled:true,onclick:"challenge_slot",{slot_icon}}});
    children.extend(panel_button(
        "ChallengesStartButton".into(),
        "Activate",
        "challenge_start",
        slotted.is_some(),
        (139.0, 504.0, 120.0, 24.0),
    ));
    children.extend(panel_button(
        "ChallengesCloseButton".into(),
        "X",
        "challenge_close",
        true,
        (365.0, 7.0, 24.0, 24.0),
    ));
    if let Some(error) = &view.snapshot.error {
        children.extend(label(
            "ChallengesError".into(),
            error,
            (20.0, 474.0, 358.0, 25.0),
            (12.0, "1,0.2,0.2,1", "CENTER"),
        ));
    }
    let x = (view.viewport[0] - 398.0) * 0.5;
    let y = (view.viewport[1] - 548.0) * 0.5 - 40.0;
    rsx! {r#frame {name:"ChallengesKeystoneFrame",width:398.0,height:548.0,strata:FrameStrata::Dialog,mouse_enabled:true,pos_type:"absolute",left:x,top:y,{children}}}
}
fn completion_banner(view: &ChallengeView, run: &ChallengeRun) -> Element {
    let mut children = named_atlas_texture(
        "ChallengeBannerTop".into(),
        "BossBanner-BgBanner-Top",
        (0.0, 44.0, 128.0, 160.0),
    );
    let title = format!("{} +{}", run.name, run.key.level);
    children.extend(label(
        "ChallengeCompleteTitle".into(),
        &title,
        (-186.0, 60.0, 500.0, 35.0),
        (22.0, "1,0.82,0,1", "CENTER"),
    ));
    let timed = run.upgrade_levels > 0;
    let result = if timed {
        "Challenge completed within time!"
    } else {
        "Challenge completed over time"
    };
    children.extend(label(
        "ChallengeCompleteDescriptionLineOne".into(),
        result,
        (-186.0, 230.0, 500.0, 30.0),
        (18.0, "1,1,1,1", "CENTER"),
    ));
    let summary = format!(
        "{} — Keystone +{} ({} upgrades)",
        clock(run.elapsed_ms),
        run.resulting_level.unwrap_or(run.key.level),
        run.upgrade_levels
    );
    children.extend(label(
        "ChallengeCompleteDescriptionLineTwo".into(),
        &summary,
        (-186.0, 265.0, 500.0, 30.0),
        (16.0, "1,0.82,0,1", "CENTER"),
    ));
    let x = (view.viewport[0] - 128.0) * 0.5;
    rsx! {r#frame {name:"ChallengeModeCompleteBanner",width:128.0,height:356.0,pos_type:"absolute",left:x,top:120.0,strata:FrameStrata::Dialog,{children}}}
}
