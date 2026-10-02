use bevy::prelude::*;
use game_engine::bag_data::InventoryState;
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
use game_engine::ui::screens::character_frame_component::parse_equipment_slot_action;
use game_engine::ui::screens::chat_frame_component::chat_spell_link_at;
use game_engine::ui::screens::inworld_hud_component::MINIMAP_MAIL_FRAME;
use game_engine::ui::screens::inworld_unit_frames_component::{TargetAuraView, target_frame_auras};
use game_engine::ui::screens::talent_frame_view::{TalentTooltip, TalentTooltips};
use game_engine::ui::spellbook_runtime::SpellbookUiRuntime;
use game_engine::ui::ui_errors::power_display_name;
use shared::components::PowerType;
use shared::protocol::ActionRef;
use ui_toolkit::layout::LayoutRect;
use ui_toolkit::screen::{Screen, SharedContext};
#[cfg(test)]
use ui_toolkit::text_measure::measure_text;
#[cfg(test)]
use ui_toolkit::widgets::font_string::GameFont;

use crate::client_options::GraphicsOptions;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;

mod item_tooltip;
mod unit_sources;
mod unit_tooltip;

use item_tooltip::{TooltipItem, item_tooltip};
use unit_sources::{FactionNames, UnitTooltipSources};

use game_engine::tooltip_presentation::{
    GRAY_FONT_COLOR, ItemMark, TOOLTIP_BUFF_COLOR, TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_LABEL_COLOR,
    TOOLTIP_SPELL_COLOR, TOOLTIP_TEXT_COLOR, TOOLTIP_W, TOOLTIP_WHITE, TooltipLineState,
    TooltipPresentation, description_lines, item_id_line, parse_rgba, tooltip_frame_screen,
    tooltip_height,
};
#[cfg(test)]
use game_engine::tooltip_presentation::{
    TOOLTIP_FONT_SIZE, TOOLTIP_INSET, TOOLTIP_LINE_H, TOOLTIP_TEXT_W, TOOLTIP_TITLE_H,
};

/// `GameTooltipDefaultContainer` (GameTooltip.xml:242-247): BOTTOMRIGHT of UIParent
/// at x -9, y 85; `GameTooltip_SetDefaultAnchor` puts the tooltip's BOTTOMRIGHT there.
const TOOLTIP_DEFAULT_ANCHOR_X: f32 = -9.0;
const TOOLTIP_DEFAULT_ANCHOR_Y: f32 = 85.0;
/// The record a tooltip describes; `place_tooltip` ends the tooltip with its ID
/// line (a user-requested deviation from Retail, like idTip-style addons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TooltipRecord {
    Creature(u32),
    Spell(u32),
    Item(u32),
}

impl TooltipRecord {
    fn id_line(self) -> TooltipLineState {
        let text = match self {
            Self::Creature(id) => format!("Creature ID: {id}"),
            Self::Spell(id) => format!("Spell ID: {id}"),
            Self::Item(id) => return item_id_line(id),
        };
        TooltipLineState::colored(text, GRAY_FONT_COLOR)
    }
}

/// Where a tooltip goes: Retail `GameTooltip_SetDefaultAnchor`, or
/// `GameTooltip:SetOwner(owner, anchorType)` next to the hovered frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum TooltipAnchor {
    /// Bottom right of the screen (`GameTooltipDefaultContainer`).
    #[default]
    Default,
    Owner {
        frame: u64,
        side: OwnerSide,
    },
}

/// `SetOwner` anchor types (Widget API): `ANCHOR_RIGHT` puts the tooltip's
/// BOTTOMLEFT on the owner's TOPRIGHT, `ANCHOR_LEFT` its BOTTOMRIGHT on the
/// owner's TOPLEFT, `ANCHOR_BOTTOMLEFT` its TOPRIGHT on the owner's BOTTOMLEFT.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnerSide {
    Right,
    Left,
    BottomLeft,
    /// `ContainerFrameItemButton_CalculateItemTooltipAnchors`
    /// (ContainerFrame.lua:1448-1458): right when the owner's right edge is left
    /// of the screen centre, else left.
    RightOrLeftByRightEdge,
    /// TargetFrame.xml:35-40 aura `OnEnter`: left when the owner's centre is
    /// right of the screen centre, else right.
    RightOrLeftByCenter,
}

