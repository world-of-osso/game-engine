//! Retail HousingModelPreview: receipt-backed isolated model, never a placed instance.
//! Native framing/drag sensitivity are input tuning, not certified UiModelScene presets.
use crate::GameClient;
use crate::character_frame::preview::{Scene, bind_sheet_light};
use game_engine_core::housing::HousingCatalog;
use game_engine_ui_housing::housing::{DashboardTab, HousingSurface};
use godot::classes::{Control, InputEvent, InputEventMouseButton, InputEventMouseMotion, Label};
use godot::global::MouseButton;
use godot::prelude::*;
use std::path::Path;

const HOST: &str = "HousingCatalogModelScene";
const DRAG_RADIANS_PER_PIXEL: f32 = 0.01;

#[derive(Default)]
pub(crate) struct CatalogPreview {
    scene: Option<Scene>,
    error_label: Option<Gd<Label>>,
    attempted: Option<u32>,
    yaw: f32,
    bounds: Option<Aabb>,
    dragging: bool,
}

impl CatalogPreview {
    pub(crate) fn reset(&mut self) {
        if let Some(scene) = self.scene.take() {
            scene.free();
        }
        if let Some(label) = self.error_label.take() {
            label.free();
        }
        self.attempted = None;
        self.yaw = 0.0;
        self.bounds = None;
        self.dragging = false;
    }

    fn load_selection(
        &mut self,
        host: Gd<Control>,
        data: &Path,
        catalog: &HousingCatalog,
        decor_id: u32,
    ) -> Result<(), String> {
        if self.attempted == Some(decor_id) {
            if let Some(scene) = &mut self.scene {
                scene.fit_viewport();
                frame_catalog_bounds(scene, self.bounds.expect("installed catalog bounds"));
            }
            return Ok(());
        }
        self.reset();
        self.attempted = Some(decor_id);
        match load_catalog_actor(data, catalog, decor_id) {
            Ok((visual, bounds)) => {
                self.scene = Some(mount_catalog_actor(host, visual, bounds));
                self.bounds = Some(bounds);
                Ok(())
            }
            Err(error) => {
                self.error_label = Some(mount_unavailable_preview(host, &error));
                Err(error)
            }
        }
    }

    fn rotate(&mut self, yaw: f32) {
        self.yaw = yaw;
        if let Some(scene) = &mut self.scene {
            scene.set_model_yaw(yaw);
            frame_catalog_bounds(scene, self.bounds.expect("installed catalog bounds"));
        }
    }

    fn pointer(&mut self, event: &Gd<InputEvent>, bounds: Rect2) -> bool {
        if self.scene.is_none() {
            return false;
        }
        if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            if button.get_button_index() != MouseButton::LEFT {
                return false;
            }
            if button.is_pressed() {
                self.dragging = bounds.contains_point(button.get_position());
                return self.dragging;
            }
            return std::mem::take(&mut self.dragging);
        }
        if self.dragging
            && let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>()
        {
            self.rotate(self.yaw + motion.get_relative().x * DRAG_RADIANS_PER_PIXEL);
            return true;
        }
        false
    }
}

fn mount_catalog_actor(host: Gd<Control>, visual: Gd<Node3D>, bounds: Aabb) -> Scene {
    let mut pivot = Node3D::new_alloc();
    pivot.set_name("HousingCatalogPivot");
    pivot.add_child(&visual);
    let mut scene = Scene::new(host);
    scene.replace_model(pivot);
    frame_catalog_bounds(&mut scene, bounds);
    scene
}

fn mount_unavailable_preview(mut host: Gd<Control>, error: &str) -> Gd<Label> {
    let mut label = Label::new_alloc();
    label.set_name("HousingCatalogPreviewUnavailable");
    label.set_text("Preview unavailable");
    label.set_tooltip_text(error);
    label.set_horizontal_alignment(godot::global::HorizontalAlignment::CENTER);
    label.set_vertical_alignment(godot::global::VerticalAlignment::CENTER);
    host.add_child(&label);
    label.set_anchors_and_offsets_preset(godot::classes::control::LayoutPreset::FULL_RECT);
    label
}

fn load_catalog_actor(
    data: &Path,
    catalog: &HousingCatalog,
    decor_id: u32,
) -> Result<(Gd<Node3D>, Aabb), String> {
    let decor = catalog
        .decor
        .iter()
        .find(|decor| decor.id == decor_id)
        .ok_or_else(|| format!("Decor {decor_id} is not in the authenticated catalog."))?;
    let (mut visual, bounds) = match decor.model_type {
        1 => load_m2_preview(data, catalog, decor_id)?,
        2 => load_wmo_preview(data, decor.model_fdid)?,
        model_type => {
            return Err(format!(
                "Decor {decor_id} model type {model_type} is not supported."
            ));
        }
    };
    visual.set_name("HousingCatalogActor");
    visual.set_meta("housing_decor_id", &(decor_id as i64).to_variant());
    visual.set_meta(
        "model_file_data_id",
        &(decor.model_fdid as i64).to_variant(),
    );
    visual.set_rotation(Vector3::from_array(decor.initial_rotation));
    visual.set_scale(Vector3::ONE * decor.initial_scale);
    bind_sheet_light(&visual);
    let bounds = visual.get_transform() * bounds;
    Ok((visual, bounds))
}

