//! Action bar slot contents from [`ActionBarSlots`]: spell icons, cooldown wipe
//! and text (including the GCD), charge counts, power/range tints, key and click
//! casting, and drag and drop between the spellbook and the slots.

use bevy::prelude::*;
use game_engine::input_bindings::InputAction;
use game_engine::network_runtime::messages::MessageSenders;
use game_engine::player_spells::{
    ActionBarSlots, ActionDrag, CooldownTimer, DragSource, DraggedAction, SLOTS_PER_BAR,
    SpellChargeStates, SpellCooldowns, VISIBLE_BARS, action_button_name, parse_action_button_name,
    spell_cast_intent,
};
use game_engine::spell_catalog::{CatalogSpell, SpellCatalog, SpellCatalogData};
use game_engine::status::CharacterStatsSnapshot;
use game_engine::targeting::CurrentTarget;
use game_engine::ui::frame::{Dimension, WidgetData, WidgetType};
use game_engine::ui::game_plugin::SpellCastTarget;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::texture::{TextureData, TextureSource};
use shared::components::{Npc, PowerType, UnitPowers};
use shared::protocol::{ActionRef, CombatChannel, SetActionButton, SpellCastIntent, TalentChannel};

use super::action_bar::ActionBarsUi;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;

const VISIBLE_SLOTS: usize = VISIBLE_BARS * SLOTS_PER_BAR;
const SLOT_SIZE: f32 = 45.0;
/// Cursor travel (UI px) before a pressed slot becomes a drag.
const DRAG_THRESHOLD: f32 = 4.0;
const DRAG_ICON_NAME: &str = "ActionDragIcon";
const DRAG_ICON_SIZE: f32 = 36.0;
/// Server rule: a spell range max of 0 means 5 yd melee.
const MELEE_RANGE_YD: f32 = 5.0;

const TINT_NORMAL: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TINT_NO_POWER: [f32; 4] = [0.5, 0.5, 1.0, 1.0];
const TINT_OUT_OF_RANGE: [f32; 4] = [0.8, 0.1, 0.1, 1.0];

#[derive(Clone, Copy, Debug)]
struct SlotWidgets {
    icon: u64,
    cooldown: u64,
    cooldown_text: u64,
    count: u64,
}

/// Child frames of every visible slot and the view last written to them.
#[derive(Resource)]
struct ActionSlotWidgets {
    slots: Vec<SlotWidgets>,
    views: Vec<Option<SlotView>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SlotView {
    icon_fdid: u32,
    tint: [f32; 4],
    /// Cooldown wipe height in whole pixels.
    cooldown_px: u8,
    cooldown_text: String,
    count: String,
}

/// Per-frame inputs shared by every slot view.
struct SlotContext<'a> {
    catalog: Option<&'a SpellCatalogData>,
    cooldowns: &'a SpellCooldowns,
    charges: &'a SpellChargeStates,
    powers: Option<&'a UnitPowers>,
    base_mana: Option<f32>,
    /// Distance to the current target in yards, and whether it is an NPC.
    target: Option<(f32, bool)>,
}

pub(super) fn register_action_slot_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            resolve_slot_widgets,
            handle_action_slot_keys,
            handle_action_slot_pointer,
            sync_action_slot_views,
            sync_drag_icon,
        )
            .chain()
            .run_if(in_state(GameState::InWorld))
            .run_if(inworld_scene_stage_allows_ui),
    );
    app.add_systems(OnExit(GameState::InWorld), clear_action_slot_widgets);
}

fn resolve_slot_widgets(
    mut commands: Commands,
    ui: Res<UiState>,
    bars: Option<Res<ActionBarsUi>>,
    widgets: Option<Res<ActionSlotWidgets>>,
) {
    let Some(bars) = bars else { return };
    if widgets.is_some() && !bars.is_changed() {
        return;
    }
    let slots = (0..VISIBLE_SLOTS)
        .map(|slot| slot_widgets(&ui.registry, slot))
        .collect::<Option<Vec<_>>>();
    match slots {
        Some(slots) => commands.insert_resource(ActionSlotWidgets {
            views: vec![None; slots.len()],
            slots,
        }),
        None => error!("action bar slot children are missing; slot contents disabled"),
    }
}

