//! Semantic scene trees for `export-scene`, in the original snapshot contract
//! (`src/scenes/scene_snapshot_data.rs`). Each screen the original client described with a
//! `SceneTree` resource builds the same labels and props here from live native state:
//! InWorldScene (`src/scenes/inworld_tree.rs`), CharSelectScene
//! (`src/scenes/char_select/scene_tree.rs`), M2DebugScene (`src/scenes/m2_debug/mod.rs`),
//! DebugCharacterScene (`src/scenes/geoset_debug/mod.rs`) and SkyboxDebugScene
//! (`src/scenes/skybox_debug/mod.rs`). Values the original filled with constants (camera
//! FOV 60, sun lux, empty Head/MainHand slots, a fixed debug character) come from the
//! live nodes instead.

use game_engine_core::{
    character_model_data::race_name,
    scene_snapshot::{NodeProps, SceneNodeTransform, SceneSnapshot, SceneSnapshotNode},
};
use godot::{
    classes::{Camera3D, Light3D, Node, Node3D, light_3d::Param},
    obj::Inherits,
    prelude::*,
};
use shared::components::{EquipmentAppearance, EquipmentVisualSlot};

use crate::assets::M2_SOURCE_META;

/// One semantic node; `node` supplies its live transform.
pub(crate) struct SceneEntry {
    pub label: String,
    pub node: Option<Gd<Node3D>>,
    pub props: NodeProps,
    pub children: Vec<SceneEntry>,
}

impl SceneEntry {
    pub fn new(label: impl Into<String>, node: Option<Gd<Node3D>>, props: NodeProps) -> Self {
        Self {
            label: label.into(),
            node,
            props,
            children: Vec::new(),
        }
    }

    pub fn scene(label: &str, frame: Option<Gd<Node3D>>, children: Vec<SceneEntry>) -> Self {
        Self {
            children,
            ..Self::new(label, frame, NodeProps::Scene)
        }
    }

    pub fn with_children(mut self, children: Vec<SceneEntry>) -> Self {
        self.children = children;
        self
    }
}

/// The root keeps the original's `transform: null`; its node, when present, is the frame
/// its children's transforms are relative to (otherwise they are global).
pub(crate) fn snapshot(root: &SceneEntry) -> SceneSnapshot {
    SceneSnapshot {
        root: SceneSnapshotNode {
            label: root.label.clone(),
            transform: None,
            props: root.props.clone(),
            children: snapshot_children(root, root.node.as_ref()),
        },
    }
}

fn snapshot_children(entry: &SceneEntry, frame: Option<&Gd<Node3D>>) -> Vec<SceneSnapshotNode> {
    entry
        .children
        .iter()
        .map(|child| snapshot_node(child, frame))
        .collect()
}

/// A node with no backing Godot node exports `transform: null` and passes its frame on.
fn snapshot_node(entry: &SceneEntry, frame: Option<&Gd<Node3D>>) -> SceneSnapshotNode {
    SceneSnapshotNode {
        label: entry.label.clone(),
        transform: entry
            .node
            .as_ref()
            .map(|node| snapshot_transform(relative_transform(node, frame))),
        props: entry.props.clone(),
        children: snapshot_children(entry, entry.node.as_ref().or(frame)),
    }
}

fn relative_transform(node: &Gd<Node3D>, frame: Option<&Gd<Node3D>>) -> Transform3D {
    let global = node.get_global_transform();
    match frame {
        Some(frame) => frame.get_global_transform().affine_inverse() * global,
        None => global,
    }
}

pub(crate) fn snapshot_transform(transform: Transform3D) -> SceneNodeTransform {
    let translation = transform.origin;
    // godot-rust names Godot's get_rotation_quaternion() get_quaternion().
    let rotation = transform.basis.get_quaternion();
    let scale = transform.basis.get_scale();
    SceneNodeTransform {
        translation: [translation.x, translation.y, translation.z],
        rotation: [rotation.x, rotation.y, rotation.z, rotation.w],
        scale: [scale.x, scale.y, scale.z],
    }
}

pub(crate) fn camera_entry(camera: &Gd<Camera3D>) -> SceneEntry {
    SceneEntry::new(
        "Camera",
        Some(camera.clone().upcast()),
        NodeProps::Camera {
            fov: camera.get_fov(),
        },
    )
}

/// A light by its native class and energy, under the original's label.
pub(crate) fn light_entry<T: Inherits<Light3D>>(label: &str, light: &Gd<T>) -> SceneEntry {
    let light = light.clone().upcast::<Light3D>();
    SceneEntry::new(
        label,
        Some(light.clone().upcast()),
        NodeProps::Light {
            kind: light.get_class().to_string(),
            intensity: light.get_param(Param::ENERGY),
        },
    )
}

