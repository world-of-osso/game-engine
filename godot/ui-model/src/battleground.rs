//! Retail Mainline PVPUIFrame, PVPReadyDialog, PVPMatchScoreboard and Results.
//! Layout authority: Blizzard_PVPUI/Mainline and Blizzard_PVPMatch XML/Lua.
use crate::bank_art::{WHITE, label};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, panel_button, window_chrome_no_portrait};
use shared::battleground::{BgOutcome, BgPhase, BgTeam, FlagState};
use shared::protocol::{
    ArenaQueueMode, BattlegroundSnapshot, PvpBracketSnapshot, PvpSnapshot, PvpStateUpdate,
};
#[path = "arena_ui.rs"]
mod arena_ui;
#[path = "pvp_bonus_ui.rs"]
mod bonus_ui;
#[path = "pvp_honor_ui.rs"]
mod honor_ui;
use ui_toolkit::{rsx, screen::SharedContext, strata::FrameStrata, widget_def::Element};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PvpAction {
    Query,
    WarMode(bool),
    SelectTalent { slot: u8, talent_id: u32 },
    Queue(u32),
    Arena(ArenaQueueMode),
    Rated(PvpBracketSnapshot),
    Target(u64),
    Dequeue,
    Respond { match_id: u64, accepted: bool },
    Leave(u64),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PvpCategory {
    #[default]
    QuickMatch,
    Rated,
    PremadeGroups,
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
    pub talent_slot: Option<u8>,
    pub category: PvpCategory,
    pub selected_arena: Option<ArenaQueueMode>,
    pub rated_bracket: PvpBracketSnapshot,
    pub selected_battleground: u32,
    pub honor_bonus: bool,
    pub honor_type_menu_open: bool,
    pub honor_faction: Option<crate::char_create_data::Faction>,
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
            talent_slot: None,
            category: PvpCategory::QuickMatch,
            selected_arena: None,
            rated_bracket: PvpBracketSnapshot::Arena2v2,
            selected_battleground: 1014,
            honor_bonus: false,
            honor_type_menu_open: false,
            honor_faction: None,
        }
    }
}
impl PvpView {
    pub fn receive(&mut self, update: PvpStateUpdate) {
        self.pending = false;
        self.error = update.error.unwrap_or_default();
        if let Some(snapshot) = update.snapshot {
            let previous_spec = self
                .snapshot
                .as_ref()
                .and_then(|s| s.talents.as_ref())
                .map(|t| t.spec_id);
            let next_spec = snapshot.talents.as_ref().map(|t| t.spec_id);
            if previous_spec != next_spec {
                self.talent_slot = None;
            }
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
            if snapshot.queue.is_some() || snapshot.battleground.is_some() {
                self.honor_type_menu_open = false;
            }
            self.snapshot = Some(snapshot);
        }
    }
    pub fn battleground(&self) -> Option<&BattlegroundSnapshot> {
        self.snapshot.as_ref()?.battleground.as_ref()
    }
    pub fn invitation(&self) -> bool {
        self.battleground().is_some_and(|bg| {
            !bg.accepted
                && match bg.phase {
                    BgPhase::Waiting => true,
                    BgPhase::InProgress => bg.arena.is_none(),
                    BgPhase::Ended => false,
                }
        })
    }
    pub fn action(&mut self, action: &str) -> Option<PvpAction> {
        if let Some(id) = action
            .strip_prefix("pvp:select:")
            .and_then(|id| id.parse().ok())
        {
            if !self.pending && specific_choices().iter().any(|template| template.id == id) {
                self.selected_battleground = id;
                self.selected_arena = None;
            }
            return None;
        }
        match action {
            "pvp:type:toggle" if !self.pending && self.battleground().is_none() => {
                self.honor_type_menu_open = !self.honor_type_menu_open;
                return None;
            }
            "pvp:type:bonus" | "pvp:type:specific"
                if !self.pending && self.battleground().is_none() =>
            {
                self.honor_bonus = action == "pvp:type:bonus";
                self.honor_type_menu_open = false;
                return None;
            }
            "pvp:bonus:random" if !self.pending && self.battleground().is_none() => {
                self.honor_bonus = true;
                return None;
            }
            "pvp:category:quick" | "pvp:category:rated" | "pvp:category:groups" => {
                self.honor_type_menu_open = false;
                self.category = match action {
                    "pvp:category:rated" => PvpCategory::Rated,
                    "pvp:category:groups" => PvpCategory::PremadeGroups,
                    _ => PvpCategory::QuickMatch,
                };
                return None;
            }
            "pvp:close" => {
                self.visible = false;
                self.honor_type_menu_open = false;
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
        if action == "pvp:talents-close" {
            self.talent_slot = None;
            return None;
        }
        if let Some(slot) = action
            .strip_prefix("pvp:talent-slot:")
            .and_then(|s| s.parse::<u8>().ok())
        {
            if self
                .snapshot
                .as_ref()?
                .talents
                .as_ref()?
                .slots
                .iter()
                .any(|s| s.index == slot && s.unlocked)
            {
                self.talent_slot = if self.talent_slot == Some(slot) {
                    None
                } else {
                    Some(slot)
                };
            }
            return None;
        }
        if let Some(entity) = action
            .strip_prefix("pvp:target:")
            .and_then(|id| id.parse::<u64>().ok())
        {
            let bg = self.battleground()?;
            let unit = bg
                .arena
                .as_ref()?
                .units
                .iter()
                .find(|unit| unit.entity == entity && unit.team != bg.team)?;
            return Some(PvpAction::Target(unit.entity));
        }
        if self.pending {
            return None;
        }
        if matches!(
            action,
            "pvp:rated:2v2" | "pvp:rated:3v3" | "pvp:rated:shuffle"
        ) {
            self.rated_bracket = match action {
                "pvp:rated:2v2" => PvpBracketSnapshot::Arena2v2,
                "pvp:rated:3v3" => PvpBracketSnapshot::Arena3v3,
                _ => PvpBracketSnapshot::SoloShuffle,
            };
            return None;
        }
        if action.starts_with("pvp:activity:") {
            self.selected_arena = match action {
                "pvp:activity:warsong" => None,
                "pvp:activity:2v2" => Some(ArenaQueueMode::Skirmish2v2),
                "pvp:activity:3v3" => Some(ArenaQueueMode::Skirmish3v3),
                _ => return None,
            };
            return None;
        }
        if let Some(id) = action
            .strip_prefix("pvp:talent-choice:")
            .and_then(|s| s.parse::<u32>().ok())
        {
            let slot = self.talent_slot?;
            let talents = self.snapshot.as_ref()?.talents.as_ref()?;
            let target = talents
                .slots
                .iter()
                .find(|s| s.index == slot && s.can_change)?;
            let choice = talents.choices.iter().find(|c| {
                c.id == id && c.slot_mask & (1 << slot) != 0 && c.unavailable_reason.is_none()
            })?;
            if target.selected == Some(choice.id)
                || talents.slots.iter().any(|s| s.selected == Some(choice.id))
            {
                return None;
            }
            self.pending = true;
            self.talent_slot = None;
            return Some(PvpAction::SelectTalent {
                slot,
                talent_id: id,
            });
        }
        let request = match action {
            "pvp:query" => PvpAction::Query,
            "pvp:rated:queue"
                if self.battleground().is_none()
                    && self
                        .snapshot
                        .as_ref()
                        .is_none_or(|snapshot| snapshot.queue.is_none()) =>
            {
                if self.rated_bracket == PvpBracketSnapshot::SoloShuffle {
                    PvpAction::Arena(ArenaQueueMode::SoloShuffle)
                } else {
                    PvpAction::Rated(self.rated_bracket.clone())
                }
            }
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
                if self.honor_bonus {
                    PvpAction::Queue(shared::battleground::RANDOM_BATTLEGROUND_ID)
                } else {
                    match self.selected_arena {
                        Some(mode) => PvpAction::Arena(mode),
                        None => PvpAction::Queue(self.selected_battleground),
                    }
                }
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
        self.honor_type_menu_open = false;
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
fn queue_frame(width: f32, children: Element) -> Element {
    rsx! { r#frame { name: "PVPUIFrame", width, height: 428.0, left: 100.0, top: 84.0,
    pos_type: "absolute", strata: FrameStrata::Medium, mouse_enabled: true, {children} } }
}
fn specific_choices() -> Vec<shared::battleground::BgTemplate> {
    shared::battleground::all_templates()
        .into_iter()
        .filter(|template| shared::battleground::has_implemented_objectives(template.map_id))
        .collect()
}

fn queue(context: &SharedContext, view: &PvpView) -> Element {
    if !view.visible {
        return vec![];
    }
    // PVPUI.lua:1812,1823: Quick/Rated show the225px HonorInset beside the563px PVE base.
    let width = if view.category == PvpCategory::PremadeGroups {
        563.0
    } else {
        563.0 + 225.0
    };
    let mut children = window_chrome_no_portrait(
        "PVPUIFrame",
        (width, 428.0),
        "Player vs. Player",
        "pvp:close",
    );
    for (index, (title, action)) in [
        ("Quick Match", "pvp:category:quick"),
        ("Rated", "pvp:category:rated"),
        ("Premade Groups", "pvp:category:groups"),
    ]
    .iter()
    .enumerate()
    {
        children.extend(crate::selection_list::text_row(
            format!("PVPQueueFrameCategoryButton{}", index + 1),
            title,
            action,
            (
                true,
                view.category
                    == [
                        PvpCategory::QuickMatch,
                        PvpCategory::Rated,
                        PvpCategory::PremadeGroups,
                    ][index],
            ),
            (10.0, 66.0 + index as f32 * 80.0, 168.0, 50.0),
            &crate::selection_list::PVP_CATEGORY,
        ));
    }
    if view.category != PvpCategory::PremadeGroups {
        children.extend(honor_ui::honor_inset(view));
    }
    if view.category != PvpCategory::QuickMatch {
        children.extend(category_panel(view));
        return queue_frame(width, children);
    }
    if let Some(wallet) = &view.snapshot {
        children.extend(text(
            "PVPUIHonorCurrency",
            &format!("Honor: {} / {}", wallet.honor, wallet.honor_max),
            (236.0, 41.0, 370.0, 22.0),
        ));
    }

    if view.honor_bonus {
        children.extend(bonus_ui::bonus_panel(view));
    } else {
        children.extend(specific_list(context, view));
    }
    let queued = view.snapshot.as_ref().is_some_and(|s| s.queue.is_some());
    let status = if view.pending {
        "Waiting for server"
    } else if view.invitation() {
        "Your battleground is ready"
    } else if view.battleground().is_some() {
        "In battleground"
    } else if queued {
        "In queue"
    } else if view.honor_bonus {
        "Random Battleground"
    } else {
        "Specific Battlegrounds"
    };
    children.extend(hidden_text("HonorFrameQueueStatus", status));
    children.extend(label(
        "PVPUIError".into(),
        &view.error,
        (
            (view.viewport[0] - 512.0) / 2.0 - 100.0,
            122.0 - 84.0,
            512.0,
            60.0,
        ),
        (16.0, "1,0.2,0.2,1", "CENTER"),
    ));
    children.extend(panel_button(
        "HonorFrameQueueButton".into(),
        if queued { "Leave Queue" } else { "Join Battle" },
        if queued { "pvp:dequeue" } else { "pvp:queue" },
        !view.pending && view.battleground().is_none(),
        (363.0, 404.0, 116.0, 22.0),
    ));
    // Keep the dropdown/menu after the list in native GUI sibling order as well as strata.
    children.extend(bonus_ui::type_selector(view));
    queue_frame(width, children)
}
pub const SPECIFIC_LIST: &str = "HonorFrameSpecificScrollBox";
const SPECIFIC_ROW_H: f32 = 40.0;
pub const SCORE_LIST: &str = "PVPMatchScrollBox";
const SCORE_ROW_H: f32 = 22.0;

/// ScrollBoxListView.lua:637-661: a list pans by its first entry's extent (no spacing here).
pub fn pvp_pan_extent(list: &str) -> Option<usize> {
    match list {
        SPECIFIC_LIST => Some(SPECIFIC_ROW_H as usize),
        SCORE_LIST => Some(SCORE_ROW_H as usize),
        _ => None,
    }
}

fn specific_list(context: &SharedContext, view: &PvpView) -> Element {
    let templates = specific_choices();
    let height = templates.len() as f32 * SPECIFIC_ROW_H + 2.0 * 55.0;
    // HonorFrame(402,403), Inset LEFT4/TOP85/RIGHT-5/BOTTOM22, ScrollBox inset2/21/2/2.
    let geometry = pixel_geometry(292.0, height, 284.0);
    let offset = geometry.clamp(context.scroll_first_row(SPECIFIC_LIST));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let mut children: Element = templates
        .iter()
        .enumerate()
        .flat_map(|(index, template)| {
            crate::selection_list::text_row(
                format!("HonorFrameSpecificBattlegroundButton{}", template.id),
                &template.name,
                &format!("pvp:select:{}", template.id),
                (!view.pending, view.selected_battleground == template.id),
                (0.0, index as f32 * SPECIFIC_ROW_H, 324.0, SPECIFIC_ROW_H),
                &crate::selection_list::PVP_INSTANCE,
            )
        })
        .collect();
    children.extend(arena_ui::queue_choices(
        view,
        templates.len() as f32 * SPECIFIC_ROW_H,
    ));
    let bar = MinimalScrollBar {
        list: SPECIFIC_LIST,
        // HonorFrame's sibling ScrollBar TOPRIGHT(-22,-92), not a clipped list child.
        left: 220.0 + 402.0 - 22.0,
        top: 22.0 + 92.0,
        height: 284.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    };
    let selected = templates
        .iter()
        .find(|template| template.id == view.selected_battleground)
        .expect("selection belongs to the supported source list");
    let mut elements = rsx! { r#frame { name: "HonorFrameSpecificScrollBox", width: 370.0,
    height: 292.0, left: 226.0, top: 109.0, pos_type: "absolute", mouse_enabled: true,
    scroll_list: config,
    r#frame { name: "HonorFrameSpecificScrollChild", width: 370.0, height, left: 0.0,
        top: child_top, pos_type: "absolute", {children} }
    } {bar.element()} };
    elements.extend(hidden_text(
        "HonorFrameDescription",
        &view.selected_arena.map_or_else(
            || format!("Selected: {}", selected.name),
            |mode| match mode {
                ArenaQueueMode::Skirmish2v2 => "Selected: Arena Skirmish 2v2".into(),
                ArenaQueueMode::Skirmish3v3 => "Selected: Arena Skirmish 3v3".into(),
                _ => unreachable!("Quick Match only selects skirmishes"),
            },
        ),
    ));
    elements
}

// Existing automation receipt, not a visible Retail row; selection is shown by source selected art.
fn hidden_text(name: &str, text: &str) -> Element {
    rsx! { fontstring { name: {DynName(name.into())}, text, hidden: true } }
}

fn category_panel(view: &PvpView) -> Element {
    if view.category == PvpCategory::PremadeGroups {
        return text(
            "LFGListPVPStub",
            "Premade Groups\nGroup listing service unavailable",
            (194.0, 52.0, 340.0, 100.0),
        );
    }
    arena_ui::rated_panel(view)
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
    // StaticPopup.lua: first standalone popup TOP at fullscreen TOP -135.
    const READY_DIALOG_TOP: f32 = 135.0;
    frame(
        "PVPReadyDialog",
        (306.0, 210.0),
        ((view.viewport[0] - 306.0) / 2.0, READY_DIALOG_TOP),
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
fn score_text(bg: &BattlegroundSnapshot) -> String {
    let template =
        shared::battleground::template_by_id(bg.battleground_id).expect("source match template");
    let limit = if template.objective == shared::battleground::BgObjective::Reinforcements {
        template.starting_reinforcements
    } else {
        template.win_score
    };
    format!(
        "Alliance {} / {}    Horde {} / {}",
        bg.score_a, limit, bg.score_b, limit
    )
}
fn world_state(bg: &BattlegroundSnapshot, id: u32) -> Option<i32> {
    bg.world_states
        .iter()
        .find(|(state, _)| *state == id)
        .map(|(_, value)| *value)
}
fn hud_cells(cells: Vec<(String, String)>) -> Element {
    let width = 430.0 / cells.len() as f32;
    cells
        .into_iter()
        .enumerate()
        .flat_map(|(index, (name, text))| {
            label(
                name,
                &text,
                (index as f32 * width, 25.0, width, 24.0),
                (9.0, WHITE, "CENTER"),
            )
        })
        .collect()
}
/// Eye of the Storm: tower control WorldStates and Netherstorm flag WorldState 8863.
fn eye_hud(rules: &shared::battleground::EyeRules, bg: &BattlegroundSnapshot) -> Element {
    let mut cells: Vec<_> = rules
        .towers
        .iter()
        .map(|tower| {
            let owner = [
                (tower.alliance_state, "Alliance"),
                (tower.horde_state, "Horde"),
                (tower.uncontrolled_state, "Uncontrolled"),
            ]
            .into_iter()
            .find(|(state, _)| world_state(bg, *state) == Some(1))
            .map_or("—", |(_, owner)| owner);
            (
                format!("WorldStateNode{}", tower.entry),
                format!("{}\n{owner}", tower.name),
            )
        })
        .collect();
    let flag = match world_state(bg, 8863) {
        Some(2) => "Carried",
        Some(3) => "Dropped",
        Some(4) => "Respawning",
        _ => "At base",
    };
    cells.push((
        "WorldStateEyeFlag".into(),
        format!("Netherstorm Flag\n{flag}"),
    ));
    hud_cells(cells)
}
fn kotmogu_hud(bg: &BattlegroundSnapshot) -> Element {
    let cells = shared::battleground::KOTMOGU_ORBS
        .iter()
        .map(|orb| {
            let [base, horde, alliance] = orb.world_states;
            let owner = [(alliance, "Alliance"), (horde, "Horde"), (base, "At base")]
                .into_iter()
                .find(|(state, _)| world_state(bg, *state) == Some(1))
                .map_or("—", |(_, owner)| owner);
            (
                format!("WorldStateOrb{}", orb.entry),
                format!("{}\n{owner}", orb.name),
            )
        })
        .collect();
    hud_cells(cells)
}

fn cart_cells(
    bg: &BattlegroundSnapshot,
    definitions: impl Iterator<Item = (&'static str, [u32; 2])>,
) -> Vec<(String, String)> {
    definitions
        .enumerate()
        .map(|(index, (name, states))| {
            let owner = match (world_state(bg, states[0]), world_state(bg, states[1])) {
                (Some(1), Some(1)) => "—",
                (Some(1), _) => "Alliance",
                (_, Some(1)) => "Horde",
                (Some(0), Some(0)) => "Neutral",
                _ => "—",
            };
            (format!("WorldStateCart{index}"), format!("{name}\n{owner}"))
        })
        .collect()
}

fn silvershard_hud(bg: &BattlegroundSnapshot) -> Element {
    hud_cells(cart_cells(
        bg,
        shared::battleground::SILVERSHARD_CARTS
            .iter()
            .map(|cart| (cart.name, cart.world_states)),
    ))
}

fn deephaul_hud(bg: &BattlegroundSnapshot) -> Element {
    let mut cells = cart_cells(
        bg,
        shared::battleground::DEEPHAUL_CARTS
            .iter()
            .map(|cart| (cart.name, cart.world_states)),
    );
    let crystal = if world_state(bg, 1545) == Some(2) {
        "Alliance"
    } else if world_state(bg, 1546) == Some(2) {
        "Horde"
    } else {
        match world_state(bg, 25412) {
            Some(3) => "Unclaimed",
            Some(0) => "Inactive",
            _ => "—",
        }
    };
    cells.push((
        "WorldStateCrystal".into(),
        format!("Deephaul Crystal\n{crystal}"),
    ));
    hud_cells(cells)
}

fn objective_hud(bg: &BattlegroundSnapshot) -> Element {
    if bg.map_id == 2656 {
        return deephaul_hud(bg);
    }
    if matches!(bg.map_id, 30 | 628 | 1803) {
        // Reinforcements/resource race: no unrelated team-flag cells.
        return vec![];
    }
    if bg.map_id == 727 {
        return silvershard_hud(bg);
    }
    if bg.map_id == 998 {
        return kotmogu_hud(bg);
    }
    if let Some(rules) = shared::battleground::eye_rules(bg.map_id) {
        return eye_hud(rules, bg);
    }
    if let Some(rules) = shared::battleground::node_rules(bg.map_id) {
        let cells = rules.nodes.iter().map(|node| {
            let status = match world_state(bg, node.poi) {
                Some(1) => "Neutral",
                Some(2) => "Horde assault",
                Some(3) => "Alliance assault",
                Some(4) => "Horde",
                Some(5) => "Alliance",
                _ => "—",
            };
            (
                format!("WorldStateNode{}", node.entry),
                format!("{}\n{status}", node.name),
            )
        });
        return hud_cells(cells.collect());
    }
    label(
        "WorldStateFlags".into(),
        &format!(
            "Alliance flag: {}    Horde flag: {}",
            flag_text(bg.flags.flag_a),
            flag_text(bg.flags.flag_b)
        ),
        (0.0, 25.0, 430.0, 22.0),
        (12.0, WHITE, "CENTER"),
    )
}
fn objective_value(
    player: &shared::protocol::BattlegroundPlayerScoreSnapshot,
    stat: u32,
) -> String {
    player
        .objective_scores
        .iter()
        .find(|(id, _)| *id == stat)
        .map_or("—".into(), |(_, value)| value.to_string())
}

fn hud(view: &PvpView) -> Element {
    let Some(bg) = view.battleground().filter(|b| b.accepted) else {
        return vec![];
    };
    if bg.arena.is_some() {
        return arena_ui::arena_hud(view, bg);
    }
    let mut children = label(
        "WorldStateScore".into(),
        &score_text(bg),
        (0.0, 0.0, 430.0, 24.0),
        (16.0, NORMAL_FONT_COLOR, "CENTER"),
    );
    children.extend(objective_hud(bg));
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
        let outcome = if bg
            .arena
            .as_ref()
            .is_some_and(|arena| arena.shuffle.is_some())
        {
            "Solo Shuffle complete"
        } else {
            match bg.outcome {
                Some(BgOutcome::Winner(team)) if team == bg.team => "Victory",
                Some(BgOutcome::Winner(_)) => "Defeat",
                Some(BgOutcome::Draw) => "Draw",
                None => "Match complete",
            }
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
    let stats = shared::battleground::pvp_stat_columns(bg.map_id);
    let arena_titles = arena_stat_titles(bg);
    let titles = BASE_TITLES
        .iter()
        .copied()
        .chain(stats.iter().map(|stat| stat.title))
        .chain(arena_titles.iter().copied());
    for (i, title) in titles.enumerate() {
        let (x, width) = column(i, stats.len() + arena_titles.len());
        children.extend(text(
            &format!("PVPMatchCategory{i}"),
            title,
            (38.0 + x, row_top - 28.0, width, 22.0),
        ));
    }
    children.extend(score_rows(context, view, bg, row_top));
    // Scoreboard.xml: Content(30,35), ScrollCategories TOP3+50, ScrollBox256,
    // TabGroup TOP(-50), first tab TOPLEFT(+0,+46); AnchorTabs uses +3 horizontal spacing.
    let mut tab_left = 40.0;
    let content_top = if height == 620.0 { 90.0 } else { 35.0 };
    for (i, (title, action)) in [
        ("All", "pvp:all"),
        ("Alliance", "pvp:alliance"),
        ("Horde", "pvp:horde"),
    ]
    .iter()
    .enumerate()
    {
        let title = if bg.arena.is_some() && i == 1 {
            "Gold"
        } else if bg.arena.is_some() && i == 2 {
            "Green"
        } else {
            title
        };
        let width = crate::merchant_frame_component::tab_width(title);
        let selected = view.filter
            == match i {
                1 => Some(BgTeam::A),
                2 => Some(BgTeam::B),
                _ => None,
            };
        children.extend(crate::merchant_frame_component::tab(
            &format!("PVPScoreboardTab{}", i + 1),
            title,
            (
                tab_left,
                content_top + 3.0 + 50.0 + 256.0 + 50.0 - 46.0,
                width,
            ),
            selected,
            action,
        ));
        tab_left += width + 3.0;
    }
    children.extend(panel_button(
        "PVPMatchLeaveButton".into(),
        if bg.arena.is_some() {
            "Leave Arena"
        } else {
            "Leave Battleground"
        },
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
fn arena_stat_titles(bg: &BattlegroundSnapshot) -> &'static [&'static str] {
    if bg
        .arena
        .as_ref()
        .is_some_and(|arena| arena.shuffle.is_some())
    {
        return &["Rounds Won"];
    }
    if bg
        .arena
        .as_ref()
        .is_some_and(|arena| !arena.rating_changes.is_empty())
    {
        &["Rating", "MMR"]
    } else {
        &[]
    }
}
const BASE_TITLES: [&str; 7] = ["Name", "KB", "HK", "Deaths", "Honor", "Damage", "Healing"];
const BASE_COLUMNS: [(f32, f32); 7] = [
    (0.0, 188.0),
    (188.0, 55.0),
    (243.0, 55.0),
    (298.0, 65.0),
    (363.0, 65.0),
    (428.0, 125.0),
    (553.0, 125.0),
];
/// Source stat columns share the remaining scroll-child width after the base columns.
fn column(index: usize, stats: usize) -> (f32, f32) {
    if let Some(base) = BASE_COLUMNS.get(index) {
        return *base;
    }
    let width = 250.0 / stats as f32;
    (678.0 + (index - BASE_COLUMNS.len()) as f32 * width, width)
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
    let height = (players.len() as f32 * SCORE_ROW_H).max(256.0);
    let geometry = pixel_geometry(256.0, height, 256.0);
    let offset = geometry.clamp(context.scroll_first_row(SCORE_LIST));
    let config = scroll_list_attr(&geometry);
    let child_top = -(offset as f32);
    let stats = shared::battleground::pvp_stat_columns(bg.map_id);
    let mut content = Vec::new();
    for (index, player) in players.iter().enumerate() {
        let rating = bg.arena.as_ref().and_then(|arena| {
            arena
                .rating_changes
                .iter()
                .find(|change| change.character_id == player.character_id)
        });
        // Combat counters remain uninstrumented. Dashes mean unknown, not measured zero.
        let combat_value = |value: u64| {
            if bg.arena.is_some() {
                value.to_string()
            } else {
                "—".into()
            }
        };
        let base = [
            (String::new(), player.name.clone()),
            ("KB".into(), combat_value(u64::from(player.killing_blows))),
            (
                "HK".into(),
                player.honorable_kills.map_or("—".into(), |n| n.to_string()),
            ),
            ("Deaths".into(), combat_value(u64::from(player.deaths))),
            (
                "Honor".into(),
                player.honor_level.map_or("—".into(), |n| n.to_string()),
            ),
            ("Damage".into(), combat_value(player.damage_done)),
            ("Healing".into(), combat_value(player.healing_done)),
        ];
        let objectives = stats.iter().map(|stat| {
            (
                format!("Stat{}", stat.stat),
                objective_value(player, stat.stat),
            )
        });
        let ratings = rating.into_iter().flat_map(|change| {
            [
                (
                    "Rating".into(),
                    format!(
                        "{} ({:+})",
                        change.rating_after,
                        i64::from(change.rating_after) - i64::from(change.rating_before)
                    ),
                ),
                ("MMR".into(), change.mmr_after.to_string()),
            ]
        });
        let rounds = bg
            .arena
            .as_ref()
            .and_then(|arena| arena.shuffle.as_ref())
            .and_then(|shuffle| {
                shuffle
                    .rounds_won
                    .iter()
                    .find(|(id, _)| *id == player.character_id)
            })
            .map(|(_, wins)| ("RoundsWon".into(), wins.to_string()));
        for (i, (suffix, value)) in base
            .into_iter()
            .chain(objectives)
            .chain(ratings)
            .chain(rounds)
            .enumerate()
        {
            let (x, width) = column(i, stats.len() + arena_stat_titles(bg).len());
            content.extend(text(
                &format!("PVPMatchPlayer{}{suffix}", player.character_id),
                &value,
                (x, index as f32 * SCORE_ROW_H, width, SCORE_ROW_H),
            ));
        }
    }
    let bar = MinimalScrollBar {
        list: SCORE_LIST,
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
    let mut elements = queue(context, view);
    elements.extend(hud(view));
    elements.extend(scoreboard(context, view));
    elements.extend(invite(view));
    elements
}
