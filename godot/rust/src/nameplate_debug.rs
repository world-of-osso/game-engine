//! `--screen nameplatedebug`: the original offline nameplate preview
//! (`src/scenes/nameplate_debug.rs`, docs/specs/nameplate-debug.md) on the in-world
//! plate renderer: three stationary owners with names, health, a looping cast and a
//! looping channel; Space pauses and resumes, a click on a plate selects its owner.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use game_engine_core::spell_catalog::{
    SPELL_DB2_BUILD, SpellCatalogData, SpellCatalogPaths, load_spell_catalog,
};
use godot::{
    classes::{
        Camera3D, CanvasLayer, INode3D, InputEvent, InputEventKey, InputEventMouseButton, Label,
        ProjectSettings,
    },
    global::{Key, MouseButton},
    prelude::*,
};
use shared::casting::CastState;

use super::{
    BAR_Y_OFFSET, Health, Nameplates, PlateAura, PlateView, health_text, plate_alpha, reaction,
    reaction_color,
};
use crate::GameClient;

/// The original owners: x position, name, health percent, channel instead of cast.
const OWNERS: [(f32, &str, f32, bool); 3] = [
    (-3.8, "Zolramus Sorcerer", 76.3, false),
    (0.0, "Channeling Adept", 45.0, true),
    (3.8, "Training Guardian", 100.0, false),
];
/// Every owner's maximum health: the "425 K" of the user's nameplate reference.
const MAX_HEALTH: f32 = 425_000.0;
/// The original camera: (0, 2.5, 12) looking at (0, 2.5, 0).
const EYE: Vector3 = Vector3::new(0.0, 2.5, 12.0);
const LOOK: Vector3 = Vector3::new(0.0, 2.5, 0.0);

/// The original `demo_cast`: Necrotic Bolt (133, 5 s) or Arcane Missiles (5143, 6 s
/// channel ticking each second), both interruptible, 40% through.
pub(crate) fn demo_cast(channel: bool) -> CastState {
    let mut cast = if channel {
        CastState::channel(5143, 0, 6.0, 1.0, true)
    } else {
        CastState::normal(133, 0, 5.0, true)
    };
    cast.spell_name = if channel {
        "Arcane Missiles"
    } else {
        "Necrotic Bolt"
    }
    .into();
    cast.elapsed = cast.duration * 0.4;
    cast
}

/// The original `advance_preview_casts`: casts loop while playing.
pub(crate) fn advance_demo_cast(cast: &mut CastState, delta: f32, paused: bool) {
    if !paused {
        cast.elapsed = (cast.elapsed + delta) % cast.duration;
    }
}

/// The preview's nameplate auras: the Zolramus Sorcerer carries two of the player's
/// debuffs (its own spell's icon once the catalog has it), 12 s and 2 m left.
pub(crate) fn demo_auras(owner: &str, icon: Option<&u32>) -> Vec<PlateAura> {
    let (Some(&icon_fdid), "Zolramus Sorcerer") = (icon, owner) else {
        return Vec::new();
    };
    ["12 s", "2 m"]
        .map(|timer| PlateAura {
            icon_fdid,
            timer: timer.into(),
        })
        .into()
}

/// The original caption.
pub(crate) fn caption(paused: bool, selected: Option<&str>) -> String {
    let state = if paused { "Paused" } else { "Playing" };
    format!(
        "Nameplate preview — {state}\nSpace: pause/resume · Click a plate to select\nSelected: {}",
        selected.unwrap_or("none")
    )
}

struct Owner {
    id: u64,
    name: &'static str,
    health: f32,
    node: Gd<Node3D>,
    cast: CastState,
}

/// The nameplate preview: owners, plates, camera and caption.
#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowNameplateDebug {
    base: Base<Node3D>,
    data_root: PathBuf,
    style: game_engine_core::nameplate_style_data::NameplateStyle,
    plates: Nameplates,
    owners: Vec<Owner>,
    camera: Option<Gd<Camera3D>>,
    caption: Option<Gd<Label>>,
    paused: bool,
    selected: Option<u64>,
    catalog: Option<Receiver<Result<SpellCatalogData, String>>>,
    icons: std::collections::HashMap<u32, u32>,
}