fn slot_widgets(registry: &FrameRegistry, slot: usize) -> Option<SlotWidgets> {
    let button = action_button_name(slot);
    let child = |suffix: &str| registry.get_by_name(&format!("{button}{suffix}"));
    Some(SlotWidgets {
        icon: child("Icon")?,
        cooldown: child("Cooldown")?,
        cooldown_text: child("CooldownText")?,
        count: child("Count")?,
    })
}

fn clear_action_slot_widgets(mut commands: Commands, mut drag: ResMut<ActionDrag>) {
    commands.remove_resource::<ActionSlotWidgets>();
    drag.0 = None;
}

fn slot_action(index: usize) -> InputAction {
    const ACTIONS: [InputAction; SLOTS_PER_BAR] = [
        InputAction::ActionSlot1,
        InputAction::ActionSlot2,
        InputAction::ActionSlot3,
        InputAction::ActionSlot4,
        InputAction::ActionSlot5,
        InputAction::ActionSlot6,
        InputAction::ActionSlot7,
        InputAction::ActionSlot8,
        InputAction::ActionSlot9,
        InputAction::ActionSlot10,
        InputAction::ActionSlot11,
        InputAction::ActionSlot12,
    ];
    ACTIONS[index]
}

#[derive(bevy::ecs::system::SystemParam)]
struct SlotCaster<'w, 's> {
    catalog: Option<Res<'w, SpellCatalog>>,
    target: SpellCastTarget<'w>,
    senders: MessageSenders<'w, 's, SpellCastIntent>,
}

impl SlotCaster<'_, '_> {
    /// Sends a cast of the slot's spell at the current target.
    fn cast(&mut self, slots: &ActionBarSlots, slot: usize) {
        let Some(ActionRef::Spell(spell_id)) = slots.get(slot) else {
            return;
        };
        let name = self
            .catalog
            .as_deref()
            .and_then(|catalog| catalog.get(spell_id))
            .map_or("", |spell| &*spell.name);
        let intent = spell_cast_intent(spell_id, name, self.target.server_bits());
        for mut sender in self.senders.iter_mut() {
            sender.send::<CombatChannel>(intent.clone());
        }
    }
}

/// Keys 1–12 (World mode only) use bar 1.
fn handle_action_slot_keys(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    slots: Res<ActionBarSlots>,
    mut caster: SlotCaster,
) {
    for index in 0..SLOTS_PER_BAR {
        if keybinds.just_pressed(slot_action(index)) {
            caster.cast(&slots, index);
        }
    }
}

/// A left press on a slot that has not become a drag.
#[derive(Default)]
struct PendingPress {
    slot: Option<usize>,
    at: Vec2,
}

#[derive(bevy::ecs::system::SystemParam)]
struct SlotPointerInput<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
}

impl SlotPointerInput<'_, '_> {
    fn shift_held(&self) -> bool {
        self.keys.as_ref().is_some_and(|keys| {
            keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
        })
    }
}

