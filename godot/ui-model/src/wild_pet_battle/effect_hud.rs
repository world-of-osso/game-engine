//! Retail Shared PetBattleUI.xml: weather725, aura21, pad/pet holders789–913.
use super::*;

pub(super) fn effects(state: &WildPetBattleSnapshot, width: f32) -> Element {
    let mut out = vec![];
    if let Some(weather) = &state.weather {
        out.extend(weather_frame(weather, width));
    }
    for (team, side) in ["Ally", "Enemy"].iter().enumerate() {
        let pet = &state.teams[team][usize::from(state.active[team])];
        out.extend(aura_holders(
            &format!("PetBattle{side}"),
            &pet.auras,
            if team == 0 { 70.0 } else { width - 292.0 },
            team == 1,
            3,
        ));
        out.extend(aura_holders(
            &format!("PetBattle{side}Pad"),
            &state.team_auras[team],
            if team == 0 { 178.0 } else { width - 400.0 },
            team == 0,
            2,
        ));
    }
    out
}

fn weather_frame(aura: &BattleAuraSnapshot, width: f32) -> Element {
    let left = width / 2.0 - 70.0;
    let top = 60.0;
    let mut children = vec![];
    let background = match aura.ability_id {
        590 => Some(615343),
        205 => Some(611370),
        171 => Some(611371),
        257 => Some(611372),
        203 => Some(603599),
        596 => Some(611373),
        718 => Some(611374),
        229 | 235 => Some(611375),
        454 => Some(611376),
        403 => Some(611377),
        2350 => Some(3028322),
        _ => None,
    };
    if let Some(fdid) = background {
        children.extend(texture(
            "PetBattleWeatherArt".into(),
            fdid,
            (-171.0, -64.0, 512.0, 128.0),
            WHITE,
        ));
    }
    children.extend(texture(
        "PetBattleWeatherIcon".into(),
        aura.icon,
        (0.0, 0.0, 32.0, 32.0),
        WHITE,
    ));
    children.extend(label(
        "PetBattleWeatherLabel".into(),
        "Weather",
        (32.0, 0.0, 138.0, 18.0),
        (16.0, "1,0.82,0,1", "LEFT"),
    ));
    children.extend(label(
        "PetBattleWeatherName".into(),
        &aura.name,
        (32.0, 18.0, 138.0, 22.0),
        (13.0, WHITE, "LEFT"),
    ));
    children.extend(cropped(
        "PetBattleWeatherDurationShadow".into(),
        HUD_TEXTURE,
        "0.71679688,0.76855469,0.85351563,0.95507813",
        (0.0, 0.0, 32.0, 32.0),
    ));
    children.extend(label(
        "PetBattleWeatherDuration".into(),
        &duration(aura, false),
        (0.0, 0.0, 32.0, 32.0),
        (24.0, "1,0.82,0,1", "CENTER"),
    ));
    rsx! { r#frame { name: "PetBattleWeatherFrame", width: 170.0, height: 40.0, left, top, pos_type: "absolute", mouse_enabled: false, {children} } }
}

fn duration(aura: &BattleAuraSnapshot, turns: bool) -> String {
    match (aura.rounds_remaining, turns) {
        (remaining, _) if remaining < 0 => String::new(),
        (remaining, true) => format!("{remaining} rounds"),
        (remaining, false) => remaining.to_string(),
    }
}

fn aura_holders(
    prefix: &str,
    auras: &[BattleAuraSnapshot],
    left: f32,
    grows_left: bool,
    per_row: usize,
) -> Element {
    let mut out = vec![];
    let mut top = 138.0;
    for (buff, kind) in [(true, "Buff"), (false, "Debuff")] {
        let matching: Vec<_> = auras.iter().filter(|a| a.is_buff == buff).collect();
        for (index, aura) in matching.iter().enumerate() {
            let column = (index % per_row) as f32 * 64.0;
            let x = if grows_left { 162.0 - column } else { column };
            let y = (index / per_row) as f32 * 49.0;
            let name = format!("{prefix}{kind}{index}");
            out.extend(texture(
                name.clone(),
                aura.icon,
                (left + x + 15.0, top + y, 30.0, 30.0),
                WHITE,
            ));
            if !buff {
                out.extend(rsx! { texture { name: {DynName(format!("{name}Border"))}, texture_fdid: 130759u32, width: 33.0, height: 32.0, left: {left+x+13.5}, top: {top+y-1.0}, pos_type: "absolute", tex_coords: "0.296875,0.5703125,0,0.515625", vertex_color: "1,0,0,1" } });
            }
            out.extend(label(
                format!("{name}Duration"),
                &duration(aura, true),
                (left + x, top + y + 32.0, 60.0, 17.0),
                (11.0, WHITE, "CENTER"),
            ));
        }
        top += if matching.is_empty() {
            1.0
        } else {
            matching.len().div_ceil(per_row) as f32 * 49.0
        };
    }
    out
}

// Retail PetBattleFrame_OnShow opens a dedicated temporary PET_BATTLE_COMBAT_LOG chat window.
pub(super) fn combat_log(view: &WildBattleView) -> Element {
    let top = view.viewport[1] - 310.0;
    let last = view.combat_text.len().saturating_sub(view.log_scroll);
    let first = last.saturating_sub(7);
    let mut children = cropped(
        "PetBattleCombatLogBackground".into(),
        HUD_TEXTURE,
        "0.56347656,0.86035156,0.03906250,0.27929688",
        (0.0, 0.0, 400.0, 150.0),
    );
    children.extend(label(
        "PetBattleCombatLogTab".into(),
        "Pet Battle",
        (5.0, 0.0, 350.0, 20.0),
        (13.0, "1,0.82,0,1", "LEFT"),
    ));
    for (index, line) in view.combat_text[first..last].iter().enumerate() {
        children.extend(label(
            format!("PetBattleCombatLogLine{index}"),
            line,
            (8.0, 22.0 + index as f32 * 17.0, 366.0, 17.0),
            (12.0, WHITE, "LEFT"),
        ));
    }
    children.extend(panel_button(
        "PetBattleCombatLogUp".into(),
        "▲",
        "pb:log-up",
        first > 0,
        (376.0, 22.0, 24.0, 24.0),
    ));
    children.extend(panel_button(
        "PetBattleCombatLogDown".into(),
        "▼",
        "pb:log-down",
        view.log_scroll > 0,
        (376.0, 110.0, 24.0, 24.0),
    ));
    rsx! { r#frame { name: "PetBattleCombatLog", width: 400.0, height: 150.0, left: 20.0, top, pos_type: "absolute", mouse_enabled: false, {children} } }
}
