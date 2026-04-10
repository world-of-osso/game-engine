use bevy::prelude::*;
use game_engine::bag_data::{InventorySlot, InventoryState, ItemQuality};
use game_engine::buff_data::{AuraInstance, AuraState, UnitAuraState};
use game_engine::targeting::CurrentTarget;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::spellbook_data::SpellbookSpell;
use game_engine::ui::spellbook_runtime::SpellbookUiRuntime;
use ui_toolkit::rsx;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

use crate::client_options::GraphicsOptions;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;

const TOOLTIP_W: f32 = 260.0;
const TOOLTIP_MIN_H: f32 = 34.0;
const TOOLTIP_INSET: f32 = 8.0;
const TOOLTIP_TITLE_H: f32 = 16.0;
const TOOLTIP_LINE_H: f32 = 14.0;
const TOOLTIP_CURSOR_X: f32 = 18.0;
const TOOLTIP_CURSOR_Y: f32 = 24.0;
const TOOLTIP_MARGIN: f32 = 8.0;

const TOOLTIP_BG: &str = "0.03,0.02,0.01,0.96";
const TOOLTIP_BORDER: &str = "1px solid 0.66,0.54,0.22,0.95";
const TOOLTIP_TEXT_COLOR: [f32; 4] = [0.92, 0.89, 0.82, 1.0];
const TOOLTIP_LABEL_COLOR: [f32; 4] = [0.72, 0.72, 0.72, 1.0];
const TOOLTIP_BUFF_COLOR: [f32; 4] = [1.0, 0.82, 0.32, 1.0];
const TOOLTIP_SPELL_COLOR: [f32; 4] = [0.98, 0.88, 0.54, 1.0];

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TooltipLineState {
    left_text: String,
    right_text: String,
    left_color: [f32; 4],
    right_color: [f32; 4],
}

impl TooltipLineState {
    fn new(text: impl Into<String>) -> Self {
        Self {
            left_text: text.into(),
            right_text: String::new(),
            left_color: TOOLTIP_TEXT_COLOR,
            right_color: TOOLTIP_TEXT_COLOR,
        }
    }

