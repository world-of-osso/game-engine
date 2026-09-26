use bevy::prelude::*;
use game_engine::bag_data::{InventorySlot, InventoryState, ItemQuality};
use game_engine::buff_data::{AuraInstance, AuraState, UnitAuraState};
use game_engine::experience_data::{self, ExperienceState};
use game_engine::mail_data::MailState;
use game_engine::merchant_data::{MerchantState, quality_color};
use game_engine::player_spells::{ActionBarSlots, parse_action_button_name};
use game_engine::spell_catalog::CatalogSpell;
use game_engine::spell_catalog::{SpellCatalog, SpellTextSources};
use game_engine::targeting::CurrentTarget;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::buff_frame_component::buff_button_at;
use game_engine::ui::screens::chat_frame_component::chat_spell_link_at;
use game_engine::ui::screens::inworld_hud_component::MINIMAP_MAIL_FRAME;
use game_engine::ui::screens::talent_frame_view::{TalentTooltip, TalentTooltips};
use game_engine::ui::spellbook_runtime::SpellbookUiRuntime;
use game_engine::ui::ui_errors::power_display_name;
use shared::components::PowerType;
use shared::protocol::ActionRef;
use ui_toolkit::rsx;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::client_options::GraphicsOptions;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
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
const TOOLTIP_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TOOLTIP_DESCRIPTION_COLOR: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const TOOLTIP_FONT_SIZE: f32 = 10.0;
const TOOLTIP_TEXT_W: f32 = TOOLTIP_W - 2.0 * TOOLTIP_INSET;

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

    fn colored(text: impl Into<String>, color: [f32; 4]) -> Self {
        Self {
            left_color: color,
            ..Self::new(text)
        }
    }

    fn pair(left: impl Into<String>, right: impl Into<String>) -> Self {
        Self {
            left_text: left.into(),
            right_text: right.into(),
            left_color: TOOLTIP_WHITE,
            right_color: TOOLTIP_WHITE,
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
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_tooltip_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_tooltip_frame_ui);
        app.add_systems(
            Update,
            (sync_tooltip_root_size, sync_tooltip_frame_state)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
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
    merchant: Option<Res<MerchantState>>,
    current_target: Res<CurrentTarget>,
    local_player: Query<Entity, With<LocalPlayer>>,
    target_auras: Query<&UnitAuraState>,
    aura_state: Option<Res<AuraState>>,
    graphics_options: Option<Res<GraphicsOptions>>,
    spells: HoveredSpellSources,
    talent_tooltips: Option<Res<TalentTooltips>>,
    experience: Option<Res<ExperienceState>>,
    mail: Option<Res<MailState>>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let state = build_state(
        &ui.registry,
        window,
        &inventory,
        merchant.as_deref(),
        &current_target,
        local_player.iter().next(),
        &target_auras,
        aura_state.as_deref(),
        graphics_options.as_deref(),
        &spells,
        talent_tooltips.as_deref(),
        experience.as_deref(),
        mail.as_deref(),
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
    merchant: Option<&MerchantState>,
    current_target: &CurrentTarget,
    local_player: Option<Entity>,
    target_auras: &Query<&UnitAuraState>,
    aura_state: Option<&AuraState>,
    graphics_options: Option<&GraphicsOptions>,
    spells: &HoveredSpellSources,
    talent_tooltips: Option<&TalentTooltips>,
    experience: Option<&ExperienceState>,
    mail: Option<&MailState>,
) -> TooltipFrameState {
    let Some(cursor) = ui_cursor_position(registry, window) else {
        return TooltipFrameState::hidden();
    };
    let Some(frame_id) = find_frame_at(registry, cursor.x, cursor.y) else {
        return TooltipFrameState::hidden();
    };
    let Some(content) = hovered_spell_tooltip(registry, frame_id, spells)
        .or_else(|| hovered_talent_tooltip(registry, frame_id, talent_tooltips))
        .or_else(|| hovered_item_tooltip(registry, frame_id, inventory))
        .or_else(|| hovered_merchant_tooltip(registry, frame_id, merchant))
        .or_else(|| hovered_xp_tooltip(registry, frame_id, experience))
        .or_else(|| hovered_mail_tooltip(registry, frame_id, mail))
        .or_else(|| hovered_player_aura_tooltip(registry, frame_id, aura_state, graphics_options))
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
    place_tooltip(content, cursor, registry)
}

#[derive(bevy::ecs::system::SystemParam)]
struct HoveredSpellSources<'w> {
    spellbook: Option<NonSend<'w, SpellbookUiRuntime>>,
    action_slots: Option<Res<'w, ActionBarSlots>>,
    catalog: Option<Res<'w, SpellCatalog>>,
    text: SpellTextSources<'w>,
}

fn hovered_spell_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    sources: &HoveredSpellSources,
) -> Option<TooltipFrameState> {
    let from_spellbook = sources
        .spellbook
        .as_deref()
        .and_then(|runtime| runtime.spell_for_frame(registry, frame_id));
    let spell_id = from_spellbook
        .or_else(|| {
            let slot = hovered_action_slot(registry, frame_id)?;
            match sources.action_slots.as_deref()?.get(slot)? {
                ActionRef::Spell(id) => Some(id),
                ActionRef::Item(_) | ActionRef::Macro(_) => None,
            }
        })
        .or_else(|| chat_spell_link_at(registry, frame_id))?;
    let catalog = sources.catalog.as_deref();
    let ctx = sources.text.context();
    Some(match catalog.and_then(|catalog| catalog.get(spell_id)) {
        Some(spell) => spell_tooltip(
            spell,
            catalog.and_then(|c| c.render_description(spell_id, &ctx)),
        ),
        None => unknown_spell_tooltip(spell_id),
    })
}

