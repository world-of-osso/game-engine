//! Aura display (docs/specs/buff-frame.md): the local player's BuffFrame/DebuffFrame and the
//! TargetFrame auras from replicated `UnitAuras`. Remaining times count down from the
//! `remaining_ms` of each unit's last aura-set change; a timed aura that ran out is dropped
//! until the server's removal arrives. TargetFrame buttons get the reverse radial cooldown
//! swipe with its edge (`CooldownFrameTemplate`, `reverse`/`drawEdge`,
//! TargetFrameAuraButton.xml:21-25); BuffFrame buttons show duration text only.

use std::collections::HashMap;
use std::f32::consts::TAU;

use crate::replicated::UnitFields;
use shared::components::UnitAuras;
use std::time::Instant;

use game_engine_session::SessionScreen;
use game_engine_ui_model::aura_display_data::{AuraCasterLookup, AuraInstance, aura_instances};
use game_engine_ui_model::buff_frame_component::{
    BuffFrameState, aura_button_name, aura_warning_alpha, buff_frame_texture_fdids,
};
use game_engine_ui_model::inworld_unit_frames_component::class_bars::ClassBarView;
use game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use game_engine_ui_model::inworld_unit_frames_component::{
    TargetAuraIconState, TargetAuraView, UnitFrameState, set_target_auras,
};
use godot::classes::texture_progress_bar::FillMode;
use godot::classes::{
    Control, Image, ImageTexture, Texture2D, TextureProgressBar, TextureRect, control, image,
    texture_rect,
};
use godot::prelude::*;
use ui_toolkit::widgets::texture::TextureSource;

use crate::faction_reaction::{Reaction, reaction};
use crate::frame_error::FrameError;
use crate::{GameClient, ui::RegistryUi};

/// `CooldownFrameTemplate` swipe colour 0,0,0,0.64 (Cooldown.xml:3-11).
const SWIPE_COLOR: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.64);
/// `Interface\Cooldown\UI-HUD-ActionBar-SecondaryCooldown`, the template's edge texture.
const EDGE_FDID: u32 = 5_423_465;
/// Side of the solid swipe texture; the bar is scaled to the button.
const SWIPE_TEXTURE_SIZE: i32 = 64;
const SWIPE_NODE: &str = "Swipe";
const EDGE_NODE: &str = "Edge";

#[derive(Default)]
pub(crate) struct Auras {
    /// When each unit's replicated aura set last changed. Wall time: the server counts
    /// aura time in real seconds, whatever the frame rate.
    received: HashMap<u64, Instant>,
    /// Start of the flash clock.
    epoch: Option<Instant>,
    buff_ui: Option<Gd<RegistryUi>>,
    swipe: Option<Gd<ImageTexture>>,
    /// The loaded edge art; `Some(None)` caches art that failed to load.
    edge: Option<Option<Gd<Texture2D>>>,
    /// The TargetFrame's buff and debuff icons as last shown.
    target_icons: (Vec<TargetAuraIconState>, Vec<TargetAuraIconState>),
    /// The class bar's Cooldown swipe crop (a rune `-LevelBar`) as loaded.
    class_bar_swipe: Option<(AtlasArt, Gd<Texture2D>)>,
}

impl Auras {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.buff_ui {
            visit(ui)?;
        }
        Ok(())
    }

    /// A unit's replicated aura set arrived changed (or the unit left, with `None`).
    pub(crate) fn aura_set_changed(&mut self, unit: u64, present: bool) {
        if present {
            self.received.insert(unit, Instant::now());
        } else {
            self.received.remove(&unit);
        }
    }

    pub(crate) fn reset(&mut self) {
        self.received.clear();
        self.target_icons = Default::default();
        if let Some(ui) = self.buff_ui.take() {
            ui.free();
        }
    }

    fn swipe_texture(&mut self) -> Result<Gd<ImageTexture>, String> {
        if let Some(texture) = &self.swipe {
            return Ok(texture.clone());
        }
        let mut solid = Image::create(
            SWIPE_TEXTURE_SIZE,
            SWIPE_TEXTURE_SIZE,
            false,
            image::Format::RGBA8,
        )
        .ok_or("Create the cooldown swipe image")?;
        solid.fill(Color::WHITE);
        let texture =
            ImageTexture::create_from_image(&solid).ok_or("Create the cooldown swipe texture")?;
        self.swipe = Some(texture.clone());
        Ok(texture)
    }

    /// `art` cut from its atlas, loaded once per crop; `None` while its file loads.
    fn class_bar_swipe(
        &mut self,
        art: &AtlasArt,
        ui: &RegistryUi,
    ) -> Result<Option<Gd<Texture2D>>, String> {
        if let Some((loaded, texture)) = &self.class_bar_swipe
            && loaded == art
        {
            return Ok(Some(texture.clone()));
        }
        let registry = ui.registry().ok_or("Class bar swipe before the UI model")?;
        let Some((image, _)) =
            crate::ui::assets::load_source(&TextureSource::FileDataId(art.fdid), registry)?
        else {
            return Ok(None);
        };
        let (left, right, top, bottom) = art.rect;
        let scale = image.get_width() as f32 / art.atlas.0;
        let region = [
            left * scale,
            top * scale,
            (right - left) * scale,
            (bottom - top) * scale,
        ];
        let texture = crate::ui::assets::sub_texture(&image, region);
        self.class_bar_swipe = Some((*art, texture.clone()));
        Ok(Some(texture))
    }

    /// The cooldown edge; `None` while its file loads, or when it failed (reported).
    fn edge_texture(&mut self, ui: &RegistryUi) -> Option<Gd<Texture2D>> {
        if let Some(edge) = &self.edge {
            return edge.clone();
        }
        let registry = ui.registry()?;
        let edge =
            match crate::ui::assets::load_source(&TextureSource::FileDataId(EDGE_FDID), registry) {
                Ok(None) => return None,
                Ok(Some((texture, _))) => Some(texture),
                Err(error) => {
                    crate::frame_error::report_once(&format!("Cooldown edge: {error}"));
                    None
                }
            };
        self.edge = Some(edge.clone());
        edge
    }
}