#[derive(Clone, Debug, PartialEq, Default)]
struct TooltipFrameState {
    visible: bool,
    x: f32,
    y: f32,
    title: String,
    title_color: [f32; 4],
    lines: Vec<TooltipLineState>,
    record: Option<TooltipRecord>,
    anchor: TooltipAnchor,
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
            record: None,
            anchor: TooltipAnchor::Default,
        }
    }

    fn height(&self) -> f32 {
        tooltip_height(self.lines.len())
    }

    fn presentation(&self) -> TooltipPresentation {
        TooltipPresentation {
            visible: self.visible,
            x: self.x,
            y: self.y,
            title: self.title.clone(),
            title_color: self.title_color,
            lines: self.lines.clone(),
        }
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
        app.insert_resource(FactionNames::load());
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
    shared.insert(state.presentation());
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
    items: ItemSources,
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
    mut units: UnitTooltipSources,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    units.request_hovered();
    let state = build_state(
        &ui.registry,
        window,
        &items.items(),
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
        &units,
    );
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state.presentation());
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// Bag and equipped items and the viewer's level for item tooltips.
#[derive(bevy::ecs::system::SystemParam)]
struct ItemSources<'w> {
    inventory: Res<'w, InventoryState>,
    stats: Option<Res<'w, game_engine::status::CharacterStatsSnapshot>>,
}

impl ItemSources<'_> {
    fn items(&self) -> TooltipItems<'_> {
        TooltipItems {
            inventory: &self.inventory,
            player_level: self.stats.as_ref().and_then(|stats| stats.level),
        }
    }
}

struct TooltipItems<'a> {
    inventory: &'a InventoryState,
    player_level: Option<u16>,
}

fn build_state(
    registry: &FrameRegistry,
    window: &Window,
    inventory: &TooltipItems,
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
    units: &UnitTooltipSources,
) -> TooltipFrameState {
    let Some(cursor) = ui_cursor_position(registry, window) else {
        return TooltipFrameState::hidden();
    };
    let frame_content = find_frame_at(registry, cursor.x, cursor.y).and_then(|frame_id| {
        hovered_frame_tooltip(
            registry,
            frame_id,
            inventory,
            merchant,
            current_target,
            local_player,
            target_auras,
            aura_state,
            graphics_options,
            spells,
            talent_tooltips,
            experience,
            mail,
        )
    });
    let Some(content) = frame_content.or_else(|| units.tooltip()) else {
        return TooltipFrameState::hidden();
    };
    place_tooltip(content, registry)
}

/// The tooltip of the UI frame under the cursor, if it has one.
fn hovered_frame_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    inventory: &TooltipItems,
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
) -> Option<TooltipFrameState> {
    hovered_spell_tooltip(registry, frame_id, spells)
        .or_else(|| hovered_talent_tooltip(registry, frame_id, talent_tooltips))
        .or_else(|| hovered_item_tooltip(registry, frame_id, inventory))
        .or_else(|| hovered_equipped_tooltip(registry, frame_id, inventory))
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
    // SpellBookItem.lua:494 `SetOwner(self.Button, "ANCHOR_RIGHT")`; action
    // buttons use the default anchor with Retail's default `UberTooltips` 1
    // (ActionButton.lua:1070-1080).
    let anchor = if from_spellbook.is_some() {
        TooltipAnchor::Owner {
            frame: frame_id,
            side: OwnerSide::Right,
        }
    } else {
        TooltipAnchor::Default
    };
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
    let tooltip = match catalog.and_then(|catalog| catalog.get(spell_id)) {
        Some(spell) => spell_tooltip(
            spell,
            catalog.and_then(|c| c.render_description(spell_id, &ctx)),
        ),
        None => unknown_spell_tooltip(spell_id),
    };
    Some(TooltipFrameState { anchor, ..tooltip })
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
            // TalentDisplayMixin:AcquireTooltip (Blizzard_TalentDisplay.lua:127-128).
            return Some(owned_by(
                frame_id,
                OwnerSide::Right,
                talent_tooltip(tooltip),
            ));
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
        record: Some(TooltipRecord::Spell(tooltip.spell_id)),
        anchor: TooltipAnchor::Default,
    }
}

