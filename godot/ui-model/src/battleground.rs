//! Retail Mainline PVPUIFrame, PVPReadyDialog, PVPMatchScoreboard and Results.
//! Layout authority: Blizzard_PVPUI/Mainline and Blizzard_PVPMatch XML/Lua.
use crate::bank_art::{WHITE, label};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome_no_portrait};
use shared::battleground::{BgOutcome, BgPhase, BgTeam, FlagState};
use shared::protocol::{BattlegroundSnapshot, PvpSnapshot, PvpStateUpdate};
use ui_toolkit::{rsx, screen::SharedContext, strata::FrameStrata, widget_def::Element};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PvpAction {
    Query,
    WarMode(bool),
    Queue(u32),
    Dequeue,
    Respond { match_id: u64, accepted: bool },
    Leave(u64),
}
#[derive(Clone, Debug, PartialEq)]
pub struct PvpView {
    pub visible: bool,
    pub scoreboard: bool,
    pub viewport: [f32; 2],
    pub snapshot: Option<PvpSnapshot>,
    pub pending: bool,
    pub error: String,
    pub filter: Option<BgTeam>,
    pub results_dismissed: bool,
}
impl Default for PvpView {
    fn default() -> Self {
        Self {
            visible: false,
            scoreboard: false,
            viewport: [1280.0, 720.0],
            snapshot: None,
            pending: false,
            error: String::new(),
            filter: None,
            results_dismissed: false,
        }
    }
}
impl PvpView {
    pub fn receive(&mut self, update: PvpStateUpdate) {
        self.pending = false;
        self.error = update.error.unwrap_or_default();
        if let Some(snapshot) = update.snapshot {
            let old_match = self.battleground().map(|b| b.match_id);
            let match_changed = old_match != snapshot.battleground.as_ref().map(|b| b.match_id);
            let just_ended = snapshot
                .battleground
                .as_ref()
                .is_some_and(|b| b.phase == BgPhase::Ended)
                && self
                    .battleground()
                    .is_none_or(|b| b.phase != BgPhase::Ended);
            if match_changed || just_ended {
                self.results_dismissed = false;
                self.scoreboard = false;
            }
            self.snapshot = Some(snapshot);
        }
    }
    pub fn battleground(&self) -> Option<&BattlegroundSnapshot> {
        self.snapshot.as_ref()?.battleground.as_ref()
    }
    pub fn invitation(&self) -> bool {
        self.battleground()
            .is_some_and(|b| b.phase == BgPhase::Waiting && !b.accepted)
    }
    pub fn action(&mut self, action: &str) -> Option<PvpAction> {
        match action {
            "pvp:close" => {
                self.visible = false;
                return None;
            }
            "pvp:scoreboard" => {
                self.results_dismissed = false;
                self.scoreboard = !self.scoreboard;
                return None;
            }
            "pvp:score-close" => {
                self.results_dismissed = true;
                self.scoreboard = false;
                return None;
            }
            "pvp:all" => {
                self.filter = None;
                return None;
            }
            "pvp:alliance" => {
                self.filter = Some(BgTeam::A);
                return None;
            }
            "pvp:horde" => {
                self.filter = Some(BgTeam::B);
                return None;
            }
            _ => {}
        }
        if self.pending {
            return None;
        }
        let request = match action {
            "pvp:query" => PvpAction::Query,
            "pvp:warmode" => {
                let mode = self.snapshot.as_ref()?.war_mode.as_ref()?;
                let permitted = if mode.desired {
                    mode.can_disable
                } else {
                    mode.can_enable
                };
                if !permitted {
                    return None;
                }
                PvpAction::WarMode(!mode.desired)
            }
            "pvp:queue"
                if self.battleground().is_none()
                    && self.snapshot.as_ref().is_none_or(|s| s.queue.is_none()) =>
            {
                PvpAction::Queue(1014)
            }
            "pvp:dequeue" if self.snapshot.as_ref().is_some_and(|s| s.queue.is_some()) => {
                PvpAction::Dequeue
            }
            "pvp:accept" | "pvp:decline" if self.invitation() => PvpAction::Respond {
                match_id: self.battleground()?.match_id,
                accepted: action == "pvp:accept",
            },
            "pvp:leave" if self.battleground().is_some_and(|b| b.accepted) => {
                PvpAction::Leave(self.battleground()?.match_id)
            }
            _ => return None,
        };
        self.pending = true;
        Some(request)
    }
}
fn text(name: &str, value: &str, rect: (f32, f32, f32, f32)) -> Element {
    label(name.into(), value, rect, (13.0, WHITE, "LEFT"))
}
fn frame(name: &str, size: (f32, f32), position: (f32, f32), children: Element) -> Element {
    let (width, height) = size;
    let (left, top) = position;
    rsx! { r#frame { name: {DynName(name.into())}, width, height, left, top,
    pos_type: "absolute", strata: FrameStrata::Dialog, mouse_enabled: true, {children} } }
}
fn queue(view: &PvpView) -> Element {
    if !view.visible {
        return vec![];
    }
    let mut children = window_chrome_no_portrait(
        "PVPUIFrame",
        (563.0, 428.0),
        "Player vs. Player",
        "pvp:close",
    );
    children.extend(text(
        "PVPQueueCategory",
        "Unrated",
        (24.0, 52.0, 150.0, 22.0),
    ));
    children.extend(text(
        "HonorFrameTitle",
        "Battlegrounds",
        (194.0, 52.0, 300.0, 22.0),
    ));
    children.extend(panel_button(
        "HonorFrameSpecificBattlegroundButton".into(),
        "Warsong Gulch",
        "",
        false,
        (194.0, 90.0, 340.0, 52.0),
    ));
    children.extend(text("HonorFrameDescription", "Capture the enemy flag and return it to your base.\nFirst team to capture three flags wins.\n10–90 • 5–10 players per team", (194.0, 157.0, 340.0, 100.0)));
    let queued = view.snapshot.as_ref().is_some_and(|s| s.queue.is_some());
    let status = if view.pending {
        "Waiting for server"
    } else if view.invitation() {
        "Your battleground is ready"
    } else if view.battleground().is_some() {
        "In battleground"
    } else if queued {
        "In queue"
    } else {
        "Specific Battlegrounds"
    };
    children.extend(text(
        "HonorFrameQueueStatus",
        status,
        (194.0, 295.0, 340.0, 22.0),
    ));
    children.extend(label(
        "PVPUIError".into(),
        &view.error,
        (24.0, 335.0, 500.0, 32.0),
        (12.0, "1,0.2,0.2,1", "LEFT"),
    ));
    children.extend(panel_button(
        "HonorFrameQueueButton".into(),
        if queued { "Leave Queue" } else { "Join Battle" },
        if queued { "pvp:dequeue" } else { "pvp:queue" },
        !view.pending && view.battleground().is_none(),
        (365.0, 388.0, 170.0, 22.0),
    ));
    frame("PVPUIFrame", (563.0, 428.0), (24.0, 90.0), children)
}
fn invite(view: &PvpView) -> Element {
    if !view.invitation() {
        return vec![];
    }
    let bg = view.battleground().unwrap();
    let mut children = window_chrome_no_portrait(
        "PVPReadyDialog",
        (306.0, 210.0),
        "Enter Battle",
        "pvp:decline",
    );
    children.extend(label(
        "PVPReadyDialogInstanceName".into(),
        &bg.name,
        (13.0, 52.0, 280.0, 24.0),
        (16.0, NORMAL_FONT_COLOR, "CENTER"),
    ));
    children.extend(label(
        "PVPReadyDialogLabel".into(),
        "Your battleground is ready!",
        (13.0, 95.0, 280.0, 30.0),
        (13.0, WHITE, "CENTER"),
    ));
    if let Some(seconds) = bg.invitation_time_left_secs {
        children.extend(text(
            "PVPReadyDialogTime",
            &format!("{seconds} seconds"),
            (60.0, 132.0, 190.0, 22.0),
        ));
    }
    for (name, value, action, x) in [
        ("EnterBattleButton", "Enter Battle", "pvp:accept", 31.0),
        ("LeaveQueueButton", "Leave Queue", "pvp:decline", 160.0),
    ] {
        children.extend(panel_button(
            format!("PVPReadyDialog{name}"),
            value,
            action,
            !view.pending,
            (x, 163.0, 115.0, 22.0),
        ));
    }
    frame(
        "PVPReadyDialog",
        (306.0, 210.0),
        (
            (view.viewport[0] - 306.0) / 2.0,
            (view.viewport[1] - 210.0) / 2.0,
        ),
        children,
    )
}
fn flag_text(flag: FlagState) -> String {
    match flag {
        FlagState::AtBase => "At base".into(),
        FlagState::Carried { .. } => "Carried".into(),
        FlagState::Dropped => "Dropped".into(),
    }
}
fn hud(view: &PvpView) -> Element {
    let Some(bg) = view.battleground().filter(|b| b.accepted) else {
        return vec![];
    };
    let mut children = label(
        "WorldStateScore".into(),
        &format!("Alliance {} / 3    Horde {} / 3", bg.score_a, bg.score_b),
        (0.0, 0.0, 430.0, 24.0),
        (16.0, NORMAL_FONT_COLOR, "CENTER"),
    );
    children.extend(label(
        "WorldStateFlags".into(),
        &format!(
            "Alliance flag: {}    Horde flag: {}",
            flag_text(bg.flags.flag_a),
            flag_text(bg.flags.flag_b)
        ),
        (0.0, 25.0, 430.0, 22.0),
        (12.0, WHITE, "CENTER"),
    ));
    children.extend(panel_button(
        "WorldStateScoreboardButton".into(),
        "Scoreboard",
        "pvp:scoreboard",
        true,
        (150.0, 49.0, 130.0, 22.0),
    ));
    frame(
        "WorldStateAlwaysUpFrame",
        (430.0, 72.0),
        ((view.viewport[0] - 430.0) / 2.0, 18.0),
        children,
    )
}
fn scoreboard(context: &SharedContext, view: &PvpView) -> Element {
    let Some(bg) = view.battleground().filter(|b| b.accepted) else {
        return vec![];
    };
    let ended = bg.phase == BgPhase::Ended;
    if !view.scoreboard && (!ended || view.results_dismissed) {
        return vec![];
    }
    let (name, height, row_top) = if ended {
        ("PVPMatchResults", 620.0, 145.0)
    } else {
        ("PVPMatchScoreboard", 420.0, 85.0)
    };
    let mut children =
        window_chrome_no_portrait(name, (1024.0, height), &bg.name, "pvp:score-close");
    if ended {
        let outcome = match bg.outcome {
            Some(BgOutcome::Winner(team)) if team == bg.team => "Victory",
            Some(BgOutcome::Winner(_)) => "Defeat",
            Some(BgOutcome::Draw) => "Draw",
            None => "Match complete",
        };
        children.extend(label(
            "PVPMatchOutcome".into(),
            outcome,
            (30.0, 45.0, 964.0, 35.0),
            (24.0, NORMAL_FONT_COLOR, "CENTER"),
        ));
        if let Some(honor) = bg.honor_reward {
            children.extend(text(
                "PVPMatchHonorReward",
                &format!("Honor: {honor}"),
                (30.0, 85.0, 300.0, 24.0),
            ));
        }
    }
    for (i, title) in [
        "Name", "KB", "HK", "Deaths", "Honor", "Damage", "Healing", "Captures", "Returns",
    ]
    .iter()
    .enumerate()
    {
        let (x, width) = column(i);
        children.extend(text(
            &format!("PVPMatchCategory{i}"),
            title,
            (38.0 + x, row_top - 28.0, width, 22.0),
        ));
    }
    children.extend(score_rows(context, view, bg, row_top));
    for (i, (title, action)) in [
        ("All", "pvp:all"),
        ("Alliance", "pvp:alliance"),
        ("Horde", "pvp:horde"),
    ]
    .iter()
    .enumerate()
    {
        children.extend(panel_button(
            format!("PVPScoreboardTab{}", i + 1),
            title,
            action,
            true,
            (30.0 + i as f32 * 110.0, height - 62.0, 105.0, 22.0),
        ));
    }
    children.extend(panel_button(
        "PVPMatchLeaveButton".into(),
        "Leave Battleground",
        "pvp:leave",
        !view.pending,
        (804.0, height - 62.0, 190.0, 22.0),
    ));
    children.extend(text(
        "PVPMatchError",
        &view.error,
        (30.0, height - 32.0, 700.0, 20.0),
    ));
    frame(
        name,
        (1024.0, height),
        (
            (view.viewport[0] - 1024.0) / 2.0,
            (view.viewport[1] - height) / 2.0,
        ),
        children,
    )
}
fn column(index: usize) -> (f32, f32) {
    [
        (0.0, 188.0),
        (188.0, 55.0),
        (243.0, 55.0),
        (298.0, 65.0),
        (363.0, 65.0),
        (428.0, 125.0),
        (553.0, 125.0),
        (678.0, 125.0),
        (803.0, 125.0),
    ][index]
}
fn score_rows(
    context: &SharedContext,
    view: &PvpView,
    bg: &BattlegroundSnapshot,
    top: f32,
) -> Element {
    let players: Vec<_> = bg
        .players
        .iter()
        .filter(|p| view.filter.is_none_or(|team| p.team == team))
        .collect();
    let height = (players.len() as f32 * 22.0).max(256.0);
    let geometry = pixel_geometry(256.0, height, 256.0);
    let offset = geometry.clamp(context.scroll_first_row("PVPMatchScrollBox"));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let mut content = Vec::new();
    for (index, player) in players.iter().enumerate() {
        // Combat counters remain uninstrumented. Dashes mean unknown, not measured zero.
        let values = [
            player.name.clone(),
            "—".into(),
            player.honorable_kills.map_or("—".into(), |n| n.to_string()),
            "—".into(),
            player.honor_level.map_or("—".into(), |n| n.to_string()),
            "—".into(),
            "—".into(),
            player.flag_captures.to_string(),
            player.flag_returns.to_string(),
        ];
        let suffixes = [
            "", "KB", "HK", "Deaths", "Honor", "Damage", "Healing", "Captures", "Returns",
        ];
        for (i, value) in values.iter().enumerate() {
            let (x, width) = column(i);
            content.extend(text(
                &format!("PVPMatchPlayer{}{}", player.character_id, suffixes[i]),
                value,
                (x, index as f32 * 22.0, width, 22.0),
            ));
        }
    }
    let bar = MinimalScrollBar {
        list: "PVPMatchScrollBox",
        left: 938.0,
        top: 0.0,
        height: 256.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    rsx! { r#frame { name: "PVPMatchScrollBox", width: 958.0, height: 256.0, left: 38.0, top,
    pos_type: "absolute", mouse_enabled: true, scroll_list: config,
    r#frame { name: "PVPMatchScrollChild", width: 928.0, height, left: 0.0, top: child_top, pos_type: "absolute", {content} }
    {bar.element()} } }
}
pub fn pvp_screen(context: &SharedContext) -> Element {
    let _ = context.get::<ui_toolkit::atlas::ActiveSkin>();
    let view = context.get::<PvpView>().expect("PvP view");
    let mut elements = queue(view);
    elements.extend(hud(view));
    elements.extend(scoreboard(context, view));
    elements.extend(invite(view));
    elements
}