/// `auras` `elapsed` seconds after they arrived, without the timed ones that ran out.
fn counted_down(mut auras: Vec<AuraInstance>, elapsed: f32) -> Vec<AuraInstance> {
    for aura in &mut auras {
        aura.remaining = (aura.remaining - elapsed).max(0.0);
    }
    auras.retain(|aura| aura.is_permanent() || aura.remaining > 0.0);
    auras
}

impl GameClient {
    /// Displayable auras of `unit` now.
    pub(crate) fn unit_auras(&self, unit: u64) -> Vec<AuraInstance> {
        let Some(views) = self
            .replica
            .unit(unit)
            .and_then(|unit| unit.get::<UnitAuras>())
        else {
            return Vec::new();
        };
        let name_of = |caster: u64| Some(self.replica.unit(caster)?.name()?.to_owned());
        let casters = AuraCasterLookup {
            local_player: self.world.local_player_id(),
            name_of: &name_of,
        };
        let auras = aura_instances(
            &views.auras,
            self.spells.catalog(),
            &casters,
            &Default::default(),
        );
        let elapsed = self
            .auras
            .received
            .get(&unit)
            .map_or(0.0, |arrived| arrived.elapsed().as_secs_f32());
        counted_down(auras, elapsed)
    }

    pub(crate) fn reaction_to(&mut self, unit: u64) -> Reaction {
        let target = self
            .replica
            .unit(unit)
            .and_then(UnitFields::faction_template);
        let viewer = self
            .world
            .local_player_id()
            .and_then(|player| self.replica.unit(player)?.faction_template());
        match self.nameplates.templates(&self.data_root) {
            Ok(templates) => reaction(
                target.and_then(|id| templates.get(&id)),
                viewer.and_then(|id| templates.get(&id)),
            ),
            Err(_) => Reaction::Neutral,
        }
    }

    /// TargetFrame aura rows of `target` as the local player sees them.
    pub(super) fn fill_target_auras(&mut self, state: &mut UnitFrameState, target: u64) {
        let player_is_target = self.world.local_player_id() == Some(target);
        let reaction = self.reaction_to(target);
        let npc = self
            .replica
            .unit(target)
            .is_some_and(|unit| unit.has::<shared::components::Npc>());
        let view = TargetAuraView {
            player_is_target,
            friendly: player_is_target || reaction == Reaction::Friendly,
            hostile_npc: npc && reaction == Reaction::Hostile,
        };
        let auras = self.unit_auras(target);
        set_target_auras(state, &auras, view);
        self.auras.target_icons = (state.target_buffs.clone(), state.target_debuffs.clone());
    }

    /// Swipe and edge over every TargetFrame aura button of `state`.
    pub(super) fn sync_target_aura_swipes(
        &mut self,
        state: Option<&UnitFrameState>,
    ) -> Result<(), String> {
        let Some(ui) = self.targeting.frame_ui().cloned() else {
            return Ok(());
        };
        let Some(state) = state else {
            self.auras.target_icons = Default::default();
            return Ok(());
        };
        let swipe = self.auras.swipe_texture()?;
        let edge = self.auras.edge_texture(&ui.bind());
        let groups = [
            ("TargetBuff", &state.target_buffs),
            ("TargetDebuff", &state.target_debuffs),
        ];
        for (prefix, icons) in groups {
            for (index, icon) in icons.iter().enumerate() {
                let name = format!("{prefix}Icon{index}Cooldown");
                if let Some(control) = ui.bind().frame_control(&name) {
                    sync_swipe(control, icon, &swipe, edge.as_ref());
                }
            }
        }
        Ok(())
    }