/// `ExhaustionTickMixin:ExhaustionToolTipText`, shown from the XP bar's `OnEnter`,
/// at the default anchor (`GameTooltip_SetDefaultAnchor(tooltip, UIParent)`,
/// ExpBarOverrides.lua:30).
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
        record: None,
        anchor: TooltipAnchor::Default,
    }
}

fn hovered_item_tooltip(
    registry: &FrameRegistry,
    frame_id: u64,
    items: &TooltipItems,
) -> Option<TooltipFrameState> {
    let (bag_index, slot_index) = hovered_bag_slot(registry, frame_id)?;
    let slot = items.inventory.slot(bag_index, slot_index)?;
    let owner = ancestor_named(registry, frame_id, |name| {
        parse_bag_slot_name(name).is_some()
    })?;
    // ContainerFrameItemButtonMixin:OnUpdate: `ANCHOR_NONE` + CalculateItemTooltipAnchors.
    (!slot.is_empty()).then(|| {
        let tooltip = item_tooltip(TooltipItem {
            slot,
            player_level: items.player_level,
        });
        owned_by(owner, OwnerSide::RightOrLeftByRightEdge, tooltip)
    })
}

/// `GameTooltip:SetInventoryItem` over a paperdoll slot
/// (`PaperDollItemSlotButton_OnEnter`: `ANCHOR_RIGHT`).
fn hovered_equipped_tooltip(
    registry: &FrameRegistry,
    mut frame_id: u64,
    items: &TooltipItems,
) -> Option<TooltipFrameState> {
    let slot = loop {
        let frame = registry.get(frame_id)?;
        if let Some(slot) = frame
            .onclick
            .as_deref()
            .and_then(parse_equipment_slot_action)
        {
            break slot;
        }
        frame_id = frame.parent_id?;
    };
    let item = items.inventory.equipped(slot)?;
    let tooltip = item_tooltip(TooltipItem {
        slot: item,
        player_level: items.player_level,
    });
    Some(owned_by(frame_id, OwnerSide::Right, tooltip))
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
    let (item_id, name, quality, count, stock) = merchant.cell_item(index)?;
    // MerchantItemButton_OnEnter (MerchantFrame.lua:710-711): `ANCHOR_RIGHT`.
    Some(owned_by(
        frame_id,
        OwnerSide::Right,
        merchant_tooltip(item_id, name, quality, count, stock),
    ))
}

/// `MerchantItem<n>` (1-based) exactly; its children carry longer names.
fn parse_merchant_cell_name(name: &str) -> Option<usize> {
    let index: usize = name.strip_prefix("MerchantItem")?.parse().ok()?;
    index.checked_sub(1)
}

fn merchant_tooltip(
    item_id: u32,
    name: &str,
    quality: u8,
    count: u32,
    stock: Option<u32>,
) -> TooltipFrameState {
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
        record: Some(TooltipRecord::Item(item_id)),
        anchor: TooltipAnchor::Default,
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
    // MiniMapMailFrameMixin:OnEnter (Minimap.lua:500-501): `ANCHOR_BOTTOMLEFT`.
    Some(owned_by(
        frame_id,
        OwnerSide::BottomLeft,
        mail_tooltip(&mail?.pending_senders),
    ))
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
        record: None,
        anchor: TooltipAnchor::Default,
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
    let owner = ancestor_named(registry, frame_id, |name| {
        parse_target_aura_name(name).is_some()
    })?;
    // TargetFrame.xml:35-40 aura `OnEnter`: left or right by the aura's centre.
    Some(owned_by(
        owner,
        OwnerSide::RightOrLeftByCenter,
        aura_tooltip(aura, colorblind_mode),
    ))
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
    // AuraButtonMixin:OnEnter (BuffFrame.lua:888-899): `ANCHOR_BOTTOMLEFT` on the
    // aura button under the cursor.
    Some(owned_by(
        frame_id,
        OwnerSide::BottomLeft,
        aura_tooltip(aura, colorblind_mode),
    ))
}