/// Click casts; Shift-drag moves a slot (bars are otherwise locked); a drop on a
/// slot places or swaps, a drop elsewhere clears the dragged slot.
fn handle_action_slot_pointer(
    input: SlotPointerInput,
    ui: Res<UiState>,
    mut slots: ResMut<ActionBarSlots>,
    mut drag: ResMut<ActionDrag>,
    mut caster: SlotCaster,
    mut set_senders: MessageSenders<SetActionButton>,
    mut pending: Local<PendingPress>,
) {
    let (Ok(window), Some(mouse)) = (input.windows.single(), input.mouse.as_deref()) else {
        return;
    };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let hovered = hovered_slot(&ui.registry, cursor);
    if mouse.just_pressed(MouseButton::Left) {
        *pending = PendingPress {
            slot: hovered,
            at: cursor,
        };
    } else if mouse.pressed(MouseButton::Left)
        && drag.0.is_none()
        && let Some(slot) = pending.slot
        && pending.at.distance(cursor) >= DRAG_THRESHOLD
    {
        pending.slot = None;
        if input.shift_held()
            && let Some(action) = slots.get(slot)
        {
            drag.0 = Some(DraggedAction {
                action,
                source: DragSource::Slot(slot),
            });
        }
    }
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    if let Some(dragged) = drag.0.take() {
        for message in drop_action(&mut slots, dragged, hovered) {
            for mut sender in set_senders.iter_mut() {
                sender.send::<TalentChannel>(message.clone());
            }
        }
    } else if let Some(slot) = pending.slot.filter(|&slot| Some(slot) == hovered) {
        caster.cast(&slots, slot);
    }
    pending.slot = None;
}

fn hovered_slot(registry: &FrameRegistry, cursor: Vec2) -> Option<usize> {
    let mut frame_id = find_frame_at(registry, cursor.x, cursor.y)?;
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(slot) = frame.name.as_deref().and_then(parse_action_button_name) {
            return Some(slot);
        }
        frame_id = frame.parent_id?;
    }
}

/// Applies a drop locally (the server does not answer accepted requests) and
/// returns the requests that mirror it.
fn drop_action(
    slots: &mut ActionBarSlots,
    dragged: DraggedAction,
    target: Option<usize>,
) -> Vec<SetActionButton> {
    let changes = match (dragged.source, target) {
        (DragSource::Slot(source), Some(target)) if source == target => Vec::new(),
        (DragSource::Slot(source), Some(target)) => {
            vec![(target, Some(dragged.action)), (source, slots.get(target))]
        }
        (DragSource::Spellbook, Some(target)) => vec![(target, Some(dragged.action))],
        (DragSource::Slot(source), None) => vec![(source, None)],
        (DragSource::Spellbook, None) => Vec::new(),
    };
    changes
        .into_iter()
        .map(|(slot, action)| {
            slots.set(slot, action);
            SetActionButton {
                slot: slot as u8,
                action,
            }
        })
        .collect()
}

#[derive(bevy::ecs::system::SystemParam)]
struct SlotSources<'w, 's> {
    slots: Res<'w, ActionBarSlots>,
    catalog: Option<Res<'w, SpellCatalog>>,
    cooldowns: Res<'w, SpellCooldowns>,
    charges: Res<'w, SpellChargeStates>,
    stats: Option<Res<'w, CharacterStatsSnapshot>>,
    current_target: Option<Res<'w, CurrentTarget>>,
    local_player:
        Query<'w, 's, (&'static Transform, Option<&'static UnitPowers>), With<LocalPlayer>>,
    targets: Query<'w, 's, (&'static Transform, Has<Npc>)>,
}

impl SlotSources<'_, '_> {
    fn context(&self) -> SlotContext<'_> {
        let local = self.local_player.iter().next();
        let target = self
            .current_target
            .as_deref()
            .and_then(|target| target.0)
            .and_then(|entity| self.targets.get(entity).ok())
            .zip(local)
            .map(|((target, is_npc), (player, _))| {
                (player.translation.distance(target.translation), is_npc)
            });
        SlotContext {
            catalog: self.catalog.as_deref().and_then(SpellCatalog::data),
            cooldowns: &self.cooldowns,
            charges: &self.charges,
            powers: local.and_then(|(_, powers)| powers),
            base_mana: self.stats.as_deref().and_then(stats_base_mana),
            target,
        }
    }
}

fn stats_base_mana(stats: &CharacterStatsSnapshot) -> Option<f32> {
    let level = u8::try_from(stats.level?).ok()?;
    shared::formulas::base_mana(stats.class?, level).map(|mana| mana as f32)
}