#[godot_api]
impl INode3D for WowNameplateDebug {
    fn process(&mut self, delta: f64) {
        if let Err(error) = self.update(delta as f32) {
            godot_error!("Nameplate debug: {error}");
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        let handled = if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            let space = key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::SPACE;
            if space {
                self.paused = !self.paused;
            }
            space
        } else if let Ok(button) = event.try_cast::<InputEventMouseButton>() {
            button.is_pressed()
                && button.get_button_index() == MouseButton::LEFT
                && self.select_at(button.get_position())
        } else {
            false
        };
        if handled && let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }
}

#[godot_api]
impl WowNameplateDebug {
    /// Playback, selection and each owner's cast, for fixtures.
    #[func]
    fn debug_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("paused", self.paused);
        state.set("selected", self.selected_name().unwrap_or_default());
        let mut elapsed = VarDictionary::new();
        for owner in &self.owners {
            elapsed.set(owner.name, owner.cast.elapsed);
        }
        state.set("elapsed", &elapsed);
        state.set("plates", &self.plates_snapshot());
        if let Some(caption) = &self.caption {
            state.set("caption", &caption.get_text());
        }
        state
    }
}

impl WowNameplateDebug {
    fn update(&mut self, delta: f32) -> Result<(), String> {
        self.poll_catalog();
        let Some(camera) = self.camera.clone() else {
            return Ok(());
        };
        let views = self.views(&camera, delta);
        for owner in &self.owners {
            self.plates.casts.observe(owner.id, Some(&owner.cast));
        }
        // The preview clock stops while paused; so do its bars.
        let bar_delta = if self.paused { 0.0 } else { delta };
        self.plates.casts.advance(bar_delta, |_| true);
        let mut parent = self.to_gd().upcast::<Node3D>();
        let data_root = self.data_root.clone();
        let icons = self.icons.clone();
        let style = self.style;
        self.plates
            .sync_nodes(&mut parent, views, &style, true, (&data_root, &icons))?;
        self.sync_caption();
        Ok(())
    }

    fn views(
        &mut self,
        camera: &Gd<Camera3D>,
        delta: f32,
    ) -> std::collections::HashMap<u64, PlateView> {
        let color = reaction_color(&self.style, reaction(None, None));
        let paused = self.paused;
        let selected = self.selected;
        let icons = &self.icons;
        let show_value = self.style.show_health_value;
        let cvars = self.plates.cvars;
        self.owners
            .iter_mut()
            .map(|owner| {
                advance_demo_cast(&mut owner.cast, delta, paused);
                let top = owner.node.get_global_position() + Vector3::new(0.0, BAR_Y_OFFSET, 0.0);
                let view = PlateView {
                    name: owner.name.into(),
                    alpha: plate_alpha(&cvars, selected == Some(owner.id), 1.0, false),
                    occluded: false,
                    anchor: camera.unproject_position(top),
                    fraction: owner.health / 100.0,
                    health_text: health_text(
                        &Health {
                            current: MAX_HEALTH * owner.health / 100.0,
                            max: MAX_HEALTH,
                        },
                        show_value,
                        None,
                    ),
                    color,
                    name_color: Color::WHITE,
                    raid_target: None,
                    classification: None,
                    level: None,
                    targeted: selected == Some(owner.id),
                    enemy: false,
                    auras: demo_auras(owner.name, icons.get(&owner.cast.spell_id)),
                };
                (owner.id, view)
            })
            .collect()
    }

    /// The original plate-owner click: the plate under the pointer selects its owner.
    fn select_at(&mut self, at: Vector2) -> bool {
        let hit = self
            .plates
            .plate_rects()
            .find(|(_, rect)| rect.contains_point(at))
            .map(|(id, _)| id);
        if hit.is_some() {
            self.selected = hit;
        }
        hit.is_some()
    }