    fn key_value(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            left_text: label.into(),
            right_text: value.into(),
            left_color: TOOLTIP_LABEL_COLOR,
            right_color: TOOLTIP_TEXT_COLOR,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
struct TooltipFrameState {
    visible: bool,
    x: f32,
    y: f32,
    title: String,
    title_color: [f32; 4],
    lines: Vec<TooltipLineState>,
}

impl TooltipFrameState {
    fn hidden() -> Self {
        Self {
            visible: false,
            x: 0.0,
            y: 0.0,
            title: String::new(),
            title_color: TOOLTIP_TEXT_COLOR,
            lines: Vec::new(),
        }
    }

    fn height(&self) -> f32 {
        let lines_h = self.lines.len() as f32 * TOOLTIP_LINE_H;
        (2.0 * TOOLTIP_INSET + TOOLTIP_TITLE_H + lines_h).max(TOOLTIP_MIN_H)
    }
}

struct TooltipFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for TooltipFrameRes {}
unsafe impl Sync for TooltipFrameRes {}

#[derive(Resource)]
struct TooltipFrameWrap(TooltipFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct TooltipFrameModel(TooltipFrameState);

pub struct TooltipFramePlugin;

impl Plugin for TooltipFramePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_tooltip_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_tooltip_frame_ui);
        app.add_systems(
            Update,
            (sync_tooltip_root_size, sync_tooltip_frame_state).run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_tooltip_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = TooltipFrameState::hidden();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(tooltip_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(TooltipFrameWrap(TooltipFrameRes { screen, shared }));
    commands.insert_resource(TooltipFrameModel(state));
}

fn teardown_tooltip_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<TooltipFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<TooltipFrameWrap>();
    commands.remove_resource::<TooltipFrameModel>();
}

fn sync_tooltip_root_size(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
}

fn sync_tooltip_frame_state(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<TooltipFrameWrap>>,
    mut last_model: Option<ResMut<TooltipFrameModel>>,
    inventory: Res<InventoryState>,
    current_target: Res<CurrentTarget>,
    local_player: Query<Entity, With<LocalPlayer>>,
    target_auras: Query<&UnitAuraState>,
    aura_state: Option<Res<AuraState>>,
    graphics_options: Option<Res<GraphicsOptions>>,
    spellbook_runtime: Option<NonSend<SpellbookUiRuntime>>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let state = build_state(
        &ui.registry,
        window,
        &inventory,
        &current_target,
        local_player.iter().next(),
        &target_auras,
        aura_state.as_deref(),
        graphics_options.as_deref(),
        spellbook_runtime.as_deref(),
    );
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    registry: &FrameRegistry,
    window: &Window,
    inventory: &InventoryState,
    current_target: &CurrentTarget,
    local_player: Option<Entity>,
    target_auras: &Query<&UnitAuraState>,
    aura_state: Option<&AuraState>,
    graphics_options: Option<&GraphicsOptions>,
    spellbook_runtime: Option<&SpellbookUiRuntime>,
) -> TooltipFrameState {
    let Some(cursor) = window.cursor_position() else {
        return TooltipFrameState::hidden();
    };
    let Some(frame_id) = find_frame_at(registry, cursor.x, cursor.y) else {
        return TooltipFrameState::hidden();
    };
    let Some(content) = hovered_spell_tooltip(registry, frame_id, spellbook_runtime)
        .or_else(|| hovered_item_tooltip(registry, frame_id, inventory))
        .or_else(|| {
            hovered_target_aura_tooltip(
                registry,
                frame_id,
                current_target,
                local_player,
                target_auras,
                aura_state,
                graphics_options,
            )
        })
    else {
        return TooltipFrameState::hidden();
    };
    place_tooltip(content, cursor, window)
}

fn hovered_spell_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    spellbook_runtime: Option<&SpellbookUiRuntime>,
) -> Option<TooltipFrameState> {
    let spell = spellbook_runtime?.spell_for_frame(registry, frame_id)?;
    Some(spell_tooltip(spell))
}

fn hovered_item_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    inventory: &InventoryState,
) -> Option<TooltipFrameState> {
    let (bag_index, slot_index) = hovered_bag_slot(registry, frame_id)?;
    let slot = inventory.slot(bag_index, slot_index)?;
    (!slot.is_empty()).then(|| item_tooltip(slot))
}

fn hovered_target_aura_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    current_target: &CurrentTarget,
    local_player: Option<Entity>,
    target_auras: &Query<&UnitAuraState>,
    aura_state: Option<&AuraState>,
    graphics_options: Option<&GraphicsOptions>,
) -> Option<TooltipFrameState> {
    let hovered = hovered_target_aura(registry, frame_id)?;
    let aura = resolve_hovered_aura(
        hovered,
        current_target.0,
        local_player,
        target_auras,
        aura_state,
    )?;
    let colorblind_mode = graphics_options.is_some_and(|graphics| graphics.colorblind_mode);
    Some(aura_tooltip(aura, colorblind_mode))
}

fn place_tooltip(
    mut tooltip: TooltipFrameState,
    cursor: Vec2,
    window: &Window,
) -> TooltipFrameState {
    let max_x = (window.width() - TOOLTIP_W - TOOLTIP_MARGIN).max(TOOLTIP_MARGIN);
    let max_y = (window.height() - tooltip.height() - TOOLTIP_MARGIN).max(TOOLTIP_MARGIN);
    tooltip.visible = true;
    tooltip.x = (cursor.x + TOOLTIP_CURSOR_X).min(max_x);
    tooltip.y = (cursor.y + TOOLTIP_CURSOR_Y).min(max_y);
    tooltip
}

