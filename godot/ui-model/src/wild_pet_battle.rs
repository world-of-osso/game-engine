//! Retail PetBattleFrame: identical controls/layout for Modern and Forever art skins.
use crate::bank_art::{WHITE, cropped, label, texture};
use crate::pet_journal::PetSpeciesVisual;
use crate::quest_art::{DynName, panel_button};
use shared::protocol::*;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const MODEL_SCENE: &str = "PetBattleModelScene";
const HUD_TEXTURE: u32 = 603587;
const HORIZONTAL_TILE: u32 = 603600;
const WHITE_PIXEL: u32 = 130871;

#[derive(Clone, Debug, PartialEq)]
pub struct WildBattleView {
    pub viewport: [f32; 2],
    pub state: Option<WildPetBattleSnapshot>,
    pub pending: bool,
    pub combat_text: Vec<String>,
    pub result: Option<WildPetBattleOutcome>,
    pub confirm_forfeit: bool,
    pub show_swap: bool,
    pub error: String,
    pub feedback_seconds: f32,
}
impl Default for WildBattleView {
    fn default() -> Self {
        Self {
            viewport: [1280.0, 720.0],
            state: None,
            pending: false,
            combat_text: vec![],
            result: None,
            confirm_forfeit: false,
            show_swap: false,
            error: String::new(),
            feedback_seconds: 0.0,
        }
    }
}
impl WildBattleView {
    pub fn receive(&mut self, update: WildPetBattleUpdate) {
        self.pending = false;
        self.error.clear();
        match update {
            WildPetBattleUpdate::Start(state) => {
                self.state = Some(state);
                self.result = None;
                self.combat_text.clear();
            }
            WildPetBattleUpdate::State(state) => {
                if !state.feedback.is_empty() {
                    self.feedback_seconds = 2.0;
                }
                self.state = Some(state);
            }
            WildPetBattleUpdate::Round { state, combat_text } => {
                self.feedback_seconds = 2.0;
                self.state = Some(state);
                self.combat_text = combat_text;
            }
            WildPetBattleUpdate::End {
                battle_id,
                outcome,
                combat_text,
                rewards,
                captured_pet_id,
            } => {
                if self
                    .state
                    .as_ref()
                    .is_none_or(|state| state.battle_id != battle_id)
                {
                    return;
                }
                self.result = Some(outcome);
                let state = self.state.as_mut().expect("matching battle checked");
                for reward in &rewards {
                    if let Some(pet) = state.teams[0]
                        .iter_mut()
                        .find(|pet| pet.instance_id == Some(reward.instance_id))
                    {
                        pet.level = reward.level;
                        pet.xp = reward.xp;
                        pet.next_level_xp = reward.next_level_xp;
                    }
                }
                self.combat_text = combat_text;
                self.combat_text.extend(
                    rewards
                        .iter()
                        .map(|r| format!("Pet {} gained {} XP", r.instance_id, r.xp_gained)),
                );
                if captured_pet_id.is_some() {
                    self.combat_text
                        .push("Captured pet added to journal".into());
                }
            }
            WildPetBattleUpdate::Rejected(error) => self.error = error,
        }
    }
    pub fn action(&mut self, action: &str) -> Option<WildPetBattleActionRequest> {
        if action == "pb:close" {
            self.state = None;
            return None;
        }
        let state = self.state.as_ref()?;
        if self.pending || self.result.is_some() {
            return None;
        }
        if action == "pb:swap" {
            self.show_swap = !self.show_swap;
            return None;
        }
        if action == "pb:forfeit" {
            self.confirm_forfeit = true;
            return None;
        }
        if action == "pb:cancel-forfeit" {
            self.confirm_forfeit = false;
            return None;
        }
        let pet = &state.teams[0][usize::from(state.active[0])];
        let decision = if let Some(button) = action.strip_prefix("pb:ability:") {
            let button: u8 = button.parse().ok()?;
            if !(1..=3).contains(&button)
                || !pet.abilities[usize::from(button - 1)].usable
                || state.replacement_required
            {
                return None;
            }
            WildPetBattleAction::Ability(button)
        } else if let Some(slot) = action.strip_prefix("pb:swap:") {
            let slot: u8 = slot.parse().ok()?;
            let index = usize::from(slot.checked_sub(1)?);
            if index == usize::from(state.active[0]) || state.teams[0].get(index)?.health <= 0 {
                return None;
            }
            WildPetBattleAction::Swap(slot)
        } else {
            match action {
                "pb:trap" if state.can_trap => WildPetBattleAction::Trap,
                "pb:pass" if !state.replacement_required => WildPetBattleAction::Pass,
                "pb:confirm-forfeit" if self.confirm_forfeit => WildPetBattleAction::Forfeit,
                _ => return None,
            }
        };
        self.pending = true;
        self.show_swap = false;
        self.confirm_forfeit = false;
        Some(WildPetBattleActionRequest {
            battle_id: state.battle_id,
            round: state.round,
            action: decision,
        })
    }
}