    fn selected_name(&self) -> Option<&'static str> {
        let selected = self.selected?;
        self.owners
            .iter()
            .find(|owner| owner.id == selected)
            .map(|owner| owner.name)
    }

    fn sync_caption(&mut self) {
        let text = caption(self.paused, self.selected_name());
        if let Some(label) = self.caption.as_mut()
            && label.get_text().to_string() != text
        {
            label.set_text(&text);
        }
    }

    fn plates_snapshot(&self) -> VarDictionary {
        let mut plates = VarDictionary::new();
        for owner in &self.owners {
            let mut entry = VarDictionary::new();
            if let Some(bar) = self.plates.casts.get(owner.id) {
                entry.set(
                    "cast",
                    &super::cast_snapshot(bar, self.plates.plates.get(&owner.id)),
                );
            }
            if let Some(plate) = self.plates.plates.get(&owner.id) {
                entry.set("frame_rect", plate.frame.get_global_rect());
                entry.set("name", &plate.name.get_text());
                entry.set("health", &plate.health.get_text());
            }
            plates.set(owner.name, &entry);
        }
        plates
    }

    /// The icons of the demo spells, once the spell catalog is ready.
    fn poll_catalog(&mut self) {
        let Some(receive) = &self.catalog else {
            return;
        };
        let catalog = match receive.try_recv() {
            Ok(Ok(catalog)) => catalog,
            Ok(Err(error)) => {
                godot_error!("Nameplate debug spell catalog: {error}");
                self.catalog = None;
                return;
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                godot_error!("Nameplate debug spell catalog loader stopped");
                self.catalog = None;
                return;
            }
        };
        self.catalog = None;
        let resolver = crate::assets::creature::local_resolver(&self.data_root);
        for owner in &self.owners {
            let spell = owner.cast.spell_id;
            let Some(fdid) = catalog.get(spell).map(|spell| spell.icon_fdid) else {
                godot_error!("Nameplate debug: spell {spell} is not in the catalog");
                continue;
            };
            let path = self.data_root.join("textures").join(format!("{fdid}.blp"));
            if path.exists() {
                self.icons.insert(spell, fdid);
                continue;
            }
            match resolver.ensure_cached(fdid, &path) {
                Ok(Some(_)) => {
                    self.icons.insert(spell, fdid);
                }
                Ok(None) => {
                    godot_error!(
                        "Nameplate debug: icon {fdid} of spell {spell} is not in local CASC"
                    );
                }
                Err(error) => {
                    godot_error!("{error}");
                }
            }
        }
    }

    fn attach(&mut self) {
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        // Bevy's default perspective, which the original camera keeps.
        camera.set_fov(45.0);
        self.base_mut().add_child(&camera);
        camera.look_at_from_position(EYE, LOOK);
        camera.make_current();
        self.camera = Some(camera);
        let environment =
            crate::m2_debug::environment_node(Color::from_rgb(0.094, 0.082, 0.078), Color::WHITE);
        self.base_mut().add_child(&environment);
        for (index, (x, name, health, channel)) in OWNERS.into_iter().enumerate() {
            let mut node = Node3D::new_alloc();
            node.set_name(name);
            node.set_position(Vector3::new(x, 0.0, 0.0));
            self.base_mut().add_child(&node);
            self.owners.push(Owner {
                id: index as u64 + 1,
                name,
                health,
                node,
                cast: demo_cast(channel),
            });
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_name("Instructions");
        layer.set_layer(2);
        let mut label = Label::new_alloc();
        label.set_name("Caption");
        label.set_position(Vector2::new(24.0, 24.0));
        label.add_theme_font_size_override("font_size", 16);
        label.add_theme_color_override("font_color", Color::WHITE);
        layer.add_child(&label);
        self.base_mut().add_child(&layer);
        self.caption = Some(label);
        self.sync_caption();
    }
}

fn spell_catalog_loader(data_root: &std::path::Path) -> Receiver<Result<SpellCatalogData, String>> {
    let mut paths = SpellCatalogPaths::for_data_dir(data_root);
    paths.cache_path = PathBuf::from(
        ProjectSettings::singleton()
            .globalize_path(&format!("user://spell_catalog-{SPELL_DB2_BUILD}.bin"))
            .to_string(),
    );
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(load_spell_catalog(&paths));
    });
    receive
}

impl GameClient {
    /// Shows the nameplate preview in place of the login screen.
    pub(crate) fn open_nameplate_debug(&mut self) -> Result<(), String> {
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        let data_root = self.data_root.clone();
        let style = self.client_options.hud.nameplate_style;
        let catalog = spell_catalog_loader(&data_root);
        let mut scene = Gd::from_init_fn(|base| WowNameplateDebug {
            base,
            data_root,
            style,
            plates: Nameplates::new(),
            owners: Vec::new(),
            camera: None,
            caption: None,
            paused: false,
            selected: None,
            catalog: Some(catalog),
            icons: std::collections::HashMap::new(),
        });
        scene.set_name("NameplateDebug");
        scene.bind_mut().attach();
        self.base_mut().add_child(&scene);
        Ok(())
    }
}

#[cfg(test)]
#[path = "nameplate_debug_tests.rs"]
mod tests;