    /// Class bar Cooldown frames (RuneFrame.xml:83-92): a `reverse` swipe fills the
    /// spec's `-LevelBar` clockwise as the rune recharges. `prefix` names the bar's frames.
    pub(super) fn sync_class_bar_swipes(
        &mut self,
        prefix: &str,
        view: Option<&ClassBarView>,
    ) -> Result<(), String> {
        let Some(ui) = self.targeting.frame_ui().cloned() else {
            return Ok(());
        };
        for texture in view.into_iter().flat_map(|view| &view.textures) {
            let Some(progress) = texture.swipe else {
                continue;
            };
            let Some(control) = ui
                .bind()
                .frame_control(&format!("{prefix}{}", texture.name))
            else {
                continue;
            };
            // Drawn from the frame its file has arrived.
            let Some(art) = self.auras.class_bar_swipe(&texture.art, &ui.bind())? else {
                continue;
            };
            sync_class_bar_swipe(control, &art, progress);
        }
        Ok(())
    }

    /// The local player's BuffFrame and DebuffFrame, flashing auras under 31 s.
    pub(super) fn update_auras(&mut self) -> Result<(), FrameError> {
        let clock = self
            .auras
            .epoch
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f32();
        let local = self
            .world
            .local_player_id()
            .filter(|_| self.account.session.screen == SessionScreen::InWorld);
        let Some(local) = local else {
            if let Some(ui) = self.auras.buff_ui.take() {
                ui.free();
            }
            return Ok(());
        };
        let auras = self.unit_auras(local);
        let state =
            BuffFrameState::from_auras(&auras, self.client_options.graphics.colorblind_mode);
        self.extract_art(&buff_frame_texture_fdids(&state));
        let ui = match self.auras.buff_ui.as_mut() {
            Some(ui) => {
                ui.bind_mut().set_state(state)?;
                ui.clone()
            }
            None => {
                let mut ui = RegistryUi::new_alloc();
                ui.set_name("BuffFrameUI");
                self.base_mut().add_child(&ui);
                let shown = ui.bind_mut().show_buff_frame(state);
                if let Err(error) = shown {
                    ui.free();
                    return Err(error.into());
                }
                self.auras.buff_ui = Some(ui.clone());
                ui
            }
        };
        flash_expiring(&ui.bind(), &auras, clock);
        Ok(())
    }

    /// Aura state for automation: each shown button's spell, icon and screen rect.
    pub(super) fn auras_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let local = self.world.local_player_id();
        let auras = local.map(|id| self.unit_auras(id)).unwrap_or_default();
        let buttons = |debuffs: bool| {
            let mut list = VarArray::new();
            let shown = auras.iter().filter(|aura| aura.is_debuff == debuffs);
            for (index, aura) in shown.enumerate() {
                let name = aura_button_name(debuffs, index);
                let rect = self
                    .auras
                    .buff_ui
                    .as_ref()
                    .and_then(|ui| ui.bind().frame_rect(&format!("{name}Icon")));
                let entry = button_entry(&name, aura.spell_id, rect, &self.auras.buff_ui);
                list.push(&entry.to_variant());
            }
            list
        };
        state.set("buffs", &buttons(false));
        state.set("debuffs", &buttons(true));
        let target = self.targeting_target();
        let target_ui = self.targeting.frame_ui().cloned();
        let target_buttons = |prefix: &str, icons: Vec<TargetAuraIconState>| {
            let mut list = VarArray::new();
            for (index, icon) in icons.iter().enumerate() {
                let name = format!("{prefix}Icon{index}");
                let rect = target_ui
                    .as_ref()
                    .and_then(|ui| ui.bind().frame_rect(&format!("{name}Texture")));
                let mut entry = button_entry(&name, icon.spell_id, rect, &target_ui);
                entry.set("large", icon.large);
                entry.set("elapsed", icon.elapsed.unwrap_or(-1.0));
                entry.set(
                    "swipe",
                    target_ui
                        .as_ref()
                        .and_then(|ui| swipe_value(&ui.bind(), &name))
                        .unwrap_or(-1.0),
                );
                list.push(&entry.to_variant());
            }
            list
        };
        let (buffs, debuffs) = self.auras.target_icons.clone();
        state.set(
            "target",
            &target.map_or(Variant::nil(), |id| (id as i64).to_variant()),
        );
        state.set("target_buffs", &target_buttons("TargetBuff", buffs));
        state.set("target_debuffs", &target_buttons("TargetDebuff", debuffs));
        state
    }
}

