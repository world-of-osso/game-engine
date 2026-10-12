//! Retail HousingModelPreview: receipt-backed isolated model, never a placed instance.
//! Native framing/drag sensitivity are input tuning, not certified UiModelScene presets.
use crate::GameClient;
use crate::character_frame::preview::{Scene, bind_sheet_light};
use game_engine_core::housing::HousingCatalog;
use game_engine_ui_model::housing::{DashboardTab, HousingSurface};
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
        self.dragging = false;
    }

    fn select(
        &mut self,
        mut host: Gd<Control>,
        data: &Path,
        catalog: &HousingCatalog,
        decor_id: u32,
    ) -> Result<(), String> {
        if self.attempted == Some(decor_id) {
            if let Some(scene) = &mut self.scene {
                scene.fit_viewport();
            }
            return Ok(());
        }
        self.reset();
        self.attempted = Some(decor_id);
        match load_catalog_actor(data, catalog, decor_id) {
            Ok(visual) => {
                let mut pivot = Node3D::new_alloc();
                pivot.set_name("HousingCatalogPivot");
                pivot.add_child(&visual);
                let mut scene = Scene::new(host);
                scene.replace_model(pivot);
                self.scene = Some(scene);
                Ok(())
            }
            Err(error) => {
                let mut label = Label::new_alloc();
                label.set_name("HousingCatalogPreviewUnavailable");
                label.set_text("Preview unavailable");
                label.set_tooltip_text(&error);
                label.set_horizontal_alignment(godot::global::HorizontalAlignment::CENTER);
                label.set_vertical_alignment(godot::global::VerticalAlignment::CENTER);
                host.add_child(&label);
                label.set_anchors_and_offsets_preset(
                    godot::classes::control::LayoutPreset::FULL_RECT,
                );
                self.error_label = Some(label);
                Err(error)
            }
        }
    }

    fn rotate(&mut self, yaw: f32) {
        self.yaw = yaw;
        if let Some(scene) = &mut self.scene {
            scene.set_model_yaw(yaw);
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

fn load_catalog_actor(
    data: &Path,
    catalog: &HousingCatalog,
    decor_id: u32,
) -> Result<Gd<Node3D>, String> {
    let decor = catalog
        .decor
        .iter()
        .find(|decor| decor.id == decor_id)
        .ok_or_else(|| format!("Decor {decor_id} is not in the authenticated catalog."))?;
    let mut visual = crate::housing_decor::load_visual(data, catalog, decor_id)?;
    visual.set_name("HousingCatalogActor");
    visual.set_rotation(Vector3::from_array(decor.initial_rotation));
    visual.set_scale(Vector3::ONE * decor.initial_scale);
    bind_sheet_light(&visual);
    Ok(visual)
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
        if let Err(error) = self
            .housing
            .catalog_preview
            .select(host, &self.data_root, &catalog, id)
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
        let result = HousingCatalog::load(&data.join("cache/housing.sqlite"))
            .and_then(|catalog| self.preview.select(host, &data, &catalog, decor_id as u32));
        result.err().unwrap_or_default().into()
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