fn hovered_action_slot(registry: &FrameRegistry, mut frame_id: u64) -> Option<usize> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(slot) = frame.name.as_deref().and_then(parse_action_button_name) {
            return Some(slot);
        }
        frame_id = frame.parent_id?;
    }
}

/// The nearest named ancestor with talent tooltip content (`TalentNode_*`).
fn hovered_talent_tooltip(
    registry: &FrameRegistry,
    mut frame_id: u64,
    talent_tooltips: Option<&TalentTooltips>,
) -> Option<TooltipFrameState> {
    let tooltips = talent_tooltips?;
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(tooltip) = frame
            .name
            .as_deref()
            .and_then(|name| tooltips.by_frame.get(name))
        {
            return Some(talent_tooltip(tooltip));
        }
        frame_id = frame.parent_id?;
    }
}

fn talent_tooltip(tooltip: &TalentTooltip) -> TooltipFrameState {
    let mut lines = vec![TooltipLineState::new(tooltip.rank.clone())];
    lines.extend(description_lines(
        &tooltip.description,
        TOOLTIP_DESCRIPTION_COLOR,
    ));
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: tooltip.title.clone(),
        title_color: TOOLTIP_TEXT_COLOR,
        lines,
    }
}

/// `ExhaustionTickMixin:ExhaustionToolTipText`, shown from the XP bar's `OnEnter`.
fn hovered_xp_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    experience: Option<&ExperienceState>,
) -> Option<TooltipFrameState> {
    let update = experience?.leveling()?;
    crate::scenes::status_tracking_bar::in_exp_bar(registry, Some(frame_id))
        .then(|| xp_tooltip(&update))
}

fn xp_tooltip(update: &shared::protocol::PlayerXpUpdate) -> TooltipFrameState {
    let (rest_name, percent) = experience_data::rest_state(update);
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: experience_data::tooltip_title(update),
        title_color: TOOLTIP_WHITE,
        // EXHAUST_TOOLTIP1: the state name in gold, then white body lines.
        lines: vec![
            TooltipLineState::colored(rest_name, TOOLTIP_DESCRIPTION_COLOR),
            TooltipLineState::colored(format!("{percent}% of normal experience"), TOOLTIP_WHITE),
            TooltipLineState::colored("gained from monsters.", TOOLTIP_WHITE),
        ],
    }
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

