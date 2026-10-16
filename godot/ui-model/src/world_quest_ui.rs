//! Retail WorldQuestDataProvider / WorldQuestObjectiveTracker presentation.
use crate::quest_runtime::QuestRuntime;
use crate::tooltip_presentation::{TooltipLineState, TooltipPresentation};
use crate::ui::screens::world_map_frame_component::{MapPin, MapPinType};
use crate::ui_map_data::UiMapCatalog;
use shared::protocol::{AvailableWorldQuest, WorldQuestPoi};
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

struct DynName(String);

pub const SELECT_PREFIX: &str = "world_quest:select:";
pub const TIME_CRITICAL_SECONDS: i64 = 15 * 60;

pub fn map_pins(
    catalog: &UiMapCatalog,
    map_id: u32,
    runtime: &QuestRuntime,
    now: i64,
) -> Vec<MapPin> {
    runtime
        .active_world_quests(now)
        .filter_map(|quest| {
            let [x, y] = quest
                .pois
                .iter()
                .filter(|poi| catalog.lineage(poi.ui_map_id).contains(&map_id))
                .find_map(|poi| {
                    catalog.map_position(
                        map_id,
                        poi.map_id,
                        [poi.x as f32, poi.y as f32, poi.z as f32],
                    )
                })?;
            Some(MapPin {
                pin_type: MapPinType::WorldQuest,
                quest_id: Some(quest.quest_id),
                label: quest.title.clone(),
                badge: String::new(),
                x,
                y,
            })
        })
        .collect()
}

/// Received active task positions, without accepting a task or inventing quest-log POIs.
/// Minimap icon composition is an engine presentation, not a Retail Lua minimap oracle.
pub fn minimap_blips(
    runtime: &QuestRuntime,
    selected: Option<u32>,
    map_id: u32,
    view: &game_engine_core::minimap_data::MinimapView,
    map_size: f32,
    now: i64,
) -> Vec<crate::minimap::MinimapBlip> {
    use crate::minimap::{BlipKind, MinimapBlip, edge_offset};
    runtime
        .active_world_quests(now)
        .filter_map(|quest| {
            let poi = find_nearest_poi(quest, map_id, view.center)?;
            let position = [poi.x as f32, -(poi.y as f32)];
            let (offset, edge) = edge_offset(view, position, map_size);
            if edge && selected != Some(quest.quest_id) {
                return None;
            }
            Some(MinimapBlip {
                unit: u64::from(quest.quest_id),
                kind: if edge {
                    BlipKind::QuestSuperTrackArrow
                } else {
                    BlipKind::WorldQuest
                },
                offset,
            })
        })
        .collect()
}

/// Minimap navigation chooses the nearest authored area in the horizontal view.
/// `center` is engine [x,z], not the source height axis retained in WorldQuestPoi.
fn find_nearest_poi(
    quest: &AvailableWorldQuest,
    map_id: u32,
    center: [f32; 2],
) -> Option<&WorldQuestPoi> {
    quest
        .pois
        .iter()
        .filter(|poi| poi.map_id == map_id)
        .min_by(|first, second| {
            poi_distance_squared(first, center).total_cmp(&poi_distance_squared(second, center))
        })
}

fn poi_distance_squared(poi: &WorldQuestPoi, center: [f32; 2]) -> f32 {
    let dx = poi.x as f32 - center[0];
    let dz = -(poi.y as f32) - center[1];
    dx * dx + dz * dz
}

pub fn pin_at(pins: &[MapPin], uv: [f32; 2], [width, height]: [f32; 2]) -> Option<u32> {
    const HIT_RADIUS: f32 = 16.;
    pins.iter()
        .find(|pin| {
            pin.pin_type == MapPinType::WorldQuest
                && ((pin.x - uv[0]) * width).abs() < HIT_RADIUS
                && ((pin.y - uv[1]) * height).abs() < HIT_RADIUS
        })
        .and_then(|pin| pin.quest_id)
}

