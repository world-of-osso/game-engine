//! Native Mainline vendor, one detached model scene, and actual server requests.
use crate::{
    GameClient,
    character_frame::preview::{Scene, bind_sheet_light},
    frame_error::FrameError,
    ui::RegistryUi,
    world_models::UnitAppearance,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::trading_post::{Action, FilterSubmenu, HEIGHT, MODEL, View, WIDTH};
use godot::classes::CanvasLayer;
use godot::prelude::*;
use shared::components::{EquipmentAppearance, EquippedAppearanceEntry, SheathState};
use shared::protocol::{EquipmentSlot, TradingPostRequest};

#[path = "trading_post_mount.rs"]
mod mount_preview;

fn contains_pointer([x, y, width, height]: [f32; 4], point: Vector2) -> bool {
    Rect2::new(Vector2::new(x, y), Vector2::new(width, height)).contains_point(point)
}

struct ProductDrag {
    id: u32,
    origin: Vector2,
    active: bool,
    from_frozen: bool,
}

#[derive(Default)]
pub(crate) struct TradingPost {
    pub view: View,
    ui: Option<Gd<RegistryUi>>,
    scene: Option<Scene>,
    pending: Option<(u64, UnitAppearance)>,
    shown: Option<UnitAppearance>,
    mount_product: Option<u32>,
    mount: Option<game_engine_core::mount_preview::MountPreview>,
    rider_pending: Option<(u64, UnitAppearance)>,
    rider_shown: Option<UnitAppearance>,
    rider_actor: Option<Gd<godot::classes::Node3D>>,
    rider_gap: String,
    yaw: f32,
    rotating: bool,
    dragged: Option<ProductDrag>,
    /// HUD layers hidden while shown: UIPanelWindows area "full" hides UIParent.
    hidden_layers: std::collections::HashSet<InstanceId>,
    preview_failed: bool,
    snapshot_observed_at: Option<std::time::Instant>,
    spinner_started_at: Option<std::time::Instant>,
}

/// The tooltip stays usable over the vendor, like Retail's own PerksProgramTooltip.
const KEPT_LAYERS: [&str; 2] = ["PerksProgramUI", "GameTooltipUI"];
impl TradingPost {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
}
impl GameClient {
    pub(super) fn open_trading_post(&mut self) -> Result<(), String> {
        self.merchant.session.close();
        self.account.trading_post = None;
        self.trading_post.view.snapshot = None;
        self.trading_post.view.visible = true;
        self.trading_post.view.hide_armor_setting = None;
        self.account
            .send_trading_post(TradingPostRequest::Query)
            .map_err(|error| error.to_string())
    }
    pub(super) fn close_trading_post(&mut self) -> bool {
        let visible = self.trading_post.view.visible;
        self.trading_post.view.visible = false;
        self.trading_post.view.clear_cart();
        self.clear_trading_model();
        for id in std::mem::take(&mut self.trading_post.hidden_layers) {
            // Other UI owners may have removed their layer while the vendor was open.
            if let Ok(mut layer) = Gd::<CanvasLayer>::try_from_instance_id(id) {
                layer.set_visible(true);
            }
        }
        if let Some(ui) = self.trading_post.ui.take() {
            ui.free();
        }
        self.trading_post.rotating = false;
        self.trading_post.dragged = None;
        visible
    }
    pub(super) fn close_trading_post_window(&mut self) -> Result<bool, FrameError> {
        if self.trading_post.view.confirm.is_some() {
            if !self.trading_post.view.pending {
                self.trading_post.view.confirm = None;
            }
            return Ok(true);
        }
        if self.trading_post.view.dropdown {
            self.trading_post.view.dropdown = false;
            self.trading_post.view.submenu = None;
            return Ok(true);
        }
        if self.trading_post.view.visible {
            if let Some(npc) = self
                .trading_post
                .view
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.npc_net_id)
            {
                self.account.send_close_interaction(npc)?;
            }
        }
        Ok(self.close_trading_post())
    }
    pub(super) fn update_trading_post(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_trading_post();
            return Ok(());
        }
        // The vendor waits for a whole catalog; a Log balance alone has no rows.
        if let Some(snapshot) = self
            .account
            .trading_post
            .as_ref()
            .filter(|snapshot| snapshot.catalog_complete)
        {
            let current = self.trading_post.view.snapshot.as_ref();
            if !current.is_some_and(|current| std::sync::Arc::ptr_eq(current, snapshot)) {
                self.trading_post.view.receive(snapshot.clone());
                self.trading_post.snapshot_observed_at = Some(std::time::Instant::now());
                if self.trading_post.view.visible && snapshot.npc_net_id.is_none() {
                    self.close_trading_post();
                }
            }
        }
        if !self.trading_post.view.visible {
            return Ok(());
        }
        if let Some(observed_at) = self.trading_post.snapshot_observed_at {
            self.trading_post.view.refund_elapsed =
                observed_at.elapsed().as_secs().min(u64::from(u32::MAX)) as u32;
        }
        self.trading_post.view.horde = self.player_is_horde();
        self.poll_trading_inputs()?;
        if self.trading_post.view.visible {
            self.hide_ui_parent();
            // Release the detached scene while its old UI host is still alive.
            let prepared = self.prepare_trading_model();
            let can_attach = prepared.is_ok();
            self.record_trading_preview(prepared);
            self.sync_trading_ui()?;
            self.animate_trading_spinner();
            self.animate_trading_rotation();
            if can_attach {
                let preview = self.sync_trading_model();
                self.record_trading_preview(preview);
            }
        }
        Ok(())
    }
    /// A failed preview must not leave the previous selection's model on screen; the
    /// next successful one restores the server's own error text.
    fn record_trading_preview(&mut self, preview: Result<(), String>) {
        match preview {
            Err(error) => {
                self.clear_trading_model();
                self.trading_post.view.error = error;
                self.trading_post.preview_failed = true;
            }
            Ok(()) if self.trading_post.preview_failed => {
                self.trading_post.preview_failed = false;
                let snapshot = self.trading_post.view.snapshot.as_ref();
                self.trading_post.view.error = snapshot
                    .and_then(|snapshot| snapshot.error.clone())
                    .unwrap_or_default();
            }
            Ok(()) => {}
        }
    }
    /// Hide every other visible client layer, including ones created while open.
    fn hide_ui_parent(&mut self) {
        for child in self.base().get_children().iter_shared() {
            let Ok(mut layer) = child.try_cast::<CanvasLayer>() else {
                continue;
            };
            let name = layer.get_name().to_string();
            if layer.is_visible() && !KEPT_LAYERS.contains(&name.as_str()) {
                layer.set_visible(false);
                self.trading_post.hidden_layers.insert(layer.instance_id());
            }
        }
    }
    fn poll_trading_inputs(&mut self) -> Result<(), String> {
        let Some(mut ui) = self.trading_post.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            let action = self.trading_post.view.click(&action)?;
            let closes_vendor = matches!(&action, Some(Action::Close));
            self.apply_trading_action(action)?;
            // Leave frees this RegistryUi; do not poll that input owner again.
            if closes_vendor {
                return Ok(());
            }
        }
        let pointer = self
            .base()
            .get_viewport()
            .ok_or("Trading Post viewport missing")?
            .get_mouse_position();
        self.trading_post.view.cart_hover =
            self.trading_post.view.cart_items().iter().find_map(|row| {
                let template = if row.category_id == 8 { "Set" } else { "Item" };
                let name = format!("PerksProgramCart{template}{}", row.id);
                [name.clone(), format!("{name}Remove")]
                    .iter()
                    .any(|name| {
                        ui.bind()
                            .frame_viewport_rect(name)
                            .is_some_and(|rect| contains_pointer(rect, pointer))
                    })
                    .then_some(row.id)
            });
        self.trading_post.view.cart_item_hover =
            ui.bind().hovered_button().and_then(|(name, _)| {
                if let Some(ids) = name.strip_prefix("PerksProgramCartSubItem") {
                    return ids.split_once('_')?.1.parse().ok();
                }
                let vendor: u32 = name.strip_prefix("PerksProgramCartItem")?.parse().ok()?;
                self.trading_post
                    .view
                    .cart_items()
                    .iter()
                    .find(|row| row.id == vendor)
                    .map(|row| row.item_id)
            });
        if self.trading_post.view.dropdown && self.trading_post.view.confirm.is_none() {
            if let Some((name, _)) = ui.bind().hovered_button() {
                match name.as_str() {
                    "PerksProgramFilterTypes" => {
                        self.trading_post.view.submenu = Some(FilterSubmenu::Types)
                    }
                    "PerksProgramFilterSort" => {
                        self.trading_post.view.submenu = Some(FilterSubmenu::Sort)
                    }
                    "PerksProgramFilterCollected"
                    | "PerksProgramFilterUncollected"
                    | "PerksProgramFilterUsable" => self.trading_post.view.submenu = None,
                    _ => {}
                }
            }
        }
        Ok(())
    }
    fn apply_trading_action(&mut self, action: Option<Action>) -> Result<(), String> {
        match action {
            Some(Action::Request(request)) => {
                if let Err(error) = self.account.send_trading_post(request) {
                    self.trading_post.view.pending = false;
                    return Err(error.to_string());
                }
            }
            Some(Action::Close) => {
                self.close_trading_post_window()
                    .map_err(|error| error.to_string())?;
            }
            None => {}
        }
        Ok(())
    }
    fn animate_trading_spinner(&mut self) {
        if !self.trading_post.view.pending
            || self.trading_post.view.confirm.as_deref() != Some("cart-checkout")
        {
            self.trading_post.spinner_started_at = None;
            return;
        }
        let started = self
            .trading_post
            .spinner_started_at
            .get_or_insert_with(std::time::Instant::now);
        // SpinnerTemplate: both textures rotate -360 degrees every two seconds.
        let rotation = started.elapsed().as_secs_f32() * std::f32::consts::PI;
        let Some(ui) = &self.trading_post.ui else {
            return;
        };
        if let Some(mut spinner) = ui.bind().frame_control("PerksProgramConfirmSpinner") {
            spinner.set_pivot_offset(Vector2::new(30., 30.));
            spinner.set_rotation(rotation);
        }
    }

    fn animate_trading_rotation(&mut self) {
        let Some(ui) = &self.trading_post.ui else {
            return;
        };
        let pressed = if self.trading_post.view.confirm.is_none() {
            ui.bind().pushed_button()
        } else {
            None
        };
        // ModelSceneMixin.OnUpdate/PerksProgram.xml:0.1 radians per update, not per click.
        for (name, increment) in [
            ("PerksProgramRotateLeftButton", -0.1),
            ("PerksProgramRotateRightButton", 0.1),
        ] {
            let down = pressed.as_deref() == Some(name);
            if down {
                self.trading_post.yaw += increment;
            }
        }
    }

    fn sync_trading_ui(&mut self) -> Result<(), String> {
        let size = self
            .base()
            .get_viewport()
            .ok_or("Trading Post viewport missing")?
            .get_visible_rect()
            .size;
        let scale = (size.x / WIDTH).min(size.y / HEIGHT).min(0.75);
        let view = self.trading_post.view.clone();
        if let Some(ui) = &mut self.trading_post.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("PerksProgramUI");
        ui.set_layer(8);
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_trading_post(view))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.trading_post.ui = Some(ui);
        Ok(())
    }
    fn desired_trading_appearance(&self) -> Result<Option<UnitAppearance>, String> {
        let Some(row) = self.trading_post.view.preview_product() else {
            return Ok(None);
        };
        // Unavailable rows show their source gap in the details text instead of a model.
        if row.category_id == 5 || row.grant_unavailable.is_some() {
            return Ok(None);
        }
        if row.creature_display_id != 0 {
            return Ok(Some(UnitAppearance::Creature {
                display_id: row.creature_display_id,
                items: Default::default(),
            }));
        }
        let Some(UnitAppearance::Player(player, equipped, tabard)) =
            self.world.local_player_appearance()
        else {
            return Ok(None);
        };
        let catalog = game_engine_ui_model::item_catalog::item_catalog()
            .ok_or("Trading Post Retail item catalog still loading")?;
        let mut equipment = if self.trading_post.view.hide_armor()? {
            EquipmentAppearance::default()
        } else {
            equipped.clone()
        };
        let preview_items = if self.trading_post.view.cart_visible {
            self.trading_post.view.cart_preview_item_ids()
        } else {
            row.preview_item_ids.clone()
        };
        for id in &preview_items {
            let item = catalog
                .get(*id)
                .ok_or_else(|| format!("Trading Post preview item{id} metadata unavailable"))?;
            let slot = EquipmentSlot::for_inventory_type(item.inventory_type)
                .iter()
                .find_map(|slot| slot.visual_slot())
                .ok_or_else(|| format!("Trading Post preview item{id} has no visual slot"))?;
            equipment.entries.retain(|entry| entry.slot != slot);
            equipment.entries.push(EquippedAppearanceEntry {
                slot,
                item_id: Some(*id),
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                display_info_id: None,
                inventory_type: item.inventory_type,
                hidden: false,
            });
        }
        Ok(Some(UnitAppearance::Player(
            player.clone(),
            equipment,
            *tabard,
        )))
    }
    fn clear_trading_rider(&mut self) {
        if let Some((id, _)) = self.trading_post.rider_pending.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(rider) = self.trading_post.rider_actor.take() {
            rider.free();
        }
        self.trading_post.rider_shown = None;
        self.trading_post.rider_gap.clear();
    }
    fn clear_trading_model(&mut self) {
        self.clear_trading_rider();
        self.trading_post.mount = None;
        self.trading_post.mount_product = None;
        if let Some((id, _)) = self.trading_post.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.trading_post.scene.take() {
            scene.free();
        }
        self.trading_post.shown = None;
    }
    fn prepare_trading_model(&mut self) -> Result<(), String> {
        let desired = self.desired_trading_appearance()?;
        let current = self
            .trading_post
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(self.trading_post.shown.as_ref());
        let mount_product = self
            .trading_post
            .view
            .preview_product()
            .filter(|row| row.category_id == 2 && desired.is_some())
            .map(|row| row.id);
        if desired.as_ref() != current || mount_product != self.trading_post.mount_product {
            self.clear_trading_model();
            if let Some(product) = mount_product {
                self.trading_post.mount = Some(self.read_trading_mount()?);
                self.trading_post.mount_product = Some(product);
            }
            if let Some(appearance) = desired {
                let id = self.world.request_detached_visual(&appearance);
                self.trading_post.pending = Some((id, appearance));
            }
        }
        Ok(())
    }
    fn read_trading_mount(&self) -> Result<game_engine_core::mount_preview::MountPreview, String> {
        let view = &self.trading_post.view;
        let row = view
            .preview_product()
            .ok_or("Trading Post mount selection missing")?;
        let journal = self
            .mount_journal
            .view
            .catalog
            .values()
            .find(|mount| mount.teaching_items.contains(&row.item_id))
            .ok_or_else(|| {
                format!(
                    "Trading Post mount item{} metadata still loading",
                    row.item_id
                )
            })?;
        let category_scene = view
            .snapshot
            .as_ref()
            .and_then(|snapshot| {
                snapshot
                    .categories
                    .iter()
                    .find(|category| category.id == row.category_id)
            })
            .map(|category| category.model_scene_id)
            .ok_or("Trading Post mount category scene missing")?;
        let mut mount = game_engine_core::mount_preview::MountPreview::read(
            &self.data_root,
            journal.mount_id,
            category_scene,
        )?;
        mount.apply_display_scene(&self.data_root, view.preview_scene_id()?)?;
        Ok(mount)
    }
    fn sync_trading_model(&mut self) -> Result<(), String> {
        if self.trading_post.pending.is_none() && self.trading_post.shown.is_none() {
            return Ok(());
        }
        if self.trading_post.scene.is_none() {
            let host = self
                .trading_post
                .ui
                .as_ref()
                .ok_or("Trading Post UI missing")?
                .bind()
                .frame_control(MODEL)
                .ok_or("Trading Post model host missing")?;
            self.trading_post.scene = Some(Scene::new(host));
        }
        self.attach_trading_model()?;
        self.sync_trading_rider()?;
        let scene = self
            .trading_post
            .scene
            .as_mut()
            .expect("Trading Post scene created");
        scene.fit_viewport();
        scene.set_model_yaw(self.trading_post.yaw);
        Ok(())
    }
    fn attach_trading_model(&mut self) -> Result<(), String> {
        let Some((id, _)) = &self.trading_post.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(*id) else {
            return Ok(());
        };
        let (_, appearance) = self
            .trading_post
            .pending
            .take()
            .expect("pending Trading Post visual");
        let model = loaded?;
        if matches!(&appearance, UnitAppearance::Player(..)) {
            if let Err(error) =
                self.world
                    .place_player_weapons(&model, &appearance, SheathState::Melee)
            {
                model.free();
                return Err(error);
            }
        }
        bind_sheet_light(&model);
        let model = if let Some(mount) = &self.trading_post.mount {
            let animation = if mount.is_self_mount() {
                mount_preview::SELF_MOUNT_ANIMATION
            } else {
                mount.actors["mount"].animation
            };
            let prepared = model
                .try_get_node_as::<crate::animation::WowAnimationPlayer>("NpcModel/M2Animation")
                .ok_or_else(|| "Trading Post mount animation owner missing".to_string())
                .and_then(|mut player| player.bind_mut().play_clip(animation, true))
                .and_then(|()| mount_preview::compose_mount(model.clone(), &mount.actors["mount"]));
            match prepared {
                Ok(group) => group,
                Err(error) => {
                    model.free();
                    return Err(error);
                }
            }
        } else {
            model
        };
        self.trading_post
            .scene
            .as_mut()
            .ok_or("Trading Post model scene missing")?
            .replace_model(model);
        self.trading_post.shown = Some(appearance);
        Ok(())
    }
    fn desired_trading_rider(&self) -> Option<UnitAppearance> {
        let mut appearance = self.world.local_player_appearance()?.clone();
        let UnitAppearance::Player(_, equipped, _) = &mut appearance else {
            return None;
        };
        // SharedXML AttachPlayerToMount: autoDress=true, hideWeapons=true.
        equipped.entries.retain(|entry| {
            !matches!(
                entry.slot,
                shared::components::EquipmentVisualSlot::MainHand
                    | shared::components::EquipmentVisualSlot::OffHand
                    | shared::components::EquipmentVisualSlot::Ranged
            )
        });
        Some(appearance)
    }
    fn sync_trading_rider(&mut self) -> Result<(), String> {
        let Some(mount) = self.trading_post.mount.clone() else {
            return Ok(());
        };
        if !mount.shows_rider(self.trading_post.view.hide_player_preview) {
            self.clear_trading_rider();
            return Ok(());
        }
        if mount.rider_animation_kit != 0 || mount.rider_spell_visual_kit != 0 {
            self.clear_trading_rider();
            self.trading_post.rider_gap = format!(
                "Rider animation kit{} / spell visual kit{} not implemented",
                mount.rider_animation_kit, mount.rider_spell_visual_kit
            );
            return Ok(());
        }
        self.request_trading_rider();
        self.attach_pending_trading_rider(&mount)
    }
    fn request_trading_rider(&mut self) {
        let desired = self.desired_trading_rider();
        let current = self
            .trading_post
            .rider_pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(self.trading_post.rider_shown.as_ref());
        if desired.as_ref() != current {
            self.clear_trading_rider();
            if let Some(appearance) = desired {
                let id = self.world.request_detached_visual(&appearance);
                self.trading_post.rider_pending = Some((id, appearance));
            }
        }
    }
    fn attach_pending_trading_rider(
        &mut self,
        mount: &game_engine_core::mount_preview::MountPreview,
    ) -> Result<(), String> {
        let Some(group) = self.trading_post.scene.as_ref().and_then(Scene::model) else {
            return Ok(());
        };
        let Some((id, _)) = self.trading_post.rider_pending.as_ref() else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(*id) else {
            return Ok(());
        };
        let (_, appearance) = self
            .trading_post
            .rider_pending
            .take()
            .expect("pending rider");
        let rider = loaded?;
        let attached = self.attach_trading_rider(&group, &rider, mount);
        match attached {
            Ok(actor) => {
                self.trading_post.rider_actor = Some(actor);
                self.trading_post.rider_shown = Some(appearance);
            }
            Err(error) => {
                rider.free();
                return Err(error);
            }
        }
        Ok(())
    }
    fn attach_trading_rider(
        &self,
        group: &Gd<godot::classes::Node3D>,
        rider: &Gd<godot::classes::Node3D>,
        mount: &game_engine_core::mount_preview::MountPreview,
    ) -> Result<Gd<godot::classes::Node3D>, String> {
        let mut animation = rider
            .try_get_node_as::<crate::animation::WowAnimationPlayer>("M2Animation")
            .ok_or("Trading Post rider animation owner missing")?;
        animation
            .bind_mut()
            .play_clip(mount_preview::RIDER_ANIMATION, true)?;
        bind_sheet_light(rider);
        let model = group
            .try_get_node_as::<godot::classes::Node3D>("MountActor/NpcVisualRoot")
            .ok_or("Trading Post mount actor missing")?;
        // CalculateMountScale's native formula is not established. Lead-authorized
        // scope retains the authored player-rider scale1; no guessed compensation.
        mount_preview::attach_rider(&model, rider.clone(), &mount.actors["player-rider"])
    }
    pub(super) fn trading_post_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if !self.trading_post.view.visible {
            return false;
        }
        self.close_trading_filter_on_outside_press(event);
        if let Some(ui) = &mut self.trading_post.ui {
            let scrolled = ui.bind_mut().scroll_list_input(event);
            match scrolled {
                Ok(true) => {
                    self.mark_viewport_input_handled();
                    return true;
                }
                Ok(false) => {}
                Err(error) => {
                    self.handle_frame_error("Trading Post scroll", error.into());
                    return true;
                }
            }
        }
        self.trading_product_pointer(event) || self.trading_model_pointer(event)
    }
    fn close_trading_filter_on_outside_press(&mut self, event: &Gd<godot::classes::InputEvent>) {
        if !self.trading_post.view.dropdown || self.trading_post.view.confirm.is_some() {
            return;
        }
        let Ok(button) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return;
        };
        if !button.is_pressed() {
            return;
        }
        let inside = self.trading_post.ui.as_ref().is_some_and(|ui| {
            [
                "PerksProgramFilter",
                "PerksProgramFilterResetButton",
                "PerksProgramFilterMenu",
                "PerksProgramTypeMenu",
                "PerksProgramSortMenu",
            ]
            .iter()
            .any(|name| {
                ui.bind()
                    .frame_viewport_rect(name)
                    .is_some_and(|rect| contains_pointer(rect, button.get_position()))
            })
        });
        if !inside {
            self.trading_post.view.dropdown = false;
            self.trading_post.view.submenu = None;
        }
    }

    fn trading_product_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if self.trading_post.view.confirm.is_some() || self.trading_post.view.pending {
            self.trading_post.dragged = None;
            return false;
        }
        if let Ok(motion) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseMotion>()
        {
            if let Some(drag) = &mut self.trading_post.dragged {
                drag.active |= drag.origin.distance_squared_to(motion.get_position()) > 16.;
                return drag.active;
            }
            return false;
        }
        let Ok(button) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return false;
        };
        if button.get_button_index() != godot::global::MouseButton::LEFT {
            return false;
        }
        let Some(ui) = self.trading_post.ui.as_ref() else {
            return false;
        };
        if button.is_pressed() {
            if let Some(id) = self
                .trading_post
                .view
                .snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.frozen_item_id)
                && ui
                    .bind()
                    .frame_viewport_rect("PerksProgramFrozenSlot")
                    .is_some_and(|rect| contains_pointer(rect, button.get_position()))
            {
                self.trading_post.dragged = Some(ProductDrag {
                    id,
                    origin: button.get_position(),
                    active: false,
                    from_frozen: true,
                });
                return false;
            }
            for row in self.trading_post.view.rows() {
                if row.grant_unavailable.is_none()
                    && ui
                        .bind()
                        .frame_viewport_rect(&format!("PerksProgramProduct{}", row.id))
                        .is_some_and(|rect| contains_pointer(rect, button.get_position()))
                {
                    self.trading_post.dragged = Some(ProductDrag {
                        id: row.id,
                        origin: button.get_position(),
                        active: false,
                        from_frozen: false,
                    });
                    break;
                }
            }
            return false;
        }
        let Some(drag) = self.trading_post.dragged.take() else {
            return false;
        };
        if !drag.active {
            return false;
        }
        let dropped = ui
            .bind()
            .frame_viewport_rect("PerksProgramFrozenSlot")
            .is_some_and(|rect| contains_pointer(rect, button.get_position()));
        if let Some(Action::Request(request)) =
            self.trading_post
                .view
                .drop_item(drag.id, drag.from_frozen, dropped)
        {
            if let Err(error) = self.account.send_trading_post(request) {
                self.trading_post.view.pending = false;
                self.trading_post.view.error = error.to_string();
            }
        }
        true
    }
    fn trading_model_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if self.trading_post.view.confirm.is_some() {
            return false;
        }
        if let Ok(motion) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseMotion>()
        {
            if self.trading_post.rotating {
                self.trading_post.yaw += motion.get_relative().x * 0.01;
                return true;
            }
            return false;
        }
        let Ok(button) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return false;
        };
        if button.get_button_index() != godot::global::MouseButton::LEFT {
            return false;
        }
        if !button.is_pressed() && self.trading_post.rotating {
            self.trading_post.rotating = false;
            return true;
        }
        let bounds = self
            .trading_post
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_viewport_rect(MODEL));
        let over_ui = self.trading_post.ui.as_ref().is_some_and(|ui| {
            [
                "PerksProgramProductsScrollBoxContainer",
                "PerksProgramFilter",
                "PerksProgramFilterResetButton",
                "PerksProgramFilterMenu",
                "PerksProgramTypeMenu",
                "PerksProgramSortMenu",
                "PerksProgramCurrencyFrame",
                "PerksProgramProductDetailsContainerFrame",
                "PerksProgramFooterFrame",
            ]
            .iter()
            .any(|name| {
                ui.bind()
                    .frame_viewport_rect(name)
                    .is_some_and(|rect| contains_pointer(rect, button.get_position()))
            })
        });
        if over_ui {
            return false;
        }
        if let Some([x, y, width, height]) = bounds {
            if Rect2::new(Vector2::new(x, y), Vector2::new(width, height))
                .contains_point(button.get_position())
            {
                self.trading_post.rotating = button.is_pressed();
                return true;
            }
        }
        false
    }
    pub(super) fn trading_post_snapshot(&self) -> VarDictionary {
        let view = &self.trading_post.view;
        let mut state = VarDictionary::new();
        state.set("open", view.visible);
        state.set("selected", view.selected.map_or(-1, i64::from));
        state.set("model_yaw", self.trading_post.yaw);
        let enabled_types: Vec<_> = view
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.categories)
            .filter(|category| !view.filters.excluded_types.contains(&category.id))
            .map(|category| category.id)
            .collect();
        state.set(
            "category",
            if enabled_types.len() == 1 {
                i64::from(enabled_types[0])
            } else {
                0
            },
        );
        let types: PackedStringArray = view
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.categories)
            .map(|category| {
                GString::from(&format!(
                    "{}|{}",
                    category.id,
                    u8::from(!view.filters.excluded_types.contains(&category.id))
                ))
            })
            .collect();
        state.set("types", &types);
        state.set("filter_collected", view.filters.collected);
        state.set("filter_uncollected", view.filters.uncollected);
        state.set("filter_usable", view.filters.usable_only);
        state.set(
            "sort",
            match view.sort {
                game_engine_ui_model::trading_post::Sort::Name => "name",
                game_engine_ui_model::trading_post::Sort::Time => "time",
                game_engine_ui_model::trading_post::Sort::Price => "price",
            },
        );
        state.set("ascending", view.ascending);
        state.set("preview_ready", self.trading_post.shown.is_some());
        state.set("hide_player_preview", view.hide_player_preview);
        state.set("rider_ready", self.trading_post.rider_shown.is_some());
        state.set("rider_pending", self.trading_post.rider_pending.is_some());
        state.set("rider_gap", self.trading_post.rider_gap.as_str());
        state.set("rider_scale_gap", self.trading_post.mount.is_some());
        state.set(
            "rider_actor_id",
            self.trading_post
                .rider_actor
                .as_ref()
                .map_or(0, |node| node.instance_id().to_i64()),
        );
        state.set(
            "preview_kind",
            match &self.trading_post.shown {
                Some(UnitAppearance::Player(..)) => "player",
                Some(UnitAppearance::Creature { .. }) => "creature",
                None => "none",
            },
        );
        let rows: PackedStringArray = view
            .rows()
            .iter()
            .take(12)
            .map(|row| {
                GString::from(&format!(
                    "{}|{}",
                    row.id,
                    u8::from(row.grant_unavailable.is_none())
                ))
            })
            .collect();
        state.set("rows", &rows);
        state.set("pending", view.pending);
        state.set("cart_visible", view.cart_visible);
        state.set(
            "cart",
            &view
                .cart
                .iter()
                .map(|id| i64::from(*id))
                .collect::<PackedInt64Array>(),
        );
        state.set(
            "cart_preview_items",
            &view
                .cart_preview_item_ids()
                .iter()
                .map(|id| i64::from(*id))
                .collect::<PackedInt64Array>(),
        );
        state.set("cart_price", view.cart_price() as i64);
        match view.hide_armor() {
            Ok(hide) => state.set("hide_armor", hide),
            Err(error) => state.set("outfit_error", error.as_str()),
        }
        state.set(
            "hidden_layers",
            self.trading_post.hidden_layers.len() as i64,
        );
        state.set("error", view.error.as_str());
        if let Some(snapshot) = &view.snapshot {
            state.set("tender", i64::from(snapshot.tender));
            state.set("points", i64::from(snapshot.points));
            state.set("frozen", snapshot.frozen_item_id.map_or(-1, i64::from));
            state.set("items", snapshot.items.len() as i64);
            if let Some(row) = view.selected_item() {
                state.set("selected_name", row.name.as_str());
                state.set("item_id", i64::from(row.item_id));
                state.set(
                    "preview_item_ids",
                    &row.preview_item_ids
                        .iter()
                        .map(|id| i64::from(*id))
                        .collect::<PackedInt64Array>(),
                );
                state.set("price", i64::from(row.price));
                state.set("purchased", row.purchased);
                state.set("collected", row.collected);
                state.set("refund_seconds", i64::from(row.refund_seconds));
            }
        }
        state
    }
}