fn item_tooltip(slot: &InventorySlot) -> TooltipFrameState {
    let mut lines = vec![TooltipLineState::key_value(
        "Quality",
        item_quality_label(slot.quality),
    )];
    if slot.count > 1 {
        lines.push(TooltipLineState::key_value(
            "Stack Count",
            slot.count.to_string(),
        ));
    }
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: slot.name.clone(),
        title_color: parse_rgba(slot.quality.border_color()),
        lines,
    }
}

fn spell_tooltip(spell: SpellbookSpell) -> TooltipFrameState {
    let mut lines = vec![TooltipLineState::new(if spell.passive {
        "Passive ability"
    } else {
        "Active ability"
    })];
    if spell.cooldown_seconds > 0.0 {
        lines.push(TooltipLineState::key_value(
            "Cooldown",
            format_spell_duration(spell.cooldown_seconds),
        ));
    }
    lines.push(TooltipLineState::key_value(
        "Spell ID",
        spell.id.to_string(),
    ));
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: spell.name.to_string(),
        title_color: TOOLTIP_SPELL_COLOR,
        lines,
    }
}

fn aura_tooltip(aura: &AuraInstance, colorblind_mode: bool) -> TooltipFrameState {
    let mut lines = Vec::new();
    if !aura.description.is_empty() {
        lines.push(TooltipLineState::new(aura.description.clone()));
    }
    lines.push(TooltipLineState::key_value(
        "Duration",
        if aura.duration <= 0.0 {
            "Permanent".to_string()
        } else {
            aura.timer_text()
        },
    ));
    if aura.stacks > 1 {
        lines.push(TooltipLineState::key_value(
            "Stacks",
            aura.stacks.to_string(),
        ));
    }
    if !aura.source.is_empty() {
        lines.push(TooltipLineState::key_value("Source", aura.source.clone()));
    }
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: aura.name.clone(),
        title_color: if aura.is_debuff {
            parse_rgba(aura.debuff_type.border_color_for_mode(colorblind_mode))
        } else {
            TOOLTIP_BUFF_COLOR
        },
        lines,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HoveredAuraKind {
    Buff,
    Debuff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HoveredAura {
    kind: HoveredAuraKind,
    index: usize,
}

fn resolve_hovered_aura<'a>(
    hovered: HoveredAura,
    current_target: Option<Entity>,
    local_player: Option<Entity>,
    target_auras: &'a Query<&UnitAuraState>,
    aura_state: Option<&'a AuraState>,
) -> Option<&'a AuraInstance> {
    let local = current_target
        .zip(local_player)
        .is_some_and(|(target, local)| target == local);
    let auras = if local {
        aura_state
            .map(|state| state.auras.as_slice())
            .unwrap_or(&[])
    } else {
        target_auras
            .get(current_target?)
            .ok()
            .map(|state| state.auras.as_slice())
            .unwrap_or(&[])
    };
    match hovered.kind {
        HoveredAuraKind::Buff => auras
            .iter()
            .filter(|aura| !aura.is_debuff)
            .nth(hovered.index),
        HoveredAuraKind::Debuff => auras
            .iter()
            .filter(|aura| aura.is_debuff)
            .nth(hovered.index),
    }
}

fn hovered_bag_slot(registry: &FrameRegistry, mut frame_id: u64) -> Option<(usize, usize)> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(name) = frame.name.as_deref()
            && let Some(indices) = parse_bag_slot_name(name)
        {
            return Some(indices);
        }
        frame_id = frame.parent_id?;
    }
}

fn hovered_target_aura(registry: &FrameRegistry, mut frame_id: u64) -> Option<HoveredAura> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(name) = frame.name.as_deref()
            && let Some(hovered) = parse_target_aura_name(name)
        {
            return Some(hovered);
        }
        frame_id = frame.parent_id?;
    }
}

fn parse_bag_slot_name(name: &str) -> Option<(usize, usize)> {
    let rest = name.strip_prefix("ContainerFrame")?;
    let (bag_index, slot_index) = rest.split_once("Slot")?;
    Some((bag_index.parse().ok()?, slot_index.parse().ok()?))
}