/// `GameTooltip:SetMerchantItem` / `SetBuybackItem` for a `MerchantItem<n>` cell.
/// The server sends name, quality and counts only, so that is what it shows.
fn hovered_merchant_tooltip(
    registry: &FrameRegistry,
    mut frame_id: u64,
    merchant: Option<&MerchantState>,
) -> Option<TooltipFrameState> {
    let merchant = merchant.filter(|merchant| merchant.is_open())?;
    let index = loop {
        let frame = registry.get(frame_id)?;
        if let Some(index) = frame.name.as_deref().and_then(parse_merchant_cell_name) {
            break index;
        }
        frame_id = frame.parent_id?;
    };
    let (name, quality, count, stock) = merchant.cell_item(index)?;
    Some(merchant_tooltip(name, quality, count, stock))
}

/// `MerchantItem<n>` (1-based) exactly; its children carry longer names.
fn parse_merchant_cell_name(name: &str) -> Option<usize> {
    let index: usize = name.strip_prefix("MerchantItem")?.parse().ok()?;
    index.checked_sub(1)
}

fn merchant_tooltip(name: &str, quality: u8, count: u32, stock: Option<u32>) -> TooltipFrameState {
    let mut lines = Vec::new();
    if count > 1 {
        lines.push(TooltipLineState::key_value(
            "Stack Count",
            count.to_string(),
        ));
    }
    if let Some(stock) = stock {
        lines.push(TooltipLineState::key_value("In Stock", stock.to_string()));
    }
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: name.to_string(),
        title_color: parse_rgba(quality_color(quality)),
        lines,
    }
}

/// `MinimapMailFrameUpdate` / `FormatUnreadMailTooltip`: `HAVE_MAIL_FROM` and one line
/// per sender, or `HAVE_MAIL` without senders.
fn hovered_mail_tooltip(
    registry: &FrameRegistry,
    mut frame_id: u64,
    mail: Option<&MailState>,
) -> Option<TooltipFrameState> {
    loop {
        let frame = registry.get(frame_id)?;
        if frame.name.as_deref() == Some(MINIMAP_MAIL_FRAME) {
            break;
        }
        frame_id = frame.parent_id?;
    }
    Some(mail_tooltip(&mail?.pending_senders))
}

fn mail_tooltip(senders: &[String]) -> TooltipFrameState {
    let title = if senders.is_empty() {
        "You have unread mail"
    } else {
        "Unread mail from:"
    };
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: title.to_string(),
        title_color: TOOLTIP_WHITE,
        lines: senders
            .iter()
            .map(|sender| TooltipLineState::colored(sender.clone(), TOOLTIP_WHITE))
            .collect(),
    }
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

fn hovered_player_aura_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    aura_state: Option<&AuraState>,
    graphics_options: Option<&GraphicsOptions>,
) -> Option<TooltipFrameState> {
    let (is_debuff, index) = buff_button_at(registry, frame_id)?;
    let auras = aura_state?;
    let aura = if is_debuff {
        auras.debuffs().nth(index)
    } else {
        auras.buffs().nth(index)
    }?;
    let colorblind_mode = graphics_options.is_some_and(|graphics| graphics.colorblind_mode);
    Some(aura_tooltip(aura, colorblind_mode))
}