pub fn wild_battle_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let Some(view) = ctx.get::<WildBattleView>() else {
        return vec![];
    };
    let Some(state) = view.state.as_ref() else {
        return vec![];
    };
    let [width, height] = view.viewport;
    let mut children = cropped(
        "PetBattleTopLeft".into(),
        HUD_TEXTURE,
        "0.00097656,0.56152344,0.00195313,0.23242188",
        (0.0, 0.0, 574.0, 118.0),
    );
    children.extend(cropped(
        "PetBattleTopRight".into(),
        HUD_TEXTURE,
        "0.56152344,0.00097656,0.00195313,0.23242188",
        (width - 574.0, 0.0, 574.0, 118.0),
    ));
    for (team, side) in ["Ally", "Enemy"].iter().enumerate() {
        children.extend(unit_frame(
            side,
            &state.teams[team][usize::from(state.active[team])],
            if team == 0 { 115.0 } else { width - 385.0 },
            5.0,
        ));
        children.extend(reserve_frames(
            side,
            &state.teams[team],
            state.active[team],
            if team == 0 { 65.0 } else { width - 103.0 },
        ));
    }
    children.extend(action_bar(view, state));
    children.extend(label(
        "PetBattleError".into(),
        &view.error,
        (width / 2.0 - 250.0, height - 310.0, 500.0, 24.0),
        (14.0, "1,0.2,0.2,1", "CENTER"),
    ));
    let left = width * 0.15;
    let top = height * 0.2;
    let model_width = width * 0.7;
    let model_height = height * 0.5;
    children.extend(rsx! { r#frame { name: "PetBattleModelScene", width: model_width, height: model_height, left, top, pos_type: "absolute", mouse_enabled: false } });
    let mut floating = vec![];
    if view.feedback_seconds > 0.0 {
        for (index, feedback) in state.feedback.iter().enumerate() {
            if state.active[usize::from(feedback.team)] != feedback.slot {
                continue;
            }
            let x = if feedback.team == 0 {
                width * 0.32
            } else {
                width * 0.68
            };
            let rise = (2.0 - view.feedback_seconds) * 24.0;
            floating.extend(label(
                format!("PetBattleFloating{index}"),
                &feedback.text,
                (
                    x - 60.0,
                    height * 0.42 - rise - index as f32 * 28.0,
                    120.0,
                    32.0,
                ),
                (
                    28.0,
                    if feedback.healing {
                        "0.2,1,0.2,1"
                    } else {
                        WHITE
                    },
                    "CENTER",
                ),
            ));
        }
    }
    children.extend(rsx! { r#frame { name: "PetBattleFloatingLayer", width, height, left: 0.0, top: 0.0, pos_type: "absolute", mouse_enabled: false, strata: ui_toolkit::strata::FrameStrata::Dialog, {floating} } });

    if view.show_swap || state.replacement_required {
        children.extend(swap_panel(view, state));
    }
    if view.confirm_forfeit {
        children.extend(forfeit_popup(width, height));
    }
    if let Some(result) = &view.result {
        children.extend(label(
            "PetBattleOutcome".into(),
            &format!("{result:?}"),
            (width / 2.0 - 150.0, height / 2.0 - 60.0, 300.0, 50.0),
            (28.0, "1,0.82,0,1", "CENTER"),
        ));
        children.extend(panel_button(
            "PetBattleClose".into(),
            "Continue",
            "pb:close",
            true,
            (width / 2.0 - 70.0, height / 2.0, 140.0, 28.0),
        ));
    }
    rsx! { r#frame { name: "PetBattleFrame", width, height, left: 0.0, top: 0.0, pos_type: "absolute", mouse_enabled: false, {children} } }
}
fn unit_frame(side: &str, pet: &BattlePetSnapshot, left: f32, top: f32) -> Element {
    let enemy = side == "Enemy";
    let icon_x = if enemy { 202.0 } else { 0.0 };
    let bar_x = if enemy { 39.0 } else { 76.0 };
    let mut children = texture(
        format!("PetBattle{side}Icon"),
        pet.icon,
        (icon_x, 6.0, 68.0, 68.0),
        WHITE,
    );
    let quality = quality_color(pet.quality);
    let border_name = DynName(format!("PetBattle{side}Border"));
    let border_x = icon_x - 21.0;
    children.extend(rsx! { texture { name: {border_name}, width: 110.0, height: 109.0, texture_fdid: HUD_TEXTURE, tex_coords: "0.86230469,0.96972656,0.03906250,0.25195313", vertex_color: quality, left: border_x, top: -14.5, pos_type: "absolute" } });
    children.extend(label(
        format!("PetBattle{side}Name"),
        &pet.name,
        (if enemy { -10.0 } else { 81.0 }, 6.0, 190.0, 24.0),
        (14.0, "1,0.82,0,1", if enemy { "RIGHT" } else { "LEFT" }),
    ));
    children.extend(cropped(
        format!("PetBattle{side}HealthBackground"),
        HUD_TEXTURE,
        "0.56347656,0.71484375,0.85351563,0.94531250",
        (bar_x, 27.0, 155.0, 47.0),
    ));
    let health_width = 145.0 * pet.health.max(0) as f32 / pet.max_health as f32;
    children.extend(texture(
        format!("PetBattle{side}HealthFill"),
        WHITE_PIXEL,
        (bar_x + 5.0, 40.0, health_width, 21.0),
        "0.15,0.75,0.2,1",
    ));
    children.extend(cropped(
        format!("PetBattle{side}HealthFrame"),
        HUD_TEXTURE,
        "0.3984375,0.54980469,0.43359375,0.52539063",
        (bar_x, 27.0, 155.0, 47.0),
    ));
    children.extend(label(
        format!("PetBattle{side}Health"),
        &format!("{} / {}", pet.health, pet.max_health),
        (bar_x + 5.0, 40.0, 145.0, 21.0),
        (12.0, WHITE, "CENTER"),
    ));
    children.extend(cropped(
        format!("PetBattle{side}LevelBubble"),
        HUD_TEXTURE,
        "0.46484375,0.48828125,0.23632813,0.28320313",
        (icon_x - 3.0, 55.0, 24.0, 24.0),
    ));
    children.extend(label(
        format!("PetBattle{side}Level"),
        &pet.level.to_string(),
        (icon_x - 3.0, 55.0, 24.0, 24.0),
        (12.0, WHITE, "CENTER"),
    ));
    let family = PetSpeciesVisual {
        species_id: pet.species_id,
        display_id: pet.display_id,
        family: pet.family,
    };
    children.extend(cropped(
        format!("PetBattle{side}Type"),
        family.type_icon(),
        "0.49218750,0.79687500,0.50390625,0.65625000",
        (if enemy { 0.0 } else { 234.0 }, 32.0, 34.0, 34.0),
    ));
    for (index, aura) in pet.auras.iter().enumerate() {
        let x = index as f32 * 60.0;
        children.extend(texture(
            format!("PetBattle{side}Aura{index}"),
            aura.icon,
            (x + 15.0, 90.0, 30.0, 30.0),
            WHITE,
        ));
        children.extend(label(
            format!("PetBattle{side}Aura{index}Duration"),
            &aura.rounds_remaining.to_string(),
            (x, 122.0, 60.0, 17.0),
            (11.0, WHITE, "CENTER"),
        ));
    }
    rsx! { r#frame { name: {DynName(format!("PetBattleActive{side}"))}, width: 270.0, height: 80.0, left, top, pos_type: "absolute", mouse_enabled: false, {children} } }
}
fn quality_color(quality: u8) -> &'static str {
    match quality {
        0 => "0.62,0.62,0.62,1",
        1 => WHITE,
        2 => "0.12,1,0,1",
        3 => "0,0.44,0.87,1",
        4 => "0.64,0.21,0.93,1",
        _ => "1,0.5,0,1",
    }
}
fn reserve_frames(side: &str, pets: &[BattlePetSnapshot], active: u8, left: f32) -> Element {
    let mut out = vec![];
    for (index, (_, pet)) in pets
        .iter()
        .enumerate()
        .filter(|(slot, _)| *slot != usize::from(active))
        .enumerate()
    {
        let name = format!("PetBattle{side}Reserve{}", index + 1);
        let top = 2.0 + index as f32 * 43.0;
        let mut children = texture(
            format!("{name}Icon"),
            pet.icon,
            (0.0, 0.0, 38.0, 38.0),
            WHITE,
        );
        children.extend(texture(
            format!("{name}Health"),
            WHITE_PIXEL,
            (
                2.0,
                30.0,
                35.0 * pet.health.max(0) as f32 / pet.max_health as f32,
                7.0,
            ),
            "0,1,0,1",
        ));
        children.extend(cropped(
            format!("{name}Border"),
            HUD_TEXTURE,
            if pet.health > 0 {
                "0.82519531,0.87109375,0.88085938,0.97265625"
            } else {
                "0.77734375,0.82324219,0.88085938,0.97265625"
            },
            (-4.5, -4.5, 47.0, 47.0),
        ));
        out.extend(rsx! { r#frame { name: {DynName(name)}, width: 38.0, height: 38.0, left, top, pos_type: "absolute", mouse_enabled: false, {children} } });
    }
    out
}
fn icon_button(name: String, action: &str, enabled: bool, rect: (f32, f32, f32, f32)) -> Element {
    let (left, top, width, height) = rect;
    let disabled = !enabled;
    rsx! { button { name: {DynName(name)}, width, height, text: "", onclick: action, disabled, pos_type: "absolute", left, top } }
}
fn action_bar(view: &WildBattleView, state: &WildPetBattleSnapshot) -> Element {
    let [width, height] = view.viewport;
    // Six 52px actions, a 15px delimiter, six 10px gaps and 260px chrome.
    let left = width / 2.0 - 323.5;
    let top = height - 69.0;
    let mut out = cropped(
        "PetBattleBottomArt".into(),
        HORIZONTAL_TILE,
        "0,1,0.00390625,0.48828125",
        (left, height - 124.0, 647.0, 124.0),
    );
    out.extend(cropped(
        "PetBattleLeftEndCap".into(),
        HUD_TEXTURE,
        "0.90136719,0.77734375,0.42578125,0.66992188",
        (left - 55.0, height - 124.0, 127.0, 125.0),
    ));
    out.extend(cropped(
        "PetBattleRightEndCap".into(),
        HUD_TEXTURE,
        "0.77734375,0.90136719,0.42578125,0.66992188",
        (left + 575.0, height - 124.0, 127.0, 125.0),
    ));
    let pet = &state.teams[0][usize::from(state.active[0])];
    let xp_left = width / 2.0 - 252.0;
    out.extend(texture(
        "PetBattleXPBar".into(),
        WHITE_PIXEL,
        (xp_left, height - 117.0, 504.0, 11.0),
        "0,0,0,0.6",
    ));
    let xp_width = if pet.level == 25 {
        504.0
    } else {
        504.0 * pet.xp as f32 / pet.next_level_xp as f32
    };
    out.extend(texture(
        "PetBattleXPFill".into(),
        WHITE_PIXEL,
        (xp_left, height - 117.0, xp_width, 11.0),
        "0.45,0.45,1,1",
    ));
    out.extend(cropped(
        "PetBattleXPLeft".into(),
        386851,
        "0.1875,0.4375,0.015625,0.265625",
        (xp_left - 3.0, height - 118.5, 14.0, 14.0),
    ));
    out.extend(cropped(
        "PetBattleXPRight".into(),
        386851,
        "0.1875,0.4375,0.296875,0.546875",
        (xp_left + 493.0, height - 118.5, 14.0, 14.0),
    ));
    out.extend(texture(
        "PetBattleXPMiddle".into(),
        386852,
        (xp_left + 11.0, height - 118.5, 482.0, 14.0),
        WHITE,
    ));
    for index in 1..20 {
        out.extend(cropped(
            format!("PetBattleXPDivision{index}"),
            386851,
            "0.015625,0.15625,0.015625,0.171875",
            (
                xp_left + 504.0 * index as f32 / 20.0 - 4.5,
                height - 115.0,
                9.0,
                9.0,
            ),
        ));
    }
    let enabled = !view.pending && view.result.is_none();
    for (index, ability) in pet.abilities.iter().enumerate() {
        let number = index + 1;
        let x = left + 35.0 + index as f32 * 62.0;
        out.extend(icon_button(
            format!("PetBattleAbility{number}"),
            &format!("pb:ability:{number}"),
            enabled && ability.usable && !state.replacement_required,
            (x, top, 52.0, 52.0),
        ));
        if ability.id != 0 {
            out.extend(texture(
                format!("PetBattleAbility{number}Icon"),
                ability.icon,
                (x, top, 52.0, 52.0),
                WHITE,
            ));
        }
        out.extend(texture(
            format!("PetBattleAbility{number}Border"),
            130841,
            (x - 18.0, top - 18.0, 88.0, 88.0),
            WHITE,
        ));
        out.extend(label(
            format!("PetBattleAbility{number}HotKey"),
            &number.to_string(),
            (x + 15.0, top + 3.0, 36.0, 10.0),
            (10.0, WHITE, "RIGHT"),
        ));
        if ability.id == 0 {
            out.extend(texture(
                format!("PetBattleAbility{number}Lock"),
                648430,
                (x + 10.0, top + 10.0, 32.0, 32.0),
                WHITE,
            ));
        } else {
            let enemy = &state.teams[1][usize::from(state.active[1])];
            if let Some(badge) = effectiveness_badge(ability.family, enemy.family) {
                out.extend(texture(
                    format!("PetBattleAbility{number}Effectiveness"),
                    badge,
                    (x + 29.0, top + 29.0, 32.0, 32.0),
                    WHITE,
                ));
            }
        }
        if ability.cooldown > 0 {
            out.extend(cropped(
                format!("PetBattleAbility{number}CooldownShadow"),
                HUD_TEXTURE,
                "0.71679688,0.76855469,0.85351563,0.95507813",
                (x, top, 53.0, 52.0),
            ));
            out.extend(label(
                format!("PetBattleAbility{number}Cooldown"),
                &ability.cooldown.to_string(),
                (x, top, 52.0, 52.0),
                (24.0, WHITE, "CENTER"),
            ));
        }
    }
    for (offset, name, icon, action, usable) in [
        (221.0, "Swap", 638663, "pb:swap", true),
        (308.0, "Trap", 638662, "pb:trap", state.can_trap),
        (370.0, "Forfeit", 638661, "pb:forfeit", true),
    ] {
        let x = left + offset;
        out.extend(icon_button(
            format!("PetBattle{name}"),
            action,
            enabled && usable,
            (x, top, 52.0, 52.0),
        ));
        out.extend(texture(
            format!("PetBattle{name}Icon"),
            icon,
            (x, top, 52.0, 52.0),
            if enabled && usable {
                WHITE
            } else {
                "0.35,0.35,0.35,1"
            },
        ));
        out.extend(texture(
            format!("PetBattle{name}Border"),
            130841,
            (x - 18.0, top - 18.0, 88.0, 88.0),
            WHITE,
        ));
    }
    out.extend(texture(
        "PetBattlePassFrame".into(),
        630622,
        (width / 2.0 - 64.0, height - 127.0, 128.0, 44.0),
        WHITE,
    ));
    out.extend(panel_button(
        "PetBattlePass".into(),
        "Pass",
        "pb:pass",
        enabled && !state.replacement_required,
        (width / 2.0 - 40.0, height - 116.0, 80.0, 22.0),
    ));
    if state.turn_time_ms != 0 {
        out.extend(cropped(
            "PetBattleTurnTimer".into(),
            HUD_TEXTURE,
            "0.00097656,0.46289063,0.23632813,0.28906250",
            (left + 13.5, height - 118.5, 473.0, 27.0),
        ));
        out.extend(label(
            "PetBattleTurnTimerText".into(),
            &state.turn_time_ms.div_ceil(1000).to_string(),
            (left + 108.0, height - 113.5, 339.0, 17.0),
            (14.0, WHITE, "CENTER"),
        ));
    }
    out
}
/// Retail GetAttackModifier: non-neutral badges use ability family, not caster family.
fn effectiveness_badge(attack: u8, defender: u8) -> Option<u32> {
    const STRONG: [u8; 10] = [1, 5, 8, 0, 3, 2, 9, 4, 6, 7];
    const WEAK: [u8; 10] = [7, 3, 1, 8, 0, 9, 4, 2, 5, 6];
    if STRONG[usize::from(attack)] == defender {
        Some(608706)
    } else if WEAK[usize::from(attack)] == defender {
        Some(608707)
    } else {
        None
    }
}
fn swap_panel(view: &WildBattleView, state: &WildPetBattleSnapshot) -> Element {
    let mut out = Vec::new();
    for (index, pet) in state.teams[0].iter().enumerate() {
        let enabled = pet.health > 0 && index != usize::from(state.active[0]) && !view.pending;
        out.extend(panel_button(
            format!("PetBattleSwapPet{}", index + 1),
            &format!("{} {} / {}", pet.name, pet.health, pet.max_health),
            &format!("pb:swap:{}", index + 1),
            enabled,
            (
                view.viewport[0] / 2.0 - 285.0 + index as f32 * 192.0,
                view.viewport[1] - 215.0,
                182.0,
                80.0,
            ),
        ));
    }
    out
}
fn forfeit_popup(width: f32, height: f32) -> Element {
    let mut out = label(
        "PetBattleForfeitPrompt".into(),
        "Forfeit this pet battle?",
        (width / 2.0 - 160.0, height / 2.0 - 65.0, 320.0, 32.0),
        (18.0, WHITE, "CENTER"),
    );
    out.extend(panel_button(
        "PetBattleForfeitConfirm".into(),
        "Forfeit",
        "pb:confirm-forfeit",
        true,
        (width / 2.0 - 145.0, height / 2.0 - 20.0, 140.0, 28.0),
    ));
    out.extend(panel_button(
        "PetBattleForfeitCancel".into(),
        "Cancel",
        "pb:cancel-forfeit",
        true,
        (width / 2.0 + 5.0, height / 2.0 - 20.0, 140.0, 28.0),
    ));
    out
}