fn parse_target_aura_name(name: &str) -> Option<HoveredAura> {
    if let Some(index) = parse_prefixed_index(name, "TargetBuffIcon") {
        return Some(HoveredAura {
            kind: HoveredAuraKind::Buff,
            index,
        });
    }
    parse_prefixed_index(name, "TargetDebuffIcon").map(|index| HoveredAura {
        kind: HoveredAuraKind::Debuff,
        index,
    })
}

fn parse_prefixed_index(name: &str, prefix: &str) -> Option<usize> {
    let rest = name.strip_prefix(prefix)?;
    let digits: String = rest.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

fn item_quality_label(quality: ItemQuality) -> &'static str {
    match quality {
        ItemQuality::Poor => "Poor",
        ItemQuality::Common => "Common",
        ItemQuality::Uncommon => "Uncommon",
        ItemQuality::Rare => "Rare",
        ItemQuality::Epic => "Epic",
        ItemQuality::Legendary => "Legendary",
    }
}

fn format_spell_duration(seconds: f32) -> String {
    let secs = seconds.round() as u32;
    if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

fn parse_rgba(input: &str) -> [f32; 4] {
    let values: Vec<f32> = input
        .split(',')
        .filter_map(|part| part.parse().ok())
        .collect();
    match values.as_slice() {
        [r, g, b, _a] => [*r, *g, *b, 1.0],
        [r, g, b] => [*r, *g, *b, 1.0],
        _ => TOOLTIP_TEXT_COLOR,
    }
}

fn tooltip_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<TooltipFrameState>()
        .expect("TooltipFrameState must be in SharedContext");
    let height = state.height();
    let title = tooltip_title(state);
    let lines = tooltip_lines(&state.lines);
    rsx! {
        r#frame {
            name: "TooltipFrame",
            width: {TOOLTIP_W},
            height: {height},
            hidden: {!state.visible},
            strata: "TOOLTIP",
            background_color: TOOLTIP_BG,
            border: TOOLTIP_BORDER,
            anchor {
                point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                relative_point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                x: {state.x},
                y: {-state.y},
            }
            {title}
            {lines}
        }
    }
}

fn tooltip_title(state: &TooltipFrameState) -> Element {
    rsx! {
        fontstring {
            name: "TooltipTitle",
            width: {TOOLTIP_W - 2.0 * TOOLTIP_INSET},
            height: {TOOLTIP_TITLE_H},
            text: {state.title.as_str()},
            font: "FrizQuadrata",
            font_size: 12.0,
            font_color: {rgba_string(state.title_color)},
            justify_h: "LEFT",
            anchor {
                point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                relative_point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                x: {TOOLTIP_INSET},
                y: {-TOOLTIP_INSET},
            }
        }
    }
}

fn tooltip_lines(lines: &[TooltipLineState]) -> Element {
    lines
        .iter()
        .enumerate()
        .flat_map(|(index, line)| tooltip_line(index, line))
        .collect()
}

fn tooltip_line(index: usize, line: &TooltipLineState) -> Element {
    let y = -(TOOLTIP_INSET + TOOLTIP_TITLE_H + index as f32 * TOOLTIP_LINE_H);
    rsx! {
        fontstring {
            name: {DynName(format!("TooltipLine{index}Left"))},
            width: {TOOLTIP_W - 2.0 * TOOLTIP_INSET},
            height: {TOOLTIP_LINE_H},
            text: {line.left_text.as_str()},
            font: "FrizQuadrata",
            font_size: 10.0,
            font_color: {rgba_string(line.left_color)},
            justify_h: "LEFT",
            anchor {
                point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                relative_point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                x: {TOOLTIP_INSET},
                y: {y},
            }
        }
        fontstring {
            name: {DynName(format!("TooltipLine{index}Right"))},
            width: {TOOLTIP_W - 2.0 * TOOLTIP_INSET},
            height: {TOOLTIP_LINE_H},
            text: {line.right_text.as_str()},
            font: "FrizQuadrata",
            font_size: 10.0,
            font_color: {rgba_string(line.right_color)},
            justify_h: "RIGHT",
            anchor {
                point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                relative_point: game_engine::ui::anchor::AnchorPoint::TopLeft,
                x: {TOOLTIP_INSET},
                y: {y},
            }
        }
    }
}