fn place_tooltip(
    mut tooltip: TooltipFrameState,
    cursor: Vec2,
    registry: &FrameRegistry,
) -> TooltipFrameState {
    let max_x = (registry.screen_width - TOOLTIP_W - TOOLTIP_MARGIN).max(TOOLTIP_MARGIN);
    let max_y = (registry.screen_height - tooltip.height() - TOOLTIP_MARGIN).max(TOOLTIP_MARGIN);
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

fn spell_tooltip(spell: &CatalogSpell, description: Option<String>) -> TooltipFrameState {
    let mut lines = Vec::new();
    if !spell.subtext.is_empty() {
        lines.push(TooltipLineState::new(spell.subtext.to_string()));
    }
    let (cost, range) = (spell_cost_text(spell), spell_range_text(spell));
    if !cost.is_empty() || !range.is_empty() {
        lines.push(TooltipLineState::pair(cost, range));
    }
    lines.push(TooltipLineState::pair(
        spell_cast_text(spell),
        spell_cooldown_text(spell),
    ));
    let description = description.unwrap_or_default();
    lines.extend(description_lines(&description, TOOLTIP_DESCRIPTION_COLOR));
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: spell.name.to_string(),
        title_color: TOOLTIP_WHITE,
        lines,
    }
}

fn unknown_spell_tooltip(spell_id: u32) -> TooltipFrameState {
    TooltipFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        title: format!("Spell {spell_id}"),
        title_color: TOOLTIP_SPELL_COLOR,
        lines: Vec::new(),
    }
}

/// First unconditional cost: "30 Rage", "1% of base mana".
fn spell_cost_text(spell: &CatalogSpell) -> String {
    let Some(cost) = spell
        .powers
        .iter()
        .find(|cost| cost.required_aura_spell_id == 0)
    else {
        return String::new();
    };
    let Some(power) = PowerType::from_db(cost.power_type.into()) else {
        return String::new();
    };
    let name = power_display_name(power).unwrap_or("power");
    if cost.flat > 0 {
        let amount = cost.flat as f32 / power_display_divisor(power);
        return format!("{} {}", trim_number(amount), title_case(name));
    }
    if cost.pct > 0.0 {
        let base = if power == PowerType::Mana {
            "base"
        } else {
            "maximum"
        };
        return format!("{}% of {base} {name}", trim_number(cost.pct));
    }
    String::new()
}

/// Raw pool units per displayed point (server `PowerType.DisplayModifier`).
fn power_display_divisor(power: PowerType) -> f32 {
    match power {
        PowerType::Rage
        | PowerType::RunicPower
        | PowerType::SoulShards
        | PowerType::LunarPower
        | PowerType::Pain => 10.0,
        PowerType::Insanity => 100.0,
        _ => 1.0,
    }
}

/// Hostile range when set, else friendly; none for self-only spells.
fn spell_range_text(spell: &CatalogSpell) -> String {
    let max = if spell.range.max_yd[0] > 0.0 {
        spell.range.max_yd[0]
    } else {
        spell.range.max_yd[1]
    };
    if max <= 0.0 {
        String::new()
    } else if max <= 5.0 {
        "Melee Range".to_string()
    } else {
        format!("{} yd range", trim_number(max))
    }
}

fn spell_cast_text(spell: &CatalogSpell) -> String {
    if spell.passive {
        "Passive".to_string()
    } else if spell.cast_time_ms <= 0 {
        "Instant".to_string()
    } else {
        format!(
            "{} sec cast",
            trim_number(spell.cast_time_ms as f32 / 1000.0)
        )
    }
}

fn spell_cooldown_text(spell: &CatalogSpell) -> String {
    if let Some(charges) = spell.charges.filter(|charges| charges.max_charges > 1) {
        return format!(
            "{} charges, {} recharge",
            charges.max_charges,
            duration_text(charges.recovery_ms)
        );
    }
    let recovery = spell
        .cooldown
        .recovery_ms
        .max(spell.cooldown.category_recovery_ms);
    if recovery == 0 {
        return String::new();
    }
    format!("{} cooldown", duration_text(recovery))
}

fn duration_text(ms: u32) -> String {
    let secs = ms as f32 / 1000.0;
    if secs >= 60.0 {
        format!("{} min", trim_number(secs / 60.0))
    } else {
        format!("{} sec", trim_number(secs))
    }
}

fn trim_number(value: f32) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map(str::to_string).unwrap_or(text)
}