fn load_m2_preview(
    data: &Path,
    catalog: &HousingCatalog,
    decor_id: u32,
) -> Result<(Gd<Node3D>, Aabb), String> {
    let visual = crate::housing_decor::load_visual(data, catalog, decor_id)?;
    // Retail SetPreferModelCollisionBounds(true): frame the solid body rather than
    // its animation/particle envelope. Models without collision retain header bounds.
    let bounds_key = if visual.has_meta(crate::assets::M2_COLLISION_BOUNDS_META) {
        crate::assets::M2_COLLISION_BOUNDS_META
    } else {
        crate::assets::M2_BOUNDS_META
    };
    let bounds = visual.get_meta(bounds_key).to::<Aabb>();
    Ok((visual, bounds))
}

fn load_wmo_preview(data: &Path, fdid: u32) -> Result<(Gd<Node3D>, Aabb), String> {
    let resolver = crate::assets::creature::local_resolver(data);
    let asset = crate::wmo::assets::read_wmo(&resolver, data, fdid)?;
    let bounds = crate::assets::wow_aabb(asset.root.bbox_min, asset.root.bbox_max);
    let built = crate::wmo::scene::build_wmo_node(&asset, &resolver, data, None, &[], None)?;
    if !built.batch_errors.is_empty() {
        built.node.free();
        return Err(format!(
            "Decor WMO {fdid}: {}",
            built.batch_errors.join("; ")
        ));
    }
    Ok((built.node, bounds))
}

fn frame_catalog_bounds(scene: &mut Scene, bounds: Aabb) {
    let model = scene.model().expect("catalog pivot installed");
    scene.frame_bounds(model.get_transform() * bounds);
}

impl GameClient {
    fn selected_housing_preview(&self) -> Option<u32> {
        let model = self.housing.model.as_ref()?;
        if model.surface != HousingSurface::Dashboard || model.tab != DashboardTab::Catalog {
            return None;
        }
        Some(model.selected(&self.housing.authority)?.id)
    }

    /// Clear before registry sync removes the preview's parent, never afterward.
    pub(super) fn clear_inactive_housing_preview(&mut self) {
        if self.selected_housing_preview().is_none() {
            self.housing.catalog_preview.reset();
        }
    }

    pub(super) fn sync_housing_catalog_preview(&mut self) -> Result<(), String> {
        let Some(id) = self.selected_housing_preview() else {
            return Ok(());
        };
        let host = self
            .housing
            .ui
            .as_ref()
            .ok_or("Housing preview UI missing")?
            .bind()
            .frame_control(HOST)
            .ok_or("Housing catalog model scene missing")?;
        let catalog = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing catalog missing")?
            .catalog
            .clone();
        if let Err(error) =
            self.housing
                .catalog_preview
                .load_selection(host, &self.data_root, &catalog, id)
        {
            // Selection attempts are cached; log once, keep the source's unavailable UI.
            godot_warn!("Housing catalog preview: {error}");
        }
        Ok(())
    }

    pub(super) fn housing_catalog_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Some(rect) = self
            .housing
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_viewport_rect(HOST))
        else {
            return false;
        };
        let bounds = Rect2::new(
            Vector2::new(rect[0], rect[1]),
            Vector2::new(rect[2], rect[3]),
        );
        self.housing.catalog_preview.pointer(event, bounds)
    }
}

/// Engine-backed behavioral probe uses the same receipt loader and lifecycle as the HUD.
#[derive(GodotClass)]
#[class(base = RefCounted)]
struct WowHousingPreviewProbe {
    base: Base<RefCounted>,
    preview: CatalogPreview,
}

#[godot_api]
impl IRefCounted for WowHousingPreviewProbe {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            preview: CatalogPreview::default(),
        }
    }
}

#[godot_api]
impl WowHousingPreviewProbe {
    #[func]
    fn select_decor(&mut self, host: Gd<Control>, decor_id: i64) -> GString {
        let data = godot::classes::ProjectSettings::singleton().globalize_path("res://../data");
        let data = std::path::PathBuf::from(data.to_string());
        let result = HousingCatalog::load(&data.join("cache/housing.sqlite")).and_then(|catalog| {
            self.preview
                .load_selection(host, &data, &catalog, decor_id as u32)
        });
        let error = result.err().unwrap_or_default();
        GString::from(error.as_str())
    }

    #[func]
    fn rotate_preview(&mut self, yaw: f32) {
        self.preview.rotate(yaw);
    }

    #[func]
    fn clear_preview(&mut self) {
        self.preview.reset();
    }
}