/// The exact model-loader input recorded on an M2 root.
pub(crate) fn m2_source(node: &Gd<Node3D>) -> Result<String, String> {
    let label = node.get_name();
    let source = node
        .get_meta(M2_SOURCE_META)
        .try_to::<GString>()
        .map_err(|_| format!("scene export: {label} has no M2 source path"))?
        .to_string();
    if source.is_empty() {
        return Err(format!("scene export: empty M2 source path on {label}"));
    }
    Ok(source)
}

pub(crate) fn child_3d(parent: &Gd<Node3D>, name: &str) -> Result<Gd<Node3D>, String> {
    parent
        .get_node_or_null(name)
        .and_then(|node| node.try_cast::<Node3D>().ok())
        .ok_or_else(|| format!("scene export: {} has no {name}", parent.get_name()))
}

/// `Character` with the model's file name, race name and sex as the original reports
/// them (`char_info_strings`), and one slot per equipped visual.
pub(crate) fn character_entry(
    label: &str,
    model: &Gd<Node3D>,
    race: u8,
    sex: u8,
    identity: (Option<String>, Option<u64>),
    equipment: &EquipmentAppearance,
) -> Result<SceneEntry, String> {
    let source = m2_source(model)?;
    let file = source.rsplit('/').next().unwrap_or(&source).to_owned();
    let gender = if sex == 0 { "Male" } else { "Female" };
    let (name, character_id) = identity;
    Ok(SceneEntry::new(
        label,
        Some(model.clone()),
        NodeProps::Character {
            model: file,
            race: race_name(race).into(),
            gender: gender.into(),
            name,
            character_id,
        },
    )
    .with_children(equipment_slot_entries(model, equipment)))
}