fn title_case(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Description paragraphs word-wrapped to the tooltip text width. A paragraph that
/// opens with a `|cAARRGGBB` colour escape takes that colour; colour escapes and `|r`
/// are not shown. Blank source lines are kept as paragraph breaks.
fn description_lines(text: &str, color: [f32; 4]) -> Vec<TooltipLineState> {
    let mut lines = Vec::new();
    for paragraph in text.lines().map(str::trim) {
        let color = leading_color_escape(paragraph).unwrap_or(color);
        let plain = strip_color_escapes(paragraph);
        let wrapped = wrap_paragraph(&plain);
        if wrapped.is_empty() {
            lines.push(TooltipLineState::colored(String::new(), color));
        }
        lines.extend(
            wrapped
                .into_iter()
                .map(|line| TooltipLineState::colored(line, color)),
        );
    }
    while lines.last().is_some_and(|line| line.left_text.is_empty()) {
        lines.pop();
    }
    lines
}

/// Colour of a `|cAARRGGBB` escape at the start of `text`.
fn leading_color_escape(text: &str) -> Option<[f32; 4]> {
    let hex = text
        .strip_prefix("|c")
        .or_else(|| text.strip_prefix("|C"))?
        .get(..8)?;
    let channel = |at: usize| {
        u8::from_str_radix(&hex[at..at + 2], 16)
            .ok()
            .map(|v| f32::from(v) / 255.0)
    };
    Some([channel(2)?, channel(4)?, channel(6)?, channel(0)?])
}

fn strip_color_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('|') {
        out.push_str(&rest[..pos]);
        let escape = &rest[pos..];
        let skip = if leading_color_escape(escape).is_some() {
            10
        } else if escape.starts_with("|r") || escape.starts_with("|R") {
            2
        } else {
            out.push('|');
            1
        };
        rest = &escape[skip..];
    }
    out.push_str(rest);
    out
}

/// Greedy word wrap to the measured text width of a tooltip line.
fn wrap_paragraph(paragraph: &str) -> Vec<String> {
    let fits = |line: &str| {
        measure_text(line, GameFont::FrizQuadrata, TOOLTIP_FONT_SIZE)
            .expect("FrizQuadrata text measurement")
            .0
            <= TOOLTIP_TEXT_W
    };
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        if !line.is_empty() && !fits(&format!("{line} {word}")) {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn aura_tooltip(aura: &AuraInstance, colorblind_mode: bool) -> TooltipFrameState {
    let mut lines = description_lines(&aura.description, TOOLTIP_TEXT_COLOR);
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
            pos_type: "absolute",
            anchor: "screen",
            pos_x: {state.x},
            pos_y: {state.y},
            {title}
            {lines}
        }
    }
}