fn rgba_string(color: [f32; 4]) -> String {
    format!("{},{},{},{}", color[0], color[1], color[2], color[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_aura() -> AuraInstance {
        AuraInstance {
            spell_id: 100,
            name: "Blessing of Kings".into(),
            description: "Increases all stats.".into(),
            icon_fdid: 1,
            source: "Uther".into(),
            duration: 1800.0,
            remaining: 125.0,
            stacks: 2,
            is_debuff: false,
            debuff_type: game_engine::buff_data::DebuffType::None,
        }
    }

    #[test]
    fn parse_bag_slot_name_extracts_indices() {
        assert_eq!(parse_bag_slot_name("ContainerFrame0Slot3"), Some((0, 3)));
        assert_eq!(parse_bag_slot_name("ContainerFrame12Slot0"), Some((12, 0)));
        assert_eq!(parse_bag_slot_name("ContainerFrame0"), None);
    }

    #[test]
    fn parse_target_aura_name_extracts_index_from_children() {
        assert_eq!(
            parse_target_aura_name("TargetBuffIcon2Texture"),
            Some(HoveredAura {
                kind: HoveredAuraKind::Buff,
                index: 2,
            })
        );
        assert_eq!(
            parse_target_aura_name("TargetDebuffIcon4Timer"),
            Some(HoveredAura {
                kind: HoveredAuraKind::Debuff,
                index: 4,
            })
        );
        assert_eq!(parse_target_aura_name("TargetFrame"), None);
    }

    #[test]
    fn item_tooltip_includes_quality_and_stack_count() {
        let tooltip = item_tooltip(&InventorySlot {
            icon_fdid: 1,
            count: 20,
            quality: ItemQuality::Rare,
            name: "Iron Ore".into(),
        });
        assert_eq!(tooltip.title, "Iron Ore");
        assert_eq!(tooltip.lines[0].left_text, "Quality");
        assert_eq!(tooltip.lines[0].right_text, "Rare");
        assert_eq!(tooltip.lines[1].right_text, "20");
    }

    #[test]
    fn spell_tooltip_shows_passive_and_cooldown_details() {
        let tooltip = spell_tooltip(SpellbookSpell {
            id: 642,
            name: "Divine Shield",
            passive: false,
            icon_file_data_id: 1,
            cooldown_seconds: 300.0,
        });
        assert_eq!(tooltip.title, "Divine Shield");
        assert_eq!(tooltip.lines[0].left_text, "Active ability");
        assert_eq!(tooltip.lines[1].left_text, "Cooldown");
        assert_eq!(tooltip.lines[1].right_text, "5m 0s");
    }

    #[test]
    fn aura_tooltip_shows_description_duration_stacks_and_source() {
        let tooltip = aura_tooltip(&sample_aura(), false);
        assert_eq!(tooltip.title, "Blessing of Kings");
        assert_eq!(tooltip.lines[0].left_text, "Increases all stats.");
        assert_eq!(tooltip.lines[1].right_text, "2m");
        assert_eq!(tooltip.lines[2].right_text, "2");
        assert_eq!(tooltip.lines[3].right_text, "Uther");
    }

    #[test]
    fn aura_tooltip_uses_colorblind_debuff_title_color_when_enabled() {
        let mut aura = sample_aura();
        aura.is_debuff = true;
        aura.debuff_type = game_engine::buff_data::DebuffType::Poison;
        let tooltip = aura_tooltip(&aura, true);
        assert_eq!(
            tooltip.title_color,
            parse_rgba(aura.debuff_type.border_color_for_mode(true))
        );
    }
}