/// One `Slot:<slot>` per shown equipment entry; a shoulder is both shoulders
/// (`geoset_debug` slot definitions). Its model is the entry's display, `display:<id>`
/// (none for an entry given only by item); the anchor and attachment are the item
/// model's parent and name when a model was placed.
fn equipment_slot_entries(
    character: &Gd<Node3D>,
    equipment: &EquipmentAppearance,
) -> Vec<SceneEntry> {
    equipment
        .entries
        .iter()
        .filter(|entry| !entry.hidden)
        .flat_map(|entry| {
            equipment_slot_names(&entry.slot)
                .iter()
                .map(|name| slot_entry(character, name, entry.display_info_id))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn equipment_slot_names(slot: &EquipmentVisualSlot) -> &'static [&'static str] {
    match slot {
        EquipmentVisualSlot::Head => &["Head"],
        EquipmentVisualSlot::Shoulder => &["ShoulderLeft", "ShoulderRight"],
        EquipmentVisualSlot::Back => &["Back"],
        EquipmentVisualSlot::Chest => &["Chest"],
        EquipmentVisualSlot::Shirt => &["Shirt"],
        EquipmentVisualSlot::Tabard => &["Tabard"],
        EquipmentVisualSlot::Wrist => &["Wrist"],
        EquipmentVisualSlot::Hands => &["Hands"],
        EquipmentVisualSlot::Waist => &["Waist"],
        EquipmentVisualSlot::Legs => &["Legs"],
        EquipmentVisualSlot::Feet => &["Feet"],
        EquipmentVisualSlot::MainHand => &["MainHand"],
        EquipmentVisualSlot::OffHand => &["OffHand"],
        EquipmentVisualSlot::Ranged => &["Ranged"],
    }
}

/// The original reports the item's parent as both `anchor` and `attachment_anchor`.
fn slot_entry(character: &Gd<Node3D>, slot: &str, display: Option<u32>) -> SceneEntry {
    let item = character
        .find_child_ex(&format!("Equipment{slot}"))
        .owned(false)
        .done()
        .and_then(|node| node.try_cast::<Node3D>().ok());
    let anchor = item
        .as_ref()
        .and_then(|item| item.get_parent())
        .map(|parent: Gd<Node>| parent.get_name().to_string());
    SceneEntry::new(
        format!("Slot:{slot}"),
        item.clone(),
        NodeProps::EquipmentSlot {
            slot: slot.into(),
            model: display.map(|display| format!("display:{display}")),
            anchor: anchor.clone(),
            attachment: item.map(|item| item.get_name().to_string()),
            attachment_anchor: anchor,
        },
    )
}

/// The debug screens' Camera, Light and Ground (`m2_debug.rs` helpers).
pub(crate) fn debug_stage_entries(root: &Gd<Node3D>) -> Result<Vec<SceneEntry>, String> {
    let camera = child_3d(root, "Camera")?
        .try_cast::<Camera3D>()
        .map_err(|_| "scene export: debug Camera is not a Camera3D")?;
    let light = child_3d(root, "Light")?
        .try_cast::<Light3D>()
        .map_err(|_| "scene export: debug Light is not a Light3D")?;
    let ground = child_3d(root, "Ground")?;
    Ok(vec![
        camera_entry(&camera),
        light_entry("Light", &light),
        SceneEntry::new("Ground", Some(ground), NodeProps::Ground),
    ])
}

impl crate::GameClient {
    /// The current screen's semantic tree, or `None` on a screen the original described
    /// with no `SceneTree` (login, loading, character creation, menus).
    pub(crate) fn semantic_scene(&self) -> Result<Option<SceneEntry>, String> {
        let client = self.base();
        if let Some(scene) = client.get_node_or_null("M2Debug") {
            let scene = scene
                .try_cast::<crate::m2_debug::WowM2Debug>()
                .map_err(|_| "scene export: M2Debug has an unexpected class")?;
            return scene.bind().scene_entry().map(Some);
        }
        if let Some(scene) = client.get_node_or_null("DebugCharacter") {
            let scene = scene
                .try_cast::<crate::debug_character::WowDebugCharacter>()
                .map_err(|_| "scene export: DebugCharacter has an unexpected class")?;
            return scene.bind().scene_entry().map(Some);
        }
        if let Some(scene) = client.get_node_or_null("SkyboxDebug") {
            let scene = scene
                .try_cast::<crate::skybox_debug::WowSkyboxDebug>()
                .map_err(|_| "scene export: SkyboxDebug has an unexpected class")?;
            return scene.bind().scene_entry().map(Some);
        }
        match self.account.session.screen {
            game_engine_session::SessionScreen::InWorld => self.inworld_scene().map(Some),
            game_engine_session::SessionScreen::CharacterSelect => {
                self.character_preview.scene_entry()
            }
            _ => Ok(None),
        }
    }

    /// Players, NPCs, the world camera and the sun (`inworld_tree.rs`; the original had
    /// no light node in the world).
    fn inworld_scene(&self) -> Result<SceneEntry, String> {
        let mut children = Vec::new();
        let units = self.world.scene_units();
        for unit in units.iter().filter(|unit| unit.is_player) {
            children.push(self.player_entry(unit)?);
        }
        for unit in units.iter().filter(|unit| !unit.is_player) {
            if let Some(entry) = self.npc_entry(unit)? {
                children.push(entry);
            }
        }
        if let Some(camera) = self.world_camera.camera() {
            children.push(camera_entry(&camera));
        }
        if let Some(sun) = self.world_lighting.sun() {
            children.push(light_entry("EnvironmentSun", &sun));
        }
        Ok(SceneEntry::scene("InWorldScene", None, children))
    }

    fn player_entry(&self, unit: &crate::world::SceneUnit) -> Result<SceneEntry, String> {
        let visual = visual_model(unit.visual.as_ref())?;
        Ok(SceneEntry::new(
            "Player",
            Some(unit.node.clone()),
            NodeProps::Player {
                name: unit.name.clone(),
                is_local: unit.is_local,
                skin_path: visual
                    .as_ref()
                    .map(|(model, _)| primary_skin_path(model))
                    .transpose()?,
                display_scale: visual.as_ref().and_then(|(_, scale)| *scale),
                model_path: visual.map(|(model, _)| model),
            },
        ))
    }

    /// The original names an NPC by its template (`template_<id>`).
    fn npc_entry(&self, unit: &crate::world::SceneUnit) -> Result<Option<SceneEntry>, String> {
        let Some(snapshot) = self.replica.unit(unit.id) else {
            return Ok(None);
        };
        let Some(npc) = snapshot.get::<shared::components::Npc>() else {
            return Ok(None);
        };
        let visual = visual_model(unit.visual.as_ref())?;
        Ok(Some(SceneEntry::new(
            "Npc",
            Some(unit.node.clone()),
            NodeProps::Npc {
                name: format!("template_{}", npc.template_id),
                display_id: snapshot
                    .get::<shared::components::ModelDisplay>()
                    .map(|display| display.display_id),
                skin_path: visual
                    .as_ref()
                    .map(|(model, _)| primary_skin_path(model))
                    .transpose()?,
                display_scale: visual.as_ref().and_then(|(_, scale)| *scale),
                model_path: visual.map(|(model, _)| model),
            },
        )))
    }
}

/// A unit visual's M2 source and, for a creature display, its display scale: a player
/// body is the `PlayerModel` M2 itself; a creature is `NpcVisualRoot`, scaled by its
/// display, holding `NpcModel` (`world_models.rs`).
fn visual_model(visual: Option<&Gd<Node3D>>) -> Result<Option<(String, Option<f32>)>, String> {
    let Some(visual) = visual else {
        return Ok(None);
    };
    if visual.has_meta(M2_SOURCE_META) {
        return Ok(Some((m2_source(visual)?, None)));
    }
    let model = child_3d(visual, "NpcModel")?;
    Ok(Some((m2_source(&model)?, Some(visual.get_scale().x))))
}

/// The `{stem}00.skin` the loader reads with an M2 (`assets::read_model_file`).
fn primary_skin_path(model: &str) -> Result<String, String> {
    crate::assets::primary_skin_path(model)
}