fn tooltip_title(state: &TooltipFrameState) -> Element {
    rsx! {
        fontstring {
            name: "TooltipTitle",
            width: {TOOLTIP_TEXT_W},
            height: {TOOLTIP_TITLE_H},
            text: {state.title.as_str()},
            font: "FrizQuadrata",
            font_size: 12.0,
            font_color: {rgba_string(state.title_color)},
            justify_h: "LEFT",
            pos_type: "absolute",
            pos_x: {TOOLTIP_INSET},
            pos_y: {TOOLTIP_INSET},
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
    let y = TOOLTIP_INSET + TOOLTIP_TITLE_H + index as f32 * TOOLTIP_LINE_H;
    rsx! {
        fontstring {
            name: {DynName(format!("TooltipLine{index}Left"))},
            width: {TOOLTIP_TEXT_W},
            height: {TOOLTIP_LINE_H},
            text: {line.left_text.as_str()},
            font: "FrizQuadrata",
            font_size: {TOOLTIP_FONT_SIZE},
            font_color: {rgba_string(line.left_color)},
            justify_h: "LEFT",
            pos_type: "absolute",
            pos_x: {TOOLTIP_INSET},
            pos_y: {y},
        }
        fontstring {
            name: {DynName(format!("TooltipLine{index}Right"))},
            width: {TOOLTIP_TEXT_W},
            height: {TOOLTIP_LINE_H},
            text: {line.right_text.as_str()},
            font: "FrizQuadrata",
            font_size: {TOOLTIP_FONT_SIZE},
            font_color: {rgba_string(line.right_color)},
            justify_h: "RIGHT",
            pos_type: "absolute",
            pos_x: {TOOLTIP_INSET},
            pos_y: {y},
        }
    }
}

fn rgba_string(color: [f32; 4]) -> String {
    format!("{},{},{},{}", color[0], color[1], color[2], color[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimap_mail_tooltip_lists_the_unread_senders_or_says_unread_mail() {
        let from = mail_tooltip(&["Tradea".into(), "Auction House".into()]);
        assert_eq!(from.title, "Unread mail from:");
        let lines: Vec<_> = from
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .collect();
        assert_eq!(lines, ["Tradea", "Auction House"]);
        let none = mail_tooltip(&[]);
        assert_eq!(none.title, "You have unread mail");
        assert!(none.lines.is_empty());
    }

    fn sample_aura() -> AuraInstance {
        AuraInstance {
            instance_id: 9,
            spell_id: 100,
            name: "Blessing of Kings".into(),
            description: "Increases all stats.".into(),
            icon_fdid: 1,
            source: "Uther".into(),
            from_local_player: false,
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
    fn merchant_cells_show_the_item_name_in_quality_color_with_stack_and_stock() {
        assert_eq!(parse_merchant_cell_name("MerchantItem2"), Some(1));
        assert_eq!(parse_merchant_cell_name("MerchantItem2Name"), None);
        let tooltip = merchant_tooltip("Refreshing Spring Water", 1, 5, Some(3));
        assert_eq!(tooltip.title, "Refreshing Spring Water");
        assert_eq!(tooltip.title_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(tooltip.lines[0].right_text, "5");
        assert_eq!(tooltip.lines[1].left_text, "In Stock");
        let uncommon = merchant_tooltip("Pattern: Blue Linen Vest", 2, 1, None);
        assert_eq!(uncommon.title_color, [0.12, 1.0, 0.0, 1.0]);
        assert!(uncommon.lines.is_empty());
    }

    #[test]
    fn xp_tooltip_uses_xp_text_and_the_rest_state() {
        let rested = xp_tooltip(&shared::protocol::PlayerXpUpdate {
            xp: 1_234,
            next_level_xp: 4_000,
            rested_xp: 800,
        });
        assert_eq!(rested.title, "1,234 / 4,000  ( 31% )");
        let lines: Vec<_> = rested.lines.iter().map(|l| l.left_text.as_str()).collect();
        assert_eq!(
            lines,
            [
                "Rested",
                "200% of normal experience",
                "gained from monsters."
            ]
        );
        let normal = xp_tooltip(&shared::protocol::PlayerXpUpdate {
            xp: 0,
            next_level_xp: 4_000,
            rested_xp: 0,
        });
        assert_eq!(normal.title, "0 / 4,000  ( 0% )");
        assert_eq!(normal.lines[0].left_text, "Normal");
        assert_eq!(normal.lines[1].left_text, "100% of normal experience");
    }

    #[test]
    fn item_tooltip_includes_quality_and_stack_count() {
        let tooltip = item_tooltip(&InventorySlot {
            icon_fdid: 1,
            count: 20,
            quality: ItemQuality::Rare,
            name: "Iron Ore".into(),
            ..Default::default()
        });
        assert_eq!(tooltip.title, "Iron Ore");
        assert_eq!(tooltip.lines[0].left_text, "Quality");
        assert_eq!(tooltip.lines[0].right_text, "Rare");
        assert_eq!(tooltip.lines[1].right_text, "20");
    }

    fn judgment() -> CatalogSpell {
        CatalogSpell {
            id: 20271,
            name: "Judgment".into(),
            subtext: "Rank 2".into(),
            range: game_engine::spell_catalog::SpellRange {
                min_yd: [0.0, 0.0],
                max_yd: [30.0, 30.0],
            },
            cooldown: game_engine::spell_catalog::SpellCooldown {
                recovery_ms: 12_000,
                category_recovery_ms: 0,
                gcd_ms: 1500,
            },
            powers: vec![game_engine::spell_catalog::SpellPowerCost {
                power_type: 0,
                flat: 0,
                pct: 3.0,
                required_aura_spell_id: 0,
            }]
            .into(),
            ..Default::default()
        }
    }

    fn texts(tooltip: &TooltipFrameState) -> Vec<(String, String)> {
        tooltip
            .lines
            .iter()
            .map(|line| (line.left_text.clone(), line.right_text.clone()))
            .collect()
    }

    #[test]
    fn spell_tooltip_shows_rank_cost_range_cast_cooldown_and_description() {
        let description = "Judges the target, dealing 0 Holy damage and causing them to take 20% \
                           increased damage from your next Holy Power ability.";
        let tooltip = spell_tooltip(&judgment(), Some(description.to_string()));
        assert_eq!(tooltip.title, "Judgment");
        let pair = |left: &str, right: &str| (left.to_string(), right.to_string());
        assert_eq!(
            texts(&tooltip),
            [
                pair("Rank 2", ""),
                pair("3% of base mana", "30 yd range"),
                pair("Instant", "12 sec cooldown"),
                pair("Judges the target, dealing 0 Holy damage and", ""),
                pair("causing them to take 20% increased damage from", ""),
                pair("your next Holy Power ability.", ""),
            ]
        );
    }

    #[test]
    fn spell_tooltip_scales_rage_costs_and_shows_melee_charges() {
        let strike = CatalogSpell {
            id: 1,
            name: "Strike".into(),
            cast_time_ms: 1500,
            range: game_engine::spell_catalog::SpellRange {
                min_yd: [0.0, 0.0],
                max_yd: [5.0, 5.0],
            },
            charges: Some(game_engine::spell_catalog::SpellCharges {
                max_charges: 2,
                recovery_ms: 6000,
            }),
            powers: vec![game_engine::spell_catalog::SpellPowerCost {
                power_type: 1,
                flat: 300,
                pct: 0.0,
                required_aura_spell_id: 0,
            }]
            .into(),
            ..Default::default()
        };
        let tooltip = spell_tooltip(&strike, None);
        let pair = |left: &str, right: &str| (left.to_string(), right.to_string());
        assert_eq!(
            texts(&tooltip),
            [
                pair("30 Rage", "Melee Range"),
                pair("1.5 sec cast", "2 charges, 6 sec recharge"),
            ]
        );
    }

    #[test]
    fn hovered_action_slot_resolves_from_button_children() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let button = registry.create_frame("ActionButton1_2", None);
        let icon = registry.create_frame("ActionButton1_2Icon", Some(button));
        assert_eq!(hovered_action_slot(&registry, icon), Some(1));
    }

    #[test]
    fn talent_tooltip_shows_rank_and_wrapped_description_for_hovered_node_child() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let node = registry.create_frame("TalentNode_81526", None);
        let icon = registry.create_frame("TalentNode_81526Icon", Some(node));
        let mut tooltips = TalentTooltips::default();
        tooltips.by_frame.insert(
            "TalentNode_81526".into(),
            TalentTooltip {
                title: "Blade of Justice".into(),
                rank: "Rank 0/1".into(),
                description: "Pierce an enemy with a blade of light, dealing Holy damage and \
                              generating 1 Holy Power."
                    .into(),
            },
        );
        let tooltip =
            hovered_talent_tooltip(&registry, icon, Some(&tooltips)).expect("talent tooltip");
        assert_eq!(tooltip.title, "Blade of Justice");
        let lines: Vec<&str> = tooltip.lines.iter().map(|l| l.left_text.as_str()).collect();
        assert_eq!(
            lines,
            [
                "Rank 0/1",
                "Pierce an enemy with a blade of light, dealing Holy",
                "damage and generating 1 Holy Power.",
            ]
        );
    }

    #[test]
    fn aura_tooltip_shows_description_duration_stacks_and_source() {
        let tooltip = aura_tooltip(&sample_aura(), false);
        assert_eq!(tooltip.title, "Blessing of Kings");
        assert_eq!(tooltip.lines[0].left_text, "Increases all stats.");
        assert_eq!(tooltip.lines[1].right_text, "3 m");
        assert_eq!(tooltip.lines[2].right_text, "2");
        assert_eq!(tooltip.lines[3].right_text, "Uther");
    }

    #[test]
    fn hovering_a_player_buff_shows_its_rendered_description() {
        use game_engine::ui::screens::buff_frame_component::{BuffFrameState, buff_frame_screen};
        let auras = AuraState {
            auras: vec![sample_aura()],
        };
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(BuffFrameState::from_auras(&auras, false));
        Screen::new(buff_frame_screen).sync(&shared, &mut registry);
        let icon = registry.get_by_name("BuffButton0Icon").unwrap();
        let tooltip =
            hovered_player_aura_tooltip(&registry, icon, Some(&auras), None).expect("tooltip");
        assert_eq!(tooltip.title, "Blessing of Kings");
        assert_eq!(tooltip.lines[0].left_text, "Increases all stats.");
        let root = registry.get_by_name("BuffFrame").unwrap();
        assert!(hovered_player_aura_tooltip(&registry, root, Some(&auras), None).is_none());
    }

    #[test]
    fn aura_tooltip_wraps_multiline_descriptions_inside_the_background() {
        let mut aura = sample_aura();
        // Power Word: Fortitude with its Magic damage line shown.
        aura.description = "Stamina increased by 5%.\r\nMagic damage taken reduced by 3% while \
                            the caster's Fortitude remains on you."
            .into();
        let tooltip = aura_tooltip(&aura, false);
        let texts: Vec<&str> = tooltip
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .collect();
        assert_eq!(
            texts[..3],
            [
                "Stamina increased by 5%.",
                "Magic damage taken reduced by 3% while the",
                "caster's Fortitude remains on you."
            ]
        );
        let text_bottom =
            TOOLTIP_INSET + TOOLTIP_TITLE_H + tooltip.lines.len() as f32 * TOOLTIP_LINE_H;
        assert!(tooltip.height() >= text_bottom + TOOLTIP_INSET);
    }

    #[test]
    fn description_lines_fit_the_tooltip_text_width() {
        // Live Slam tooltip text plus a second sentence, several lines long.
        let text = "Slams an opponent, causing {?$s1} Physical damage. Hits up to 3 \
                    additional targets within 8 yards for 45% of the damage dealt.";
        let tooltip = spell_tooltip(&judgment(), Some(text.into()));
        let description: Vec<&str> = tooltip
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .skip_while(|line| !line.starts_with("Slams"))
            .collect();
        assert_eq!(description.join(" "), text);
        for line in description {
            let (width, _) = measure_text(line, GameFont::FrizQuadrata, TOOLTIP_FONT_SIZE).unwrap();
            assert!(width <= TOOLTIP_TEXT_W, "{line:?} is {width}px");
        }
    }

    #[test]
    fn description_colour_escapes_colour_their_paragraph_and_are_not_shown() {
        // Rendered Crusader Strike (35395) description.
        let text = "Strike the target for 12 Physical damage.\r\n\r\n\
                    |cFFFFFFFFGenerates 1 Holy Power.|r";
        let lines = description_lines(text, TOOLTIP_DESCRIPTION_COLOR);
        let shown: Vec<(&str, [f32; 4])> = lines
            .iter()
            .map(|line| (line.left_text.as_str(), line.left_color))
            .collect();
        assert_eq!(
            shown,
            [
                (
                    "Strike the target for 12 Physical damage.",
                    TOOLTIP_DESCRIPTION_COLOR
                ),
                ("", TOOLTIP_DESCRIPTION_COLOR),
                ("Generates 1 Holy Power.", [1.0, 1.0, 1.0, 1.0]),
            ]
        );
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