/// Place the tooltip at its anchor (clamped to the screen, `clampedToScreen` in
/// SharedTooltipTemplates.xml:10). Every tooltip that describes a record ends with
/// its grey ID line.
fn place_tooltip(mut tooltip: TooltipFrameState, registry: &FrameRegistry) -> TooltipFrameState {
    if let Some(record) = tooltip.record {
        tooltip.lines.push(record.id_line());
    }
    let height = tooltip.height();
    let (x, y) = match tooltip.anchor {
        TooltipAnchor::Default => default_anchor_position(registry, height),
        TooltipAnchor::Owner { frame, side } => {
            let Some(owner) = registry
                .get(frame)
                .and_then(|frame| frame.layout_rect.clone())
            else {
                return TooltipFrameState::hidden();
            };
            owner_anchor_position(&owner, side, registry.screen_width, height)
        }
    };
    tooltip.visible = true;
    tooltip.x = x.clamp(0.0, (registry.screen_width - TOOLTIP_W).max(0.0));
    tooltip.y = y.clamp(0.0, (registry.screen_height - height).max(0.0));
    tooltip
}

/// Retail `GameTooltip_SetDefaultAnchor` (SharedTooltipTemplates.lua:87-114): the
/// tooltip's bottom-right corner on `GameTooltipDefaultContainer`'s, 9 left of and
/// 85 above UIParent's bottom right.
fn default_anchor_position(registry: &FrameRegistry, height: f32) -> (f32, f32) {
    (
        registry.screen_width + TOOLTIP_DEFAULT_ANCHOR_X - TOOLTIP_W,
        registry.screen_height - TOOLTIP_DEFAULT_ANCHOR_Y - height,
    )
}

/// Top-left of a tooltip `height` tall anchored to `owner` (screen coordinates, y down).
fn owner_anchor_position(
    owner: &LayoutRect,
    side: OwnerSide,
    screen_width: f32,
    height: f32,
) -> (f32, f32) {
    let right_of = (owner.x + owner.width, owner.y - height);
    let left_of = (owner.x - TOOLTIP_W, owner.y - height);
    match side {
        OwnerSide::Right => right_of,
        OwnerSide::Left => left_of,
        OwnerSide::BottomLeft => (owner.x - TOOLTIP_W, owner.y + owner.height),
        OwnerSide::RightOrLeftByRightEdge if owner.x + owner.width < screen_width / 2.0 => right_of,
        OwnerSide::RightOrLeftByRightEdge => left_of,
        OwnerSide::RightOrLeftByCenter if owner.x + owner.width / 2.0 > screen_width / 2.0 => {
            left_of
        }
        OwnerSide::RightOrLeftByCenter => right_of,
    }
}

/// `tooltip` anchored beside `frame` (`SetOwner(frame, side)`).
fn owned_by(frame: u64, side: OwnerSide, tooltip: TooltipFrameState) -> TooltipFrameState {
    TooltipFrameState {
        anchor: TooltipAnchor::Owner { frame, side },
        ..tooltip
    }
}