fn sync_action_slot_views(
    mut ui: ResMut<UiState>,
    widgets: Option<ResMut<ActionSlotWidgets>>,
    sources: SlotSources,
) {
    let Some(mut widgets) = widgets else { return };
    let context = sources.context();
    let views: Vec<SlotView> = (0..widgets.slots.len())
        .map(|slot| slot_view(sources.slots.get(slot), &context))
        .collect();
    for (slot, view) in views.into_iter().enumerate() {
        if widgets.views[slot].as_ref() == Some(&view) {
            continue;
        }
        write_slot_view(&mut ui.registry, widgets.slots[slot], &view);
        widgets.views[slot] = Some(view);
    }
}

fn slot_view(action: Option<ActionRef>, context: &SlotContext) -> SlotView {
    let Some(ActionRef::Spell(spell_id)) = action else {
        return SlotView::default();
    };
    let spell = context.catalog.and_then(|catalog| catalog.get(spell_id));
    let charges = context.charges.get(spell_id);
    let charge_cooldown = charges
        .filter(|charges| charges.current == 0)
        .map(|charges| CooldownTimer {
            duration: charges.recharge,
            remaining: charges.remaining,
        });
    let uses_gcd = spell.is_none_or(|spell| spell.cooldown.gcd_ms > 0);
    let gcd = context.cooldowns.gcd().filter(|_| uses_gcd);
    let cooldown = [context.cooldowns.spell(spell_id), charge_cooldown, gcd]
        .into_iter()
        .flatten()
        .max_by(|a, b| a.remaining.total_cmp(&b.remaining));
    let is_gcd_only = cooldown.is_some() && cooldown == gcd && gcd.is_some();
    SlotView {
        icon_fdid: spell.map_or(0, |spell| spell.icon_fdid),
        tint: spell.map_or(TINT_NORMAL, |spell| slot_tint(spell, context)),
        cooldown_px: cooldown.map_or(0, cooldown_wipe_px),
        cooldown_text: cooldown
            .filter(|_| !is_gcd_only)
            .map(|timer| cooldown_text(timer.remaining))
            .unwrap_or_default(),
        count: charges
            .filter(|charges| charges.max > 1)
            .map(|charges| charges.current.to_string())
            .unwrap_or_default(),
    }
}

fn slot_tint(spell: &CatalogSpell, context: &SlotContext) -> [f32; 4] {
    if out_of_range(spell, context.target) {
        TINT_OUT_OF_RANGE
    } else if lacks_power(spell, context.powers, context.base_mana) {
        TINT_NO_POWER
    } else {
        TINT_NORMAL
    }
}

/// Hostile range for NPC targets, friendly otherwise. Self-only spells (both
/// maxima 0) never tint.
fn out_of_range(spell: &CatalogSpell, target: Option<(f32, bool)>) -> bool {
    let Some((distance, hostile)) = target else {
        return false;
    };
    let max = spell.range.max_yd;
    if max == [0.0, 0.0] {
        return false;
    }
    let max = if hostile { max[0] } else { max[1] };
    distance > if max > 0.0 { max } else { MELEE_RANGE_YD }
}

/// Unconditional costs in raw pool units (`ManaCost + PowerCostPct% × base`).
/// The mana base is gtBaseMP for the class and level; other pools use their max.
fn lacks_power(spell: &CatalogSpell, powers: Option<&UnitPowers>, base_mana: Option<f32>) -> bool {
    let Some(powers) = powers else {
        return false;
    };
    spell
        .powers
        .iter()
        .filter(|cost| cost.required_aura_spell_id == 0)
        .any(|cost| {
            let Some(power) = PowerType::from_db(cost.power_type.into()) else {
                return false;
            };
            let entry = powers.entries.iter().find(|entry| entry.power == power);
            let max = entry.map_or(0.0, |entry| entry.max as f32);
            let base = if power == PowerType::Mana {
                base_mana.unwrap_or(max)
            } else {
                max
            };
            let amount = cost.flat as f32 + cost.pct / 100.0 * base;
            amount > 0.0 && entry.is_none_or(|entry| (entry.current as f32) < amount)
        })
}