fn button_entry(
    name: &str,
    spell_id: u32,
    rect: Option<([f32; 4], u32)>,
    ui: &Option<Gd<RegistryUi>>,
) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("name", name);
    entry.set("spell_id", i64::from(spell_id));
    let (rect, fdid) = rect.unwrap_or(([0.0; 4], 0));
    let rect: PackedFloat32Array = rect.into_iter().collect();
    entry.set("rect", &rect);
    entry.set("texture_fdid", i64::from(fdid));
    entry.set(
        "visible",
        ui.as_ref()
            .and_then(|ui| ui.bind().frame_control(name))
            .is_some_and(|control| control.is_visible_in_tree()),
    );
    entry
}

fn swipe_value(ui: &RegistryUi, name: &str) -> Option<f32> {
    let control = ui.frame_control(&format!("{name}Cooldown"))?;
    let bar = control
        .get_node_or_null(SWIPE_NODE)?
        .try_cast::<TextureProgressBar>()
        .ok()?;
    bar.is_visible_in_tree().then(|| bar.get_value() as f32)
}

/// `BUFF_WARNING_TIME` flash of each button (`AuraButtonMixin:UpdateAuraWarning`).
fn flash_expiring(ui: &RegistryUi, auras: &[AuraInstance], clock: f32) {
    for debuffs in [false, true] {
        let shown = auras.iter().filter(|aura| aura.is_debuff == debuffs);
        for (index, aura) in shown.enumerate() {
            let Some(mut control) = ui.frame_control(&aura_button_name(debuffs, index)) else {
                continue;
            };
            let time_left = (!aura.is_permanent()).then_some(aura.remaining);
            let alpha = aura_warning_alpha(clock, time_left);
            control.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, alpha));
        }
    }
}

/// The dark swipe covers the elapsed part clockwise from 12 o'clock; the edge marks its
/// leading side. Permanent auras have neither.
fn sync_class_bar_swipe(mut cooldown: Gd<Control>, art: &Gd<Texture2D>, progress: f32) {
    let mut bar = match cooldown.get_node_or_null(SWIPE_NODE) {
        Some(node) => node.cast::<TextureProgressBar>(),
        None => {
            let mut bar = TextureProgressBar::new_alloc();
            bar.set_name(SWIPE_NODE);
            bar.set_mouse_filter(control::MouseFilter::IGNORE);
            bar.set_fill_mode(FillMode::CLOCKWISE);
            bar.set_min(0.0);
            bar.set_max(1.0);
            bar.set_step(0.0);
            cooldown.add_child(&bar);
            bar
        }
    };
    bar.set_progress_texture(art);
    let native = art.get_size();
    bar.set_size(native);
    bar.set_scale(cooldown.get_size() / native);
    bar.set_value(f64::from(progress));
}

fn sync_swipe(
    mut cooldown: Gd<Control>,
    icon: &TargetAuraIconState,
    swipe: &Gd<ImageTexture>,
    edge: Option<&Gd<Texture2D>>,
) {
    let size = cooldown.get_size();
    let mut bar = match cooldown.get_node_or_null(SWIPE_NODE) {
        Some(node) => node.cast::<TextureProgressBar>(),
        None => {
            let mut bar = TextureProgressBar::new_alloc();
            bar.set_name(SWIPE_NODE);
            bar.set_mouse_filter(control::MouseFilter::IGNORE);
            bar.set_fill_mode(FillMode::CLOCKWISE);
            bar.set_progress_texture(swipe);
            bar.set_tint_progress(SWIPE_COLOR);
            bar.set_min(0.0);
            bar.set_max(1.0);
            bar.set_step(0.0);
            bar.set_size(Vector2::splat(SWIPE_TEXTURE_SIZE as f32));
            cooldown.add_child(&bar);
            bar
        }
    };
    let elapsed = icon.elapsed.unwrap_or(0.0);
    bar.set_scale(size / SWIPE_TEXTURE_SIZE as f32);
    bar.set_value(f64::from(elapsed));
    bar.set_visible(icon.elapsed.is_some());
    let Some(edge) = edge else { return };
    let mut hand = match cooldown.get_node_or_null(EDGE_NODE) {
        Some(node) => node.cast::<TextureRect>(),
        None => {
            let mut hand = TextureRect::new_alloc();
            hand.set_name(EDGE_NODE);
            hand.set_mouse_filter(control::MouseFilter::IGNORE);
            hand.set_texture(edge);
            hand.set_expand_mode(texture_rect::ExpandMode::IGNORE_SIZE);
            hand.set_stretch_mode(texture_rect::StretchMode::SCALE);
            cooldown.add_child(&hand);
            hand
        }
    };
    hand.set_size(size);
    hand.set_pivot_offset(size / 2.0);
    hand.set_rotation(elapsed * TAU);
    hand.set_visible(
        icon.elapsed
            .is_some_and(|elapsed| elapsed > 0.0 && elapsed < 1.0),
    );
}
