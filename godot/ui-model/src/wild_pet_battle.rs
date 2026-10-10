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
            WildPetBattleUpdate::State(state) => self.state = Some(state),
            WildPetBattleUpdate::Round { state, combat_text } => {
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
    let mut children = unit_frame(
        "Ally",
        &state.teams[0][usize::from(state.active[0])],
        45.0,
        25.0,
    );
    children.extend(unit_frame(
        "Enemy",
        &state.teams[1][usize::from(state.active[1])],
        width - 315.0,
        25.0,
    ));
    children.extend(action_bar(view, state));
    children.extend(label(
        "PetBattleCombatText".into(),
        &view
            .combat_text
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
        (width / 2.0 - 230.0, height - 290.0, 460.0, 130.0),
        (16.0, WHITE, "CENTER"),
    ));
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
    let mut children = cropped(
        format!("PetBattle{side}HealthFrame"),
        HUD_TEXTURE,
        "0.3984375,0.54980469,0.43359375,0.52539063",
        (66.0, 28.0, 155.0, 47.0),
    );
    children.extend(label(
        format!("PetBattle{side}Name"),
        &pet.name,
        (35.0, 0.0, 235.0, 24.0),
        (14.0, WHITE, "LEFT"),
    ));
    let family = PetSpeciesVisual {
        species_id: pet.species_id,
        display_id: pet.display_id,
        family: pet.family,
    };
    children.extend(texture(
        format!("PetBattle{side}Type"),
        family.type_icon(),
        (0.0, 28.0, 34.0, 34.0),
        WHITE,
    ));
    children.extend(label(
        format!("PetBattle{side}Level"),
        &pet.level.to_string(),
        (36.0, 34.0, 25.0, 24.0),
        (16.0, WHITE, "CENTER"),
    ));
    let health_width = 145.0 * (pet.health.max(0) as f32 / pet.max_health as f32);
    children.extend(texture(
        format!("PetBattle{side}HealthFill"),
        WHITE_PIXEL,
        (71.0, 39.0, health_width, 21.0),
        "0.15,0.75,0.2,1",
    ));
    children.extend(label(
        format!("PetBattle{side}Health"),
        &format!("{} / {}", pet.health, pet.max_health),
        (71.0, 39.0, 145.0, 21.0),
        (12.0, WHITE, "CENTER"),
    ));
    for (index, aura) in pet.auras.iter().enumerate() {
        children.extend(label(
            format!("PetBattle{side}Aura{index}"),
            &format!("{} ({} rounds)", aura.ability_id, aura.rounds_remaining),
            (index as f32 * 90.0, 82.0, 90.0, 49.0),
            (11.0, WHITE, "CENTER"),
        ));
    }
    let name = DynName(format!("PetBattleActive{side}"));
    rsx! { r#frame { name: {name}, width: 270.0, height: 80.0, left, top, pos_type: "absolute", mouse_enabled: false, {children} } }
}
fn action_bar(view: &WildBattleView, state: &WildPetBattleSnapshot) -> Element {
    let [width, height] = view.viewport;
    let left = width / 2.0 - 250.0;
    let top = height - 100.0;
    let mut out = cropped(
        "PetBattleBottomArt".into(),
        HORIZONTAL_TILE,
        "0,1,0.00390625,0.48828125",
        (left, top - 24.0, 500.0, 124.0),
    );
    let pet = &state.teams[0][usize::from(state.active[0])];
    let enabled = !view.pending && view.result.is_none();
    for (index, ability) in pet.abilities.iter().enumerate() {
        let number = index + 1;
        let x = left + 27.0 + index as f32 * 64.0;
        out.extend(panel_button(
            format!("PetBattleAbility{number}"),
            "",
            &format!("pb:ability:{number}"),
            enabled && ability.usable && !state.replacement_required,
            (x, top, 56.0, 56.0),
        ));
        if ability.id != 0 {
            out.extend(texture(
                format!("PetBattleAbility{number}Icon"),
                ability.icon,
                (x + 7.0, top + 7.0, 42.0, 42.0),
                WHITE,
            ));
        }
        out.extend(label(
            format!("PetBattleAbility{number}Label"),
            &format!("{number}: {}", ability.name),
            (x - 4.0, top + 58.0, 64.0, 18.0),
            (10.0, WHITE, "CENTER"),
        ));
        if ability.cooldown > 0 {
            out.extend(label(
                format!("PetBattleAbility{number}Cooldown"),
                &ability.cooldown.to_string(),
                (x, top, 56.0, 56.0),
                (24.0, WHITE, "CENTER"),
            ));
        }
    }
    for (index, name, text, action, usable) in [
        (0, "Swap", "Swap", "pb:swap", true),
        (1, "Trap", "Trap", "pb:trap", state.can_trap),
        (2, "Forfeit", "Forfeit", "pb:forfeit", true),
    ] {
        out.extend(panel_button(
            format!("PetBattle{name}"),
            text,
            action,
            enabled && usable,
            (left + 225.0 + index as f32 * 78.0, top, 72.0, 56.0),
        ));
    }
    out.extend(panel_button(
        "PetBattlePass".into(),
        "Pass",
        "pb:pass",
        enabled && !state.replacement_required,
        (left + 395.0, top - 48.0, 72.0, 24.0),
    ));
    let timer = if state.turn_time_ms == 0 {
        "Select an action".to_string()
    } else {
        format!("{}", state.turn_time_ms.div_ceil(1000))
    };
    out.extend(label(
        "PetBattleTurnTimerText".into(),
        &timer,
        (left + 70.0, top - 48.0, 320.0, 24.0),
        (14.0, WHITE, "CENTER"),
    ));
    out
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