pub fn tooltip(quest: &AvailableWorldQuest, now: i64, position: [f32; 2]) -> TooltipPresentation {
    let remaining = (quest.expires_at - now).max(0);
    let color = if remaining <= TIME_CRITICAL_SECONDS {
        [1., 0.1, 0.1, 1.]
    } else {
        [1.; 4]
    };
    let time = format!("Time Left: {}", time_left(remaining));
    let mut lines = vec![TooltipLineState::colored(time, color)];
    let has_rewards = quest.xp > 0
        || quest.rewards.money > 0
        || !quest.rewards.items.is_empty()
        || !quest.rewards.choice_items.is_empty()
        || !quest.currencies.is_empty()
        || !quest.reputation.is_empty();
    if has_rewards {
        lines.push(TooltipLineState::new(""));
        lines.push(TooltipLineState::new("Rewards"));
    }
    if quest.xp > 0 {
        lines.push(TooltipLineState::new(format!("{} XP", quest.xp)));
    }
    if quest.rewards.money > 0 {
        lines.push(TooltipLineState::money(
            "Reward",
            u64::from(quest.rewards.money),
        ));
    }
    lines.extend(
        quest
            .rewards
            .items
            .iter()
            .chain(&quest.rewards.choice_items)
            .map(|item| TooltipLineState::new(format!("{} x{}", item.name, item.count))),
    );
    lines.extend(
        quest.currencies.iter().map(|reward| {
            TooltipLineState::new(format!("Currency {}: {}", reward.id, reward.amount))
        }),
    );
    lines.extend(quest.reputation.iter().map(|reward| {
        TooltipLineState::new(format!("Reputation {}: {:+}", reward.id, reward.amount))
    }));
    TooltipPresentation {
        visible: true,
        x: position[0],
        y: position[1],
        title: quest.title.clone(),
        lines,
        ..Default::default()
    }
}

/// QuestUtils WorldQuestsSecondsFormatter uses one full unit at a day or less.
fn time_left(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    if seconds < MINUTE {
        return "< 1 Minute".into();
    }
    let unit_count = if seconds > DAY { 2 } else { 1 };
    let mut remaining = seconds;
    [(DAY, "Day"), (HOUR, "Hour"), (MINUTE, "Minute")]
        .into_iter()
        .filter_map(|(unit, name)| {
            let count = remaining / unit;
            remaining %= unit;
            let plural = if count == 1 { "" } else { "s" };
            (count > 0).then(|| format!("{count} {name}{plural}"))
        })
        .take(unit_count)
        .collect::<Vec<_>>()
        .join(" ")
}

/// BonusTrackerProgressBarTemplate: 191x17 fill, 192x38 block.
pub fn progress_bar(id: u32, progress: (u32, u32), y: f32, scale: f32) -> Element {
    let (current, total) = progress;
    let fraction = if total == 0 {
        0.
    } else {
        (current as f32 / total as f32).clamp(0., 1.)
    };
    let text = format!("{}%", (fraction * 100.).round() as u32);
    let fill = 191. * fraction * scale;
    let mut elements = rsx! {
        r#frame {name: {DynName(format!("WorldQuest{id}ProgressBackground"))},width: {191.*scale},height:{17.*scale},background_color:"0.04,0.07,0.18,1",pos_type:"absolute",left:{30.*scale},top:{y*scale}}
        texture {name: {DynName(format!("WorldQuest{id}ProgressFill"))},texture_fdid:137012u32,width:{fill},height:{17.*scale},hidden:{fill <= 0.},vertex_color:"0.26,0.42,1,1",pos_type:"absolute",left:{30.*scale},top:{y*scale}}
    };
    elements.extend(crate::quest_art::named_atlas_texture(
        format!("WorldQuest{id}ProgressFrame"),
        "bonusobjectives-bar-frame-5",
        (22. * scale, (y - 17.) * scale, 240. * scale, 51. * scale),
    ));
    elements.extend(rsx! {fontstring {name: {DynName(format!("WorldQuest{id}ProgressLabel"))},text:{text.as_str()},font:GameFont::FrizQuadrata,font_size:{12.*scale},font_color:"1,1,1,1",justify_h:"CENTER",width:{191.*scale},height:{17.*scale},pos_type:"absolute",left:{30.*scale},top:{y*scale}}});
    elements
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldQuestToast {
    pub title: String,
    pub completed: bool,
}

pub fn toast_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<WorldQuestToast>()
        .expect("world quest toast state");
    let label = if state.completed {
        "World Quest Complete!"
    } else {
        "World Quest"
    };
    let mut content = crate::quest_art::named_atlas_texture(
        "WorldQuestToastBackground".into(),
        "bonusobjectives-title-bg",
        (0., 0., 400., 60.),
    );
    content.extend(rsx! {
        fontstring {name:"WorldQuestToastStatus",text:label,font:GameFont::FrizQuadrata,font_size:16.,font_color:"1,0.82,0,1",justify_h:"CENTER",width:400.,height:24.,pos_type:"absolute",left:0.,top:2.}
        fontstring {name:"WorldQuestToastTitle",text:{state.title.as_str()},font:GameFont::FrizQuadrata,font_size:14.,font_color:"1,1,1,1",justify_h:"CENTER",width:400.,height:24.,pos_type:"absolute",left:0.,top:28.}
    });
    rsx! {r#frame {name:"WorldQuestToast",width:400.,height:60.,pos_type:"absolute",left:"50%",margin_left:-200.,top:"20%",{content}}}
}