/// The nearest ancestor of `frame_id` (itself included) whose name matches.
fn ancestor_named(
    registry: &FrameRegistry,
    mut frame_id: u64,
    matches: impl Fn(&str) -> bool,
) -> Option<u64> {
    loop {
        let frame = registry.get(frame_id)?;
        if frame.name.as_deref().is_some_and(&matches) {
            return Some(frame_id);
        }
        frame_id = frame.parent_id?;
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
        record: Some(TooltipRecord::Spell(spell.id)),
        anchor: TooltipAnchor::Default,
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
        record: Some(TooltipRecord::Spell(spell_id)),
        anchor: TooltipAnchor::Default,
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
        record: Some(TooltipRecord::Spell(aura.spell_id)),
        anchor: TooltipAnchor::Default,
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
    // TargetFrame's sort; other players' debuffs on a hostile NPC, which the frame hides,
    // are not told apart here.
    let view = TargetAuraView {
        player_is_target: local,
        friendly: local,
        hostile_npc: false,
    };
    let (buffs, debuffs) = target_frame_auras(auras, view);
    match hovered.kind {
        HoveredAuraKind::Buff => buffs.get(hovered.index).copied(),
        HoveredAuraKind::Debuff => debuffs.get(hovered.index).copied(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::bag_data::{InventorySlot, ItemQuality};

    /// Left text and colour of the placed tooltip's last line.
    fn placed_id_line(content: TooltipFrameState) -> (String, [f32; 4]) {
        let placed = place_tooltip(content, &FrameRegistry::new(1920.0, 1080.0));
        let last = placed.lines.last().expect("id line");
        (last.left_text.clone(), last.left_color)
    }

    #[test]
    fn every_record_tooltip_ends_with_its_grey_id_line() {
        let frostbolt = CatalogSpell {
            id: 116,
            name: "Frostbolt".into(),
            ..judgment()
        };
        let linen = InventorySlot {
            icon_fdid: 132_889,
            count: 20,
            quality: ItemQuality::Common,
            name: "Linen Cloth".into(),
            item_guid: 7,
            item_id: 2589,
            ..Default::default()
        };
        let talent = TalentTooltip {
            spell_id: 184_575,
            title: "Blade of Justice".into(),
            rank: "Rank 0/1".into(),
            description: String::new(),
        };
        let cases = [
            (spell_tooltip(&frostbolt, None), "Spell ID: 116"),
            (unknown_spell_tooltip(116), "Spell ID: 116"),
            (talent_tooltip(&talent), "Spell ID: 184575"),
            (aura_tooltip(&sample_aura(), false), "Spell ID: 100"),
            (
                item_tooltip(TooltipItem {
                    slot: &linen,
                    player_level: None,
                }),
                "Item ID: 2589",
            ),
            (
                merchant_tooltip(2589, "Linen Cloth", 1, 1, None),
                "Item ID: 2589",
            ),
        ];
        for (content, expected) in cases {
            let lines_before = content.lines.len();
            let placed = place_tooltip(content.clone(), &FrameRegistry::new(1920.0, 1080.0));
            assert_eq!(placed.lines.len(), lines_before + 1, "{expected}");
            assert_eq!(
                placed_id_line(content),
                (expected.to_string(), GRAY_FONT_COLOR)
            );
        }
        // No record, no ID line.
        let mail = place_tooltip(mail_tooltip(&[]), &FrameRegistry::new(1920.0, 1080.0));
        assert!(mail.lines.is_empty());
    }

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LayoutRect {
        LayoutRect {
            x,
            y,
            width,
            height,
        }
    }

    /// A 36×36 owner at `(x, y)` on a 1920×1080 screen, and a 5-line tooltip.
    fn placed_beside(x: f32, y: f32, side: OwnerSide) -> (f32, f32, f32) {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let owner = registry.create_frame("Owner", None);
        registry
            .set_computed_layout(owner, rect(x, y, 36.0, 36.0))
            .unwrap();
        let content = TooltipFrameState {
            lines: vec![TooltipLineState::new("line"); 5],
            ..TooltipFrameState::hidden()
        };
        let height = content.height();
        let placed = place_tooltip(owned_by(owner, side, content), &registry);
        assert!(placed.visible);
        (placed.x, placed.y, height)
    }

    #[test]
    fn owner_anchors_follow_the_retail_anchor_types() {
        // ANCHOR_RIGHT: BOTTOMLEFT on the owner's TOPRIGHT.
        let (x, y, h) = placed_beside(600.0, 500.0, OwnerSide::Right);
        assert_eq!((x, y), (636.0, 500.0 - h));
        // ANCHOR_LEFT: BOTTOMRIGHT on the owner's TOPLEFT.
        let (x, y, h) = placed_beside(600.0, 500.0, OwnerSide::Left);
        assert_eq!((x, y), (600.0 - TOOLTIP_W, 500.0 - h));
        // ANCHOR_BOTTOMLEFT: TOPRIGHT on the owner's BOTTOMLEFT.
        let (x, y, _) = placed_beside(1600.0, 20.0, OwnerSide::BottomLeft);
        assert_eq!((x, y), (1600.0 - TOOLTIP_W, 56.0));
        // Bag slots: right of a slot in the left half, left of one in the right half.
        let (x, _, _) = placed_beside(600.0, 500.0, OwnerSide::RightOrLeftByRightEdge);
        assert_eq!(x, 636.0);
        let (x, _, _) = placed_beside(1700.0, 500.0, OwnerSide::RightOrLeftByRightEdge);
        assert_eq!(x, 1700.0 - TOOLTIP_W);
        // Target auras: by the aura's centre.
        let (x, _, _) = placed_beside(1000.0, 200.0, OwnerSide::RightOrLeftByCenter);
        assert_eq!(x, 1000.0 - TOOLTIP_W);
        let (x, _, _) = placed_beside(300.0, 200.0, OwnerSide::RightOrLeftByCenter);
        assert_eq!(x, 336.0);
        // Clamped to the screen: a right-anchored tooltip at the top edge.
        let (x, y, _) = placed_beside(1800.0, 10.0, OwnerSide::Right);
        assert_eq!((x, y), (1920.0 - TOOLTIP_W, 0.0));
    }

    #[test]
    fn merchant_cells_anchor_right_of_the_cell_and_action_buttons_use_the_default() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let cell = registry.create_frame("MerchantItem1", None);
        let name = registry.create_frame("MerchantItem1Name", Some(cell));
        let mut merchant = MerchantState::default();
        merchant.npc = Some(1);
        merchant.items = vec![shared::protocol::VendorItem {
            slot: 1,
            item_id: 2117,
            name: "Thin Cloth Shoes".into(),
            quality: 1,
            price: 5,
            stack_count: 1,
            max_stack: 1,
            num_available: None,
            usable: true,
            max_durability: None,
        }];
        let tooltip = hovered_merchant_tooltip(&registry, name, Some(&merchant)).expect("merchant");
        assert_eq!(
            tooltip.anchor,
            TooltipAnchor::Owner {
                frame: cell,
                side: OwnerSide::Right
            }
        );
        assert_eq!(unknown_spell_tooltip(1464).anchor, TooltipAnchor::Default);
    }

    #[test]
    fn tooltips_use_the_retail_default_anchor_at_the_bottom_right() {
        let registry = FrameRegistry::new(1920.0, 1080.0);
        let content = mail_tooltip(&["Tradea".into()]);
        let height = content.height();

        let placed = place_tooltip(content, &registry);

        assert!(placed.visible);
        // BOTTOMRIGHT at UIParent BOTTOMRIGHT (-9, +85).
        assert_eq!(placed.x + TOOLTIP_W, 1920.0 - 9.0);
        assert_eq!(placed.y + height, 1080.0 - 85.0);
    }

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
            from_player: false,
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
        let tooltip = merchant_tooltip(159, "Refreshing Spring Water", 1, 5, Some(3));
        assert_eq!(tooltip.title, "Refreshing Spring Water");
        assert_eq!(tooltip.title_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(tooltip.lines[0].right_text, "5");
        assert_eq!(tooltip.lines[1].left_text, "In Stock");
        let uncommon = merchant_tooltip(6272, "Pattern: Blue Linen Vest", 2, 1, None);
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
                spell_id: 184575,
                title: "Blade of Justice".into(),
                rank: "Rank 0/1".into(),
                description: "Pierce an enemy with a blade of light, dealing Holy damage and \
                              generating 1 Holy Power."
                    .into(),
            },
        );
        let tooltip =
            hovered_talent_tooltip(&registry, icon, Some(&tooltips)).expect("talent tooltip");
        assert_eq!(
            tooltip.anchor,
            TooltipAnchor::Owner {
                frame: node,
                side: OwnerSide::Right
            }
        );
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
        shared.insert(BuffFrameState::from_auras(&auras.auras, false));
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