fn cooldown_wipe_px(timer: CooldownTimer) -> u8 {
    if timer.duration <= 0.0 {
        return 0;
    }
    let fraction = (timer.remaining / timer.duration).clamp(0.0, 1.0);
    (fraction * SLOT_SIZE).ceil() as u8
}

fn cooldown_text(remaining: f32) -> String {
    let secs = remaining.ceil() as u32;
    match secs {
        0 => String::new(),
        1..60 => secs.to_string(),
        60..3600 => format!("{}m", secs.div_ceil(60)),
        _ => format!("{}h", secs.div_ceil(3600)),
    }
}

fn write_slot_view(registry: &mut FrameRegistry, widgets: SlotWidgets, view: &SlotView) {
    if let Some(frame) = registry.get_mut(widgets.icon) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::FileDataId(view.icon_fdid),
            vertex_color: view.tint,
            ..Default::default()
        }));
    }
    registry.set_hidden(widgets.icon, view.icon_fdid == 0);
    let wipe = f32::from(view.cooldown_px);
    if let Some(frame) = registry.get_mut(widgets.cooldown) {
        frame.height = Dimension::Fixed(wipe);
    }
    let _ = registry.set_pos(widgets.cooldown, 0.0, SLOT_SIZE - wipe);
    registry.set_hidden(widgets.cooldown, view.cooldown_px == 0);
    set_text(registry, widgets.cooldown_text, &view.cooldown_text);
    set_text(registry, widgets.count, &view.count);
}

/// The HUD screen creates these as font strings; only their text changes.
fn set_text(registry: &mut FrameRegistry, id: u64, text: &str) {
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::FontString(data)) = &mut frame.widget_data
    {
        data.text = text.to_string();
    }
}

/// Icon following the cursor while an action is dragged.
fn sync_drag_icon(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    drag: Res<ActionDrag>,
    catalog: Option<Res<SpellCatalog>>,
) {
    let icon = drag.0.and_then(|dragged| match dragged.action {
        ActionRef::Spell(id) => catalog.as_deref()?.get(id).map(|spell| spell.icon_fdid),
        ActionRef::Item(_) | ActionRef::Macro(_) => None,
    });
    let existing = ui.registry.get_by_name(DRAG_ICON_NAME);
    let Some(fdid) = icon else {
        if let Some(id) = existing.filter(|&id| ui.registry.get(id).is_some_and(|f| !f.hidden)) {
            ui.registry.set_hidden(id, true);
        }
        return;
    };
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window));
    let Some(cursor) = cursor else { return };
    let id = existing.unwrap_or_else(|| create_drag_icon(&mut ui.registry));
    if let Some(frame) = ui.registry.get_mut(id) {
        frame.widget_data = Some(WidgetData::Texture(TextureData {
            source: TextureSource::FileDataId(fdid),
            ..Default::default()
        }));
    }
    let half = DRAG_ICON_SIZE * 0.5;
    let _ = ui.registry.set_pos(id, cursor.x - half, cursor.y - half);
    ui.registry.set_hidden(id, false);
}

fn create_drag_icon(registry: &mut FrameRegistry) -> u64 {
    let id = registry.create_frame(DRAG_ICON_NAME, None);
    if let Some(frame) = registry.get_mut(id) {
        frame.widget_type = WidgetType::Texture;
        frame.width = Dimension::Fixed(DRAG_ICON_SIZE);
        frame.height = Dimension::Fixed(DRAG_ICON_SIZE);
        frame.strata = FrameStrata::Tooltip;
        frame.mouse_enabled = false;
    }
    let _ = registry.set_pos_type(id, PositionType::Absolute);
    id
}

#[cfg(test)]
#[path = "action_bar_slots_tests.rs"]
mod tests;
